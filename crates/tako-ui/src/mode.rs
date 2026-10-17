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
