//! Modo da interface (ADR-0027): desktop (janela para mouse e teclado, o padrão) ou tela cheia (controle e TV).
//! Os dois são apresentações do mesmo `Shell`/`Ctx`; aqui fica só a decisão de como abrir.

use tako_core::supervision::args;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiMode {
    #[default]
    Desktop,
    BigPicture,
}

/// Como a interface abre: o modo e se pergunta pela tela cheia ao achar um controle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Start {
    pub mode: UiMode,
    pub ask: bool,
}

/// `--big-picture` (modo Xbox/AnyFSE) e `--console-session` (Console Mode) vão direto para a tela cheia;
/// `--desktop` é o supervisor reabrindo no modo em que estava. Sem nada, desktop com a pergunta.
pub fn start(all: &[String]) -> Start {
    if args::has(all, args::BIG_PICTURE) || args::has(all, args::CONSOLE_SESSION) {
        return Start {
            mode: UiMode::BigPicture,
            ask: false,
        };
    }
    Start {
        mode: UiMode::Desktop,
        ask: !args::has(all, args::DESKTOP),
    }
}

/// Configuração "Abrir em" (onboarding, Configurações › Geral › Modo): `desktop` ou `big-picture`.
pub const OPEN_IN: &str = "OpenIn";
pub const OPEN_IN_BIG_PICTURE: &str = "big-picture";
pub const OPEN_IN_DESKTOP: &str = "desktop";

/// O modo de abertura com a preferência "Abrir em": só a abertura sem argumentos (`plain`) segue a preferência;
/// `--big-picture`, o Console Mode e o supervisor continuam decidindo.
pub fn preferred(mode: UiMode, plain: bool, settings: &crate::shell::Settings) -> UiMode {
    if mode == UiMode::Desktop && plain && settings.get(OPEN_IN) == Some(OPEN_IN_BIG_PICTURE) {
        UiMode::BigPicture
    } else {
        mode
    }
}

/// Configuração: `false` = "Não perguntar de novo" na pergunta do controle.
pub const ASK_SETTING: &str = "AskBigPictureWithController";

/// Por quanto tempo depois de abrir um controle achado ainda conta como "conectado ao abrir" (o SDL avisa os
/// controles já ligados nos primeiros quadros).
pub const ASK_WINDOW: std::time::Duration = std::time::Duration::from_secs(3);

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn open_in_full_screen_only_changes_the_plain_start() {
        let mut settings = crate::shell::Settings::new(None);
        assert_eq!(preferred(UiMode::Desktop, true, &settings), UiMode::Desktop);
        settings.set(OPEN_IN, OPEN_IN_BIG_PICTURE);
        assert_eq!(
            preferred(UiMode::Desktop, true, &settings),
            UiMode::BigPicture
        );
        // O supervisor reabrindo no desktop (`--desktop`) não é a abertura sem argumentos.
        assert_eq!(
            preferred(UiMode::Desktop, false, &settings),
            UiMode::Desktop
        );
    }

    #[test]
    fn opens_on_the_desktop_unless_told_otherwise() {
        assert_eq!(
            start(&v(&["tako"])),
            Start {
                mode: UiMode::Desktop,
                ask: true
            }
        );
        for flag in ["--big-picture", "--BIG-PICTURE", "--console-session"] {
            assert_eq!(
                start(&v(&["tako", flag])),
                Start {
                    mode: UiMode::BigPicture,
                    ask: false
                },
                "{flag}"
            );
        }
        // Reaberta pelo supervisor no desktop: sem perguntar de novo.
        assert_eq!(
            start(&v(&["tako", "--ui", "--desktop"])),
            Start {
                mode: UiMode::Desktop,
                ask: false
            }
        );
    }
}
