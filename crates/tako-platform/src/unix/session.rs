//! Qual sessão gráfica do Linux está rodando, para escolher o backend de telas e de cursor.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    /// gamescope (Steam Deck, modo jogo): a resolução é da sessão.
    Gamescope,
    X11,
    KdeWayland,
    GnomeWayland,
    /// Sway, Hyprland, river, Wayfire e afins (`wlr-randr`).
    Wlroots,
    /// Outro compositor Wayland.
    OtherWayland,
    Unknown,
}

/// `env` lê uma variável de ambiente (injetável nos testes).
pub fn detect(env: impl Fn(&str) -> Option<String>) -> Session {
    let set = |k: &str| env(k).is_some_and(|v| !v.is_empty());
    let desktop = env("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let has = |name: &str| desktop.split(':').any(|d| d == name);
    if set("GAMESCOPE_WAYLAND_DISPLAY") || has("gamescope") {
        return Session::Gamescope;
    }
    let kind = env("XDG_SESSION_TYPE").unwrap_or_default();
    let wayland = kind == "wayland" || (kind.is_empty() && set("WAYLAND_DISPLAY"));
    if !wayland {
        return if set("DISPLAY") || kind == "x11" {
            Session::X11
        } else {
            Session::Unknown
        };
    }
    if has("kde") {
        Session::KdeWayland
    } else if has("gnome") {
        Session::GnomeWayland
    } else if set("SWAYSOCK")
        || set("HYPRLAND_INSTANCE_SIGNATURE")
        || ["sway", "hyprland", "river", "wayfire", "labwc", "niri"]
            .iter()
            .any(|n| has(n))
    {
        Session::Wlroots
    } else {
        Session::OtherWayland
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(vars: &[(&str, &str)]) -> Session {
        detect(|k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        })
    }

    #[test]
    fn detects_sessions() {
        assert_eq!(
            with(&[("XDG_SESSION_TYPE", "x11"), ("DISPLAY", ":0")]),
            Session::X11
        );
        assert_eq!(with(&[("DISPLAY", ":0")]), Session::X11);
        assert_eq!(
            with(&[
                ("XDG_SESSION_TYPE", "wayland"),
                ("XDG_CURRENT_DESKTOP", "KDE")
            ]),
            Session::KdeWayland
        );
        assert_eq!(
            with(&[
                ("WAYLAND_DISPLAY", "wayland-0"),
                ("XDG_CURRENT_DESKTOP", "ubuntu:GNOME")
            ]),
            Session::GnomeWayland
        );
        assert_eq!(
            with(&[("XDG_SESSION_TYPE", "wayland"), ("SWAYSOCK", "/run/s")]),
            Session::Wlroots
        );
        assert_eq!(
            with(&[
                ("XDG_SESSION_TYPE", "wayland"),
                ("XDG_CURRENT_DESKTOP", "Hyprland")
            ]),
            Session::Wlroots
        );
        assert_eq!(
            with(&[("XDG_SESSION_TYPE", "wayland"), ("DISPLAY", ":1")]),
            Session::OtherWayland
        );
        assert_eq!(
            with(&[
                ("GAMESCOPE_WAYLAND_DISPLAY", "gamescope-0"),
                ("DISPLAY", ":1")
            ]),
            Session::Gamescope
        );
        assert_eq!(
            with(&[("XDG_CURRENT_DESKTOP", "gamescope")]),
            Session::Gamescope
        );
        assert_eq!(with(&[("XDG_SESSION_TYPE", "tty")]), Session::Unknown);
        assert_eq!(with(&[]), Session::Unknown);
    }
}
