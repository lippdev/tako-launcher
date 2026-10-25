//! Onde está a interface: pacote `web/dist` ou servidor de desenvolvimento. Sem CEF, para o núcleo conferir
//! antes de abrir o processo da interface, que resolve de novo do mesmo jeito.

use std::path::PathBuf;

pub const APP_URL: &str = "tako-app://app/";
pub const APP_ORIGIN: &str = "tako-app://app";

pub struct Target {
    pub url: String,
    pub origin: String,
    pub root: PathBuf,
}

pub fn resolve() -> Result<Target, String> {
    let root = if cfg!(debug_assertions) {
        std::env::var_os("TAKO_WEB_TEST_ROOT").map(PathBuf::from)
    } else {
        None
    }
    .unwrap_or_else(|| {
        if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist")
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("web/dist")))
                .unwrap_or_default()
        }
    });
    if cfg!(debug_assertions)
        && let Ok(url) = std::env::var("TAKO_WEB_DEV_URL")
    {
        let origin = dev_origin(&url)
            .ok_or("TAKO_WEB_DEV_URL deve usar http://localhost:<porta>/ ou loopback")?;
        return Ok(Target { url, origin, root });
    }
    if !root.join("index.html").is_file() {
        return Err(format!(
            "pacote web ausente: {}; gere web/dist ou defina TAKO_WEB_TEST_ROOT no debug",
            root.display()
        ));
    }
    Ok(Target {
        url: APP_URL.into(),
        origin: APP_ORIGIN.into(),
        root,
    })
}

/// Uma origem exata, incluindo porta. Só localhost é aceito para desenvolvimento.
pub fn dev_origin(url: &str) -> Option<String> {
    let normalized = url.to_ascii_lowercase();
    let rest = normalized.strip_prefix("http://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let (host, port) = if let Some(rest) = authority.strip_prefix("[::1]") {
        (
            "[::1]",
            if rest.is_empty() {
                "80"
            } else {
                rest.strip_prefix(':')?
            },
        )
    } else {
        authority.rsplit_once(':').unwrap_or((authority, "80"))
    };
    let port = port.parse::<u16>().ok()?;
    if !matches!(host, "localhost" | "127.0.0.1" | "[::1]") || port == 0 {
        return None;
    }
    Some(if port == 80 {
        format!("http://{host}")
    } else {
        format!("http://{host}:{port}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_origin_is_exact_and_loopback_only() {
        assert_eq!(
            dev_origin("http://localhost:5173/"),
            Some("http://localhost:5173".into())
        );
        assert_eq!(
            dev_origin("http://127.0.0.1:80/"),
            Some("http://127.0.0.1".into())
        );
        assert!(dev_origin("http://localhost:5173@evil/").is_none());
        assert!(dev_origin("https://example.org/").is_none());
        assert_eq!(
            dev_origin("HTTP://LOCALHOST:05173/path"),
            Some("http://localhost:5173".into())
        );
        assert_eq!(
            dev_origin("http://[::1]:5173/"),
            Some("http://[::1]:5173".into())
        );
        assert_eq!(dev_origin("http://[::1]/"), Some("http://[::1]".into()));
        assert!(dev_origin("http://[::1]:0/").is_none());
        assert!(dev_origin("http://[::1]evil:5173/").is_none());
        assert!(dev_origin("http://localhost:65536/").is_none());
    }
}
