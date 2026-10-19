//! Detecção de instalação, versão e capacidades do Console Mode (ADR-0025, CMI-002).
//!
//! Conecta-se estritamente em socket local (named pipe no Windows, socket Unix no Linux e macOS)
//! ou lê o arquivo de descoberta local (`discovery.json`). Timeout curto (≤ 500 ms), sem conexões
//! externas ou de rede não-loopback.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tako_core::console_mode::{ConsoleModeStatus, parse_discovery_content, perform_handshake};

/// Timeout máximo para conexão e aperto de mão com o Console Mode (500 ms).
pub const DETECTION_TIMEOUT: Duration = Duration::from_millis(500);

/// Caminhos padrão por SO para o Console Mode.
pub trait ConsoleModePaths {
    fn socket_path(&self) -> Option<PathBuf>;
    fn discovery_path(&self) -> Option<PathBuf>;
    fn app_paths(&self) -> Vec<PathBuf>;
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
                .unwrap_or_else(|_| "default".into());
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
            if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
                Some(PathBuf::from(runtime_dir).join("consolemode-api.sock"))
            } else {
                Some(PathBuf::from("/tmp/consolemode-api.sock"))
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    fn discovery_path(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let local_app_data = std::env::var_os("LOCALAPPDATA")?;
            Some(
                PathBuf::from(local_app_data)
                    .join("ConsoleMode")
                    .join("discovery.json"),
            )
        }
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var_os("HOME")?;
            Some(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("ConsoleMode")
                    .join("discovery.json"),
            )
        }
        #[cfg(target_os = "linux")]
        {
            let base = if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
                PathBuf::from(config_home)
            } else if let Some(home) = std::env::var_os("HOME") {
                PathBuf::from(home).join(".config")
            } else {
                return None;
            };
            Some(base.join("consolemode").join("discovery.json"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    fn app_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        #[cfg(target_os = "windows")]
        {
            if let Some(lad) = std::env::var_os("LOCALAPPDATA") {
                paths.push(
                    PathBuf::from(lad)
                        .join("ConsoleMode")
                        .join("ConsoleMode.exe"),
                );
            }
            if let Some(pf) = std::env::var_os("ProgramFiles") {
                paths.push(
                    PathBuf::from(pf)
                        .join("ConsoleMode")
                        .join("ConsoleMode.exe"),
                );
            }
        }
        #[cfg(target_os = "macos")]
        {
            paths.push(PathBuf::from("/Applications/ConsoleMode.app"));
            if let Some(home) = std::env::var_os("HOME") {
                let h = PathBuf::from(home);
                paths.push(h.join("Applications").join("ConsoleMode.app"));
                paths.push(
                    h.join("Library")
                        .join("Application Support")
                        .join("ConsoleMode"),
                );
            }
        }
        #[cfg(target_os = "linux")]
        {
            paths.push(PathBuf::from("/usr/bin/consolemode"));
            paths.push(PathBuf::from("/usr/local/bin/consolemode"));
            if let Some(home) = std::env::var_os("HOME") {
                let h = PathBuf::from(home);
                paths.push(h.join(".local").join("bin").join("consolemode"));
                paths.push(h.join(".config").join("consolemode"));
            }
        }
        paths
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
    let file = fs::OpenOptions::new().read(true).write(true).open(path)?;
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

/// Detecção nativa do Console Mode no ambiente atual.
pub fn detect() -> ConsoleModeStatus {
    detect_internal(&PlatformPaths, |p| connect_local_socket(p))
}

/// Lógica de detecção desacoplada para testes com fixtures e fakes.
pub fn detect_internal<P: ConsoleModePaths, C>(paths: &P, connector: C) -> ConsoleModeStatus
where
    C: Fn(&Path) -> io::Result<Box<dyn ReadWrite>>,
{
    // 1. Tenta conectar no socket ou named pipe
    if let Some(socket_path) = paths.socket_path() {
        match connector(&socket_path) {
            Ok(mut stream) => {
                let status = perform_handshake(&mut stream);
                return status;
            }
            Err(e) => {
                // Caso não encontre o socket ou a conexão seja recusada, continua para arquivos locais
                if e.kind() != io::ErrorKind::NotFound
                    && e.kind() != io::ErrorKind::ConnectionRefused
                {
                    tracing::debug!(
                        path = %socket_path.display(),
                        "não foi possível conectar ao socket do Console Mode: {e}"
                    );
                }
            }
        }
    }

    // 2. Tenta ler o arquivo de descoberta local (ex.: se o processo deixou metadados de API)
    if let Some(discovery_path) = paths.discovery_path() {
        if discovery_path.is_file() {
            match fs::read(&discovery_path) {
                Ok(bytes) => match parse_discovery_content(&bytes) {
                    Ok(status) => return status,
                    Err(e) => {
                        tracing::warn!(
                            path = %discovery_path.display(),
                            "arquivo de descoberta do Console Mode inválido: {e}"
                        );
                        return ConsoleModeStatus::Erro;
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        path = %discovery_path.display(),
                        "erro ao ler arquivo de descoberta do Console Mode: {e}"
                    );
                    return ConsoleModeStatus::Erro;
                }
            }
        }
    }

    // 3. Verifica se há indícios de instalação
    let installed_on_disk = paths.app_paths().iter().any(|p| p.exists());
    if installed_on_disk {
        // Conforme ADR-0025: sem resposta no socket e sem arquivo de descoberta válido,
        // o Console Mode não está ativo ou não expõe API. Retorna NaoInstalado.
        ConsoleModeStatus::NaoInstalado
    } else {
        ConsoleModeStatus::NaoInstalado
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tako_core::console_mode::CURRENT_API_VERSION;
    use tempfile::tempdir;

    struct TestPaths {
        socket: Option<PathBuf>,
        discovery: Option<PathBuf>,
        apps: Vec<PathBuf>,
    }

    impl ConsoleModePaths for TestPaths {
        fn socket_path(&self) -> Option<PathBuf> {
            self.socket.clone()
        }
        fn discovery_path(&self) -> Option<PathBuf> {
            self.discovery.clone()
        }
        fn app_paths(&self) -> Vec<PathBuf> {
            self.apps.clone()
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
            discovery: Some(PathBuf::from("/nonexistent/discovery.json")),
            apps: vec![PathBuf::from("/nonexistent/bin")],
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
    fn detect_returns_instalado_from_socket() {
        let paths = TestPaths {
            socket: Some(PathBuf::from("/tmp/fake-consolemode.sock")),
            discovery: None,
            apps: vec![],
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
            discovery: None,
            apps: vec![],
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
            discovery: None,
            apps: vec![],
        };
        let status = detect_internal(&paths, |_| Ok(Box::new(TimeoutStream)));
        assert_eq!(status, ConsoleModeStatus::Erro);
    }

    #[test]
    fn detect_returns_instalado_from_discovery_file() {
        let dir = tempdir().unwrap();
        let disc = dir.path().join("discovery.json");
        fs::write(
            &disc,
            br#"{"api":1,"version":"1.1.2","capabilities":["windows","fps"]}"#,
        )
        .unwrap();

        let paths = TestPaths {
            socket: None,
            discovery: Some(disc),
            apps: vec![],
        };
        let status = detect_internal(&paths, |_| {
            Err(io::Error::new(io::ErrorKind::NotFound, "sem socket"))
        });
        assert_eq!(
            status,
            ConsoleModeStatus::Instalado {
                versao_api: 1,
                capacidades: vec!["windows".into(), "fps".into()],
            }
        );
    }

    #[test]
    fn detect_returns_incompativel_from_discovery_file() {
        let dir = tempdir().unwrap();
        let disc = dir.path().join("discovery.json");
        fs::write(
            &disc,
            br#"{"api":2,"version":"2.0.0-beta","capabilities":[]}"#,
        )
        .unwrap();

        let paths = TestPaths {
            socket: None,
            discovery: Some(disc),
            apps: vec![],
        };
        let status = detect_internal(&paths, |_| {
            Err(io::Error::new(io::ErrorKind::NotFound, "sem socket"))
        });
        assert_eq!(
            status,
            ConsoleModeStatus::Incompativel {
                versao: "2.0.0-beta".into(),
            }
        );
    }

    #[test]
    fn detect_returns_erro_from_malformed_discovery_file() {
        let dir = tempdir().unwrap();
        let disc = dir.path().join("discovery.json");
        fs::write(&disc, b"conteudo invalido nao json").unwrap();

        let paths = TestPaths {
            socket: None,
            discovery: Some(disc),
            apps: vec![],
        };
        let status = detect_internal(&paths, |_| {
            Err(io::Error::new(io::ErrorKind::NotFound, "sem socket"))
        });
        assert_eq!(status, ConsoleModeStatus::Erro);
    }
}
