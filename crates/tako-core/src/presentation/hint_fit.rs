//! Port de `Presentation/HintFit.cs` (ADR-0020): a barra de dicas se adapta à largura. Primeiro abrevia (textos
//! curtos), depois esconde as dicas menos importantes. Nunca corta uma dica no meio.

use std::collections::BTreeSet;

/// Uma dica da barra.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hint {
    /// Largura com o texto inteiro (inclui o espaço antes dela).
    pub full: f64,
    /// Largura abreviada.
    pub compact: f64,
    /// Importância: a de menor valor some primeiro; no empate, a mais à direita.
    pub priority: i32,
}

/// `compact` usa os textos curtos; `hidden` são os índices que somem.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HintLayout {
    pub compact: bool,
    pub hidden: BTreeSet<usize>,
}

pub fn plan(hints: &[Hint], available: f64) -> HintLayout {
    if hints.iter().map(|h| h.full).sum::<f64>() <= available {
        return HintLayout::default();
    }
    let mut total: f64 = hints.iter().map(|h| h.compact).sum();
    let mut order: Vec<usize> = (0..hints.len()).collect();
    order.sort_by_key(|&i| (hints[i].priority, std::cmp::Reverse(i)));

    let mut hidden = BTreeSet::new();
    // A dica mais importante fica sempre, mesmo que sozinha não caiba.
    for &i in order.iter().take(hints.len().saturating_sub(1)) {
        if total <= available {
            break;
        }
        hidden.insert(i);
        total -= hints[i].compact;
    }
    HintLayout {
        compact: true,
        hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hints(full: &[f64], compact: &[f64], priority: &[i32]) -> Vec<Hint> {
        (0..full.len())
            .map(|i| Hint {
                full: full[i],
                compact: compact[i],
                priority: priority[i],
            })
            .collect()
    }

    #[test]
    fn hints_that_fit_stay_as_they_are() {
        let layout = plan(
            &hints(&[200.0, 150.0, 180.0], &[120.0, 90.0, 100.0], &[9, 8, 1]),
            600.0,
        );
        assert!(!layout.compact);
        assert!(layout.hidden.is_empty());
    }

    #[test]
    fn hints_abbreviate_before_anything_is_hidden() {
        let layout = plan(
            &hints(&[200.0, 150.0, 180.0], &[120.0, 90.0, 100.0], &[9, 8, 1]),
            400.0,
        );
        assert!(layout.compact);
        assert!(layout.hidden.is_empty());
    }

    #[test]
    fn least_important_hints_hide_first_and_the_most_important_always_stays() {
        let layout = plan(
            &hints(
                &[200.0, 150.0, 180.0, 160.0],
                &[120.0, 90.0, 100.0, 100.0],
                &[9, 8, 1, 1],
            ),
            200.0,
        );
        assert!(layout.compact);
        assert_eq!(layout.hidden.into_iter().collect::<Vec<_>>(), [1, 2, 3]);
        let tiny = plan(&hints(&[200.0, 150.0], &[120.0, 90.0], &[9, 8]), 10.0);
        assert_eq!(tiny.hidden.into_iter().collect::<Vec<_>>(), [1]);
        assert_eq!(plan(&[], 0.0), HintLayout::default());
    }

    #[test]
    fn ties_hide_the_rightmost_hint_first() {
        let layout = plan(&hints(&[100.0; 3], &[100.0; 3], &[5, 1, 1]), 200.0);
        assert_eq!(layout.hidden.into_iter().collect::<Vec<_>>(), [2]);
    }
}
