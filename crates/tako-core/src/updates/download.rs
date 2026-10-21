//! Rede da atualização (port de `UpdateService.cs`): procurar, destaque do changelog, baixar e conferir.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::SemVer;
use super::feed::{self, Arch, Package, RELEASES_URL, REPOSITORY, UpdateInfo};
use crate::i18n::{Lang, t};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Teto da consulta, compartilhado com o relógio da UI.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(60);
/// Sem tempo total: o limite é por falta de dados, para não cortar downloads lentos que ainda progridem.
pub const STALL_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_FEED_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CHANGELOG_BYTES: u64 = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("{}", t("updates.error.not_installable"))]
    NotInstallable,
    #[error("{}", t("updates.error.digest_mismatch"))]
    DigestMismatch,
    #[error("{}", t("updates.error.stalled"))]
    Stalled,
    #[error("{0}")]
    Http(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl From<ureq::Error> for UpdateError {
    fn from(e: ureq::Error) -> Self {
        match e {
            ureq::Error::Io(io) => UpdateError::Io(io),
            other => UpdateError::Http(other.to_string()),
        }
    }
}

/// Agente HTTP com o User-Agent que a API do GitHub exige.
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(format!(
            "Tako/{} (+https://github.com/{REPOSITORY})",
            SemVer::current()
        ))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        // A API do GitHub chega a levar mais de 10 s para começar a responder, mesmo em rede rápida.
        .timeout_recv_response(Some(STALL_TIMEOUT))
        .build()
        .into()
}

/// Procura atualização nas releases do GitHub.
pub fn check(
    agent: &ureq::Agent,
    current: &SemVer,
    channel: feed::Channel,
    package: Package,
    arch: Arch,
) -> Result<Option<UpdateInfo>, UpdateError> {
    let json = fetch_text(
        agent,
        RELEASES_URL,
        MAX_FEED_BYTES,
        CHECK_TIMEOUT,
        STALL_TIMEOUT,
    )?;
    Ok(feed::pick(&json, current, channel, package, arch))
}

/// Primeiro item da versão no changelog do idioma; sem ele, `None` (o aviso usa as notas da release).
pub fn fetch_highlight(agent: &ureq::Agent, update: &UpdateInfo, lang: Lang) -> Option<String> {
    let text = fetch_text(
        agent,
        &feed::changelog_url(&update.version, lang),
        MAX_CHANGELOG_BYTES,
        CHECK_TIMEOUT,
        STALL_TIMEOUT,
    )
    .ok()?;
    feed::first_note(&feed::changelog_section(&text, &update.version)?)
}

/// Cada leitura reinicia a espera por dados; o teto global também encerra a thread de rede.
fn fetch_text(
    agent: &ureq::Agent,
    url: &str,
    limit: u64,
    total_timeout: Duration,
    stall_timeout: Duration,
) -> Result<String, UpdateError> {
    let start = Instant::now();
    let (tx, rx) = mpsc::channel();
    let (agent, url) = (agent.clone(), url.to_owned());
    std::thread::spawn(move || {
        let run = || -> Result<Option<String>, UpdateError> {
            let mut response = agent
                .get(&url)
                .header("Accept", "application/vnd.github+json")
                .config()
                .timeout_global(Some(total_timeout))
                .build()
                .call()?;
            let mut reader = response.body_mut().with_config().limit(limit).reader();
            let mut bytes = Vec::new();
            let mut buf = [0; 8192];
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buf[..n]);
                if tx.send(Ok(None)).is_err() {
                    return Ok(None);
                }
            }
            String::from_utf8(bytes)
                .map(Some)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e).into())
        };
        let _ = tx.send(run());
    });
    loop {
        let remaining = total_timeout.saturating_sub(start.elapsed());
        match rx.recv_timeout(stall_timeout.min(remaining)) {
            Ok(Ok(None)) => {}
            Ok(Ok(Some(text))) => return Ok(text),
            Ok(Err(e)) => return Err(e),
            Err(mpsc::RecvTimeoutError::Timeout) if start.elapsed() >= total_timeout => {
                return Err(UpdateError::Http("timeout: global".into()));
            }
            Err(_) => return Err(UpdateError::Stalled),
        }
    }
}

/// Pasta onde o arquivo é baixado (`<temp>/TakoUpdate`, como no C#).
pub fn download_dir() -> PathBuf {
    std::env::temp_dir().join("TakoUpdate")
}

/// Baixa para `dir` e confere o SHA-256; reaproveita um arquivo já conferido, apaga o que não confere.
/// `progress` recebe a fração (0..=1) quando o servidor informa o tamanho.
pub fn download(
    agent: &ureq::Agent,
    update: &UpdateInfo,
    dir: &Path,
    progress: &mut dyn FnMut(f64),
) -> Result<PathBuf, UpdateError> {
    let asset = update.asset.as_ref().ok_or(UpdateError::NotInstallable)?;
    // O nome vem da API: só o último componente, sem separadores.
    let file_name = Path::new(&asset.name)
        .file_name()
        .filter(|n| n.to_str() == Some(asset.name.as_str()))
        .ok_or(UpdateError::NotInstallable)?;
    fs::create_dir_all(dir)?;
    let file = dir.join(file_name);
    if file.exists() && verify_file(&file, &asset.digest)? {
        return Ok(file); // baixado numa abertura anterior
    }
    // Arquivo parcial único: um download parado que ainda escreve não atrapalha o próximo.
    let part = dir.join(format!(
        "{}.part-{}",
        asset.name,
        uuid::Uuid::new_v4().simple()
    ));
    let result = fetch_with_stall_timeout(agent, &asset.url, &part, progress);
    if let Err(e) = result {
        let _ = fs::remove_file(&part);
        return Err(e);
    }
    if !verify_file(&part, &asset.digest)? {
        fs::remove_file(&part)?;
        return Err(UpdateError::DigestMismatch);
    }
    fs::rename(&part, &file)?;
    tracing::info!(version = %update.version, file = %file.display(), "atualização baixada e verificada");
    Ok(file)
}

enum Chunk {
    Progress(f64),
    Done(Result<(), UpdateError>),
}

/// O `ureq` não tem tempo por leitura: o download roda numa thread e cada pedaço reinicia o relógio.
fn fetch_with_stall_timeout(
    agent: &ureq::Agent,
    url: &str,
    target: &Path,
    progress: &mut dyn FnMut(f64),
) -> Result<(), UpdateError> {
    let (tx, rx) = mpsc::channel();
    let abandoned = Arc::new(AtomicBool::new(false));
    let (agent, url, target, stop) = (
        agent.clone(),
        url.to_owned(),
        target.to_path_buf(),
        abandoned.clone(),
    );
    std::thread::spawn(move || {
        let run = || -> Result<(), UpdateError> {
            let mut response = agent
                .get(&url)
                .config()
                .timeout_recv_response(Some(STALL_TIMEOUT))
                .build()
                .call()?;
            let total = response.body().content_length().unwrap_or(0);
            let mut source = response.body_mut().with_config().limit(u64::MAX).reader();
            let mut out = File::create(&target)?;
            let mut buf = vec![0u8; 81920];
            let mut done = 0u64;
            loop {
                let n = source.read(&mut buf)?;
                if n == 0 || stop.load(Ordering::Relaxed) {
                    break;
                }
                out.write_all(&buf[..n])?;
                done += n as u64;
                let frac = if total > 0 {
                    done as f64 / total as f64
                } else {
                    -1.0
                };
                let _ = tx.send(Chunk::Progress(frac));
            }
            out.sync_all()?;
            Ok(())
        };
        let _ = tx.send(Chunk::Done(run()));
    });
    loop {
        match rx.recv_timeout(STALL_TIMEOUT) {
            Ok(Chunk::Progress(f)) if f >= 0.0 => progress(f.min(1.0)),
            Ok(Chunk::Progress(_)) => {}
            Ok(Chunk::Done(r)) => return r,
            Err(_) => {
                abandoned.store(true, Ordering::Relaxed);
                return Err(UpdateError::Stalled);
            }
        }
    }
}

/// Confere o arquivo com o SHA-256 publicado (comparação em tempo constante).
pub fn verify_file(path: &Path, expected_digest: &str) -> io::Result<bool> {
    if !feed::is_valid_sha256_digest(expected_digest) {
        return Ok(false);
    }
    let Some(expected) = decode_hex(&expected_digest[7..]) else {
        return Ok(false);
    };
    let mut hasher = Sha256::new();
    io::copy(&mut File::open(path)?, &mut hasher)?;
    let actual = hasher.finalize();
    let diff = actual
        .iter()
        .zip(&expected)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b));
    Ok(diff == 0 && expected.len() == actual.len())
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    s.as_bytes()
        .chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slow_server(
        chunks: Vec<Vec<u8>>,
        delay: Duration,
        gzip: bool,
    ) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let length: usize = chunks.iter().map(Vec::len).sum();
            let encoding = if gzip {
                "Content-Encoding: gzip\r\n"
            } else {
                ""
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n{encoding}Connection: close\r\n\r\n"
            )
            .unwrap();
            for chunk in chunks {
                if stream.write_all(&chunk).is_err() {
                    break;
                }
                stream.flush().unwrap();
                std::thread::sleep(delay);
            }
            String::from_utf8(request).unwrap()
        });
        (url, thread)
    }

    #[test]
    fn check_body_can_progress_beyond_the_old_global_budget() {
        let (url, server) = slow_server(
            vec![b" ".to_vec(); 8]
                .into_iter()
                .chain([br#"[{"tag_name":"v0.2.0-alpha.19","prerelease":true}]"#.to_vec()])
                .collect(),
            Duration::from_millis(40),
            false,
        );
        let result = fetch_text(
            &agent(),
            &url,
            MAX_FEED_BYTES,
            Duration::from_secs(2),
            Duration::from_millis(200),
        );
        server.join().unwrap();
        let update = feed::pick(
            &result.unwrap(),
            &SemVer::parse("0.2.0-alpha.16"),
            feed::Channel::Testing,
            Package::WindowsSetup,
            Arch::X64,
        )
        .unwrap();
        assert_eq!(update.version, "0.2.0-alpha.19");
    }

    #[test]
    fn check_body_stops_when_data_stalls() {
        let (url, server) = slow_server(
            vec![b"[".to_vec(), b"]".to_vec()],
            Duration::from_millis(300),
            false,
        );
        let result = fetch_text(
            &agent(),
            &url,
            MAX_FEED_BYTES,
            Duration::from_secs(2),
            Duration::from_millis(100),
        );
        server.join().unwrap();
        assert!(matches!(result, Err(UpdateError::Stalled)), "{result:?}");
    }

    #[test]
    fn check_body_has_a_total_ceiling_even_with_progress() {
        let (url, server) = slow_server(vec![b" ".to_vec(); 20], Duration::from_millis(40), false);
        let result = fetch_text(
            &agent(),
            &url,
            MAX_FEED_BYTES,
            Duration::from_millis(150),
            Duration::from_millis(300),
        );
        server.join().unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn check_requests_and_decodes_gzip() {
        // gzip de `[]`, para verificar negociação e descompressão sem outra dependência.
        let gzip = vec![
            31, 139, 8, 0, 0, 0, 0, 0, 2, 3, 139, 142, 5, 0, 41, 187, 76, 13, 2, 0, 0, 0,
        ];
        let (url, server) = slow_server(vec![gzip], Duration::ZERO, true);
        let result = fetch_text(
            &agent(),
            &url,
            MAX_FEED_BYTES,
            Duration::from_secs(2),
            Duration::from_millis(200),
        );
        let request = server.join().unwrap().to_ascii_lowercase();
        assert!(
            request
                .lines()
                .any(|line| line.starts_with("accept-encoding:") && line.contains("gzip")),
            "{request}"
        );
        assert_eq!(result.unwrap(), "[]");
    }

    #[test]
    fn check_body_preserves_the_size_limit() {
        let (url, server) = slow_server(vec![b"[]".to_vec()], Duration::ZERO, false);
        let result = fetch_text(
            &agent(),
            &url,
            1,
            Duration::from_secs(2),
            Duration::from_millis(200),
        );
        server.join().unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn verifies_downloaded_file_against_digest() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        fs::write(&file, [1u8, 2, 3]).unwrap();
        let hex: String = Sha256::digest([1u8, 2, 3])
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(verify_file(&file, &format!("sha256:{hex}")).unwrap());
        assert!(verify_file(&file, &format!("SHA256:{}", hex.to_uppercase())).unwrap());
        assert!(
            !verify_file(
                &file,
                "sha256:1861bbb28004cc7811dc37015e74acee00f5cf59242dcb8e64d995754ba2dea7"
            )
            .unwrap()
        );
        assert!(!verify_file(&file, "md5:abc").unwrap());
    }

    #[test]
    fn download_reuses_a_verified_file_and_refuses_unsafe_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Tako-Portable-x64.exe"), [1u8, 2, 3]).unwrap();
        let hex: String = Sha256::digest([1u8, 2, 3])
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let mut update = UpdateInfo {
            version: "0.2.0".into(),
            name: "Tako 0.2.0".into(),
            notes: String::new(),
            page_url: String::new(),
            // URL inválida: se tentasse baixar, falharia.
            asset: Some(feed::Asset {
                size: 0,
                url: "http://invalid.invalid/x".into(),
                name: "Tako-Portable-x64.exe".into(),
                digest: format!("sha256:{hex}"),
            }),
            is_prerelease: false,
        };
        let got = download(&agent(), &update, dir.path(), &mut |_| {}).unwrap();
        assert_eq!(got, dir.path().join("Tako-Portable-x64.exe"));

        update.asset.as_mut().unwrap().name = "../evil.exe".into();
        assert!(matches!(
            download(&agent(), &update, dir.path(), &mut |_| {}),
            Err(UpdateError::NotInstallable)
        ));
        update.asset = None;
        assert!(matches!(
            download(&agent(), &update, dir.path(), &mut |_| {}),
            Err(UpdateError::NotInstallable)
        ));
    }
}
