//! Detecção de instalação, versão e capacidades do Console Mode (ADR-0025, CMI-002).
//!
//! Conecta-se estritamente em socket local (named pipe no Windows, socket Unix no Linux e macOS).
//! Timeout curto (≤ 500 ms), sem conexões externas ou de rede não-loopback.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tako_core::console_mode::{ConsoleModeStatus, perform_handshake};

/// Timeout máximo para conexão e aperto de mão com o Console Mode (500 ms).
pub const DETECTION_TIMEOUT: Duration = Duration::from_millis(500);

/// Caminhos padrão por SO para o Console Mode.
pub trait ConsoleModePaths {
    fn socket_path(&self) -> Option<PathBuf>;
}

/// Implementação dos caminhos reais do sistema operacional atual.
#[derive(Debug, Default, Clone)]
pub struct PlatformPaths;

impl ConsoleModePaths for PlatformPaths {
    fn socket_path(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let user = std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .ok()?;
            Some(PathBuf::from(format!(r"\\.\pipe\consolemode-api-{user}")))
        }
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var_os("HOME")?;
            Some(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("ConsoleMode")
                    .join("api.sock"),
            )
        }
        #[cfg(target_os = "linux")]
        {
            let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")?;
            Some(PathBuf::from(runtime_dir).join("consolemode-api.sock"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

/// Conecta ao socket local Unix ou named pipe com timeout curto.
#[cfg(unix)]
fn connect_local_socket(path: &Path) -> io::Result<Box<dyn ReadWrite>> {
    use std::os::unix::net::UnixStream;
    let stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(DETECTION_TIMEOUT))?;
    stream.set_write_timeout(Some(DETECTION_TIMEOUT))?;
    Ok(Box::new(stream))
}

#[cfg(windows)]
fn connect_local_socket(path: &Path) -> io::Result<Box<dyn ReadWrite>> {
    let file = std::fs::OpenOptions::new().read(true).write(true).open(path)?;
    Ok(Box::new(file))
}

#[cfg(not(any(unix, windows)))]
fn connect_local_socket(_path: &Path) -> io::Result<Box<dyn ReadWrite>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "plataforma sem suporte a socket do Console Mode",
    ))
}

/// Trait utilitário para permitir leitura e escrita em caixas de fluxos.
pub trait ReadWrite: Read + Write {}
impl<T: Read + Write + ?Sized> ReadWrite for T {}

/// Detecção nativa do Console Mode no ambiente atual com timeout garantido por thread.
pub fn detect() -> ConsoleModeStatus {
    detect_with(&PlatformPaths, connect_local_socket, DETECTION_TIMEOUT)
}

/// Detecção com caminhos, conector e timeout desacoplados para testes.
pub fn detect_with<P, C>(paths: &P, connector: C, timeout: Duration) -> ConsoleModeStatus
where
    P: ConsoleModePaths + Clone + Send + 'static,
    C: Fn(&Path) -> io::Result<Box<dyn ReadWrite>> + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let paths_clone = paths.clone();
    let handle = std::thread::Builder::new()
        .name("console-mode-detect".into())
        .spawn(move || {
            let res = detect_internal(&paths_clone, connector);
            let _ = tx.send(res);
        });

    match handle {
        Ok(_) => match rx.recv_timeout(timeout) {
            Ok(status) => status,
            Err(_) => {
                tracing::warn!("tempo limite esgotado ao detectar o Console Mode");
                ConsoleModeStatus::Erro
            }
        },
        Err(e) => {
            tracing::warn!("falha ao criar thread para detecção do Console Mode: {e}");
            ConsoleModeStatus::Erro
        }
    }
}

/// Lógica de detecção síncrona interna.
pub fn detect_internal<P: ConsoleModePaths, C>(paths: &P, connector: C) -> ConsoleModeStatus
where
    C: Fn(&Path) -> io::Result<Box<dyn ReadWrite>>,
{
    let Some(socket_path) = paths.socket_path() else {
        return ConsoleModeStatus::NaoInstalado;
    };

    match connector(&socket_path) {
        Ok(mut stream) => perform_handshake(&mut stream),
        Err(e) => {
            tracing::debug!(
                path = %socket_path.display(),
                "não foi possível conectar ao socket do Console Mode: {e}"
            );
            ConsoleModeStatus::NaoInstalado
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tako_core::console_mode::CURRENT_API_VERSION;

    #[derive(Clone)]
    struct TestPaths {
        socket: Option<PathBuf>,
    }

    impl ConsoleModePaths for TestPaths {
        fn socket_path(&self) -> Option<PathBuf> {
            self.socket.clone()
        }
    }

    struct MockSocketStream {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl MockSocketStream {
        fn new(response_payload: &[u8]) -> Self {
            let len = u32::try_from(response_payload.len()).unwrap();
            let mut data = Vec::with_capacity(4 + response_payload.len());
            data.extend_from_slice(&len.to_le_bytes());
            data.extend_from_slice(response_payload);
            Self {
                input: Cursor::new(data),
                output: Vec::new(),
            }
        }
    }

    impl Read for MockSocketStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl Write for MockSocketStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct TimeoutStream;
    impl Read for TimeoutStream {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "tempo limite esgotado",
            ))
        }
    }
    impl Write for TimeoutStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn detect_returns_nao_instalado_when_empty() {
        let paths = TestPaths {
            socket: Some(PathBuf::from("/nonexistent/api.sock")),
        };
        let status = detect_internal(&paths, |_| {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "socket inexistente",
            ))
        });
        assert_eq!(status, ConsoleModeStatus::NaoInstalado);
    }

    #[test]
    fn detect_returns_nao_instalado_when_no_socket_path() {
        let paths = TestPaths { socket: None };
        let status = detect_internal(&paths, |_| unreachable!());
        assert_eq!(status, ConsoleModeStatus::NaoInstalado);
    }

    #[test]
    fn detect_returns_instalado_from_socket() {
        let paths = TestPaths {
            socket: Some(PathBuf::from("/tmp/fake-consolemode.sock")),
        };
        let json = br#"{"type":"hello","server":"consolemode","api":1,"version":"1.0.0","capabilities":["tv","fps"]}"#;
        let status = detect_internal(&paths, |_| Ok(Box::new(MockSocketStream::new(json))));
        assert_eq!(
            status,
            ConsoleModeStatus::Instalado {
                versao_api: CURRENT_API_VERSION,
                capacidades: vec!["tv".into(), "fps".into()],
            }
        );
    }

    #[test]
    fn detect_returns_incompativel_from_socket() {
        let paths = TestPaths {
            socket: Some(PathBuf::from("/tmp/fake-consolemode.sock")),
        };
        let json = br#"{"type":"hello","server":"consolemode","api":99,"version":"99.0.0","capabilities":[]}"#;
        let status = detect_internal(&paths, |_| Ok(Box::new(MockSocketStream::new(json))));
        assert_eq!(
            status,
            ConsoleModeStatus::Incompativel {
                versao: "99.0.0".into(),
            }
        );
    }

    #[test]
    fn detect_returns_erro_on_timeout() {
        let paths = TestPaths {
            socket: Some(PathBuf::from("/tmp/fake-consolemode.sock")),
        };
        let status = detect_internal(&paths, |_| Ok(Box::new(TimeoutStream)));
        assert_eq!(status, ConsoleModeStatus::Erro);
    }

    #[cfg(unix)]
    #[test]
    fn detect_with_fake_server_that_accepts_and_does_not_respond_times_out() {
        use std::os::unix::net::UnixListener;
        let dir = tempfile::tempdir().unwrap();
        let sock_path = dir.path().join("fake_hang.sock");
        let listener = UnixListener::bind(&sock_path).unwrap();

        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                std::thread::sleep(Duration::from_millis(400));
                let _ = stream.write_all(b"tardio");
            }
        });

        let paths = TestPaths {
            socket: Some(sock_path),
        };
        let status = detect_with(&paths, connect_local_socket, Duration::from_millis(50));
        assert_eq!(status, ConsoleModeStatus::Erro);

        let _ = handle.join();
    }
}
