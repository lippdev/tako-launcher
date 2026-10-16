//! Port de `Navigation/SpatialNavigator.cs` (MVP-028/029): escolhe o vizinho mais próximo na direção pedida,
//! penalizando o desvio lateral. Mesmo algoritmo do protótipo em `design/prototype`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Retângulo em coordenadas de tela.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NavRect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl NavRect {
    pub const fn new(left: f64, top: f64, width: f64, height: f64) -> NavRect {
        NavRect {
            left,
            top,
            width,
            height,
        }
    }

    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.top + self.height
    }

    pub fn center_x(&self) -> f64 {
        self.left + self.width / 2.0
    }

    pub fn center_y(&self) -> f64 {
        self.top + self.height / 2.0
    }
}

/// Candidato à navegação: item, posição e grupo (fileira, barra lateral...).
#[derive(Debug, Clone, PartialEq)]
pub struct NavCandidate<T> {
    pub item: T,
    pub rect: NavRect,
    pub group: String,
}

const TOLERANCE: f64 = 2.0;
const ORTHOGONAL_WEIGHT: f64 = 2.2;

pub fn find_next<'a, T>(
    from: NavRect,
    candidates: impl IntoIterator<Item = &'a NavCandidate<T>>,
    direction: NavDirection,
) -> Option<&'a NavCandidate<T>> {
    let mut best = None;
    let mut best_score = f64::MAX;
    for candidate in candidates {
        let c = candidate.rect;
        if c == from {
            continue;
        }
        let (primary, orthogonal) = match direction {
            NavDirection::Left
                if !(c.right() > from.left + TOLERANCE
                    && c.center_x() >= from.center_x() - TOLERANCE) =>
            {
                (
                    from.left - c.right(),
                    (c.center_y() - from.center_y()).abs(),
                )
            }
            NavDirection::Right
                if !(c.left < from.right() - TOLERANCE
                    && c.center_x() <= from.center_x() + TOLERANCE) =>
            {
                (
                    c.left - from.right(),
                    (c.center_y() - from.center_y()).abs(),
                )
            }
            NavDirection::Up
                if !(c.bottom() > from.top + TOLERANCE
                    && c.center_y() >= from.center_y() - TOLERANCE) =>
            {
                (
                    from.top - c.bottom(),
                    (c.center_x() - from.center_x()).abs(),
                )
            }
            NavDirection::Down
                if !(c.top < from.bottom() - TOLERANCE
                    && c.center_y() <= from.center_y() + TOLERANCE) =>
            {
                (
                    c.top - from.bottom(),
                    (c.center_x() - from.center_x()).abs(),
                )
            }
            _ => continue,
        };
        let score = primary.max(0.0) + orthogonal * ORTHOGONAL_WEIGHT;
        if score < best_score {
            best_score = score;
            best = Some(candidate);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use NavDirection::*;

    // Duas fileiras de capas (como a Início) e um botão acima.
    fn layout() -> Vec<NavCandidate<&'static str>> {
        [
            ("play", NavRect::new(184.0, 380.0, 210.0, 76.0), "hero"),
            ("r1c1", NavRect::new(184.0, 560.0, 204.0, 306.0), "row1"),
            ("r1c2", NavRect::new(414.0, 560.0, 204.0, 306.0), "row1"),
            ("r1c3", NavRect::new(644.0, 560.0, 204.0, 306.0), "row1"),
            ("r2c1", NavRect::new(184.0, 940.0, 204.0, 306.0), "row2"),
            ("r2c2", NavRect::new(414.0, 940.0, 204.0, 306.0), "row2"),
        ]
        .into_iter()
        .map(|(item, rect, group)| NavCandidate {
            item,
            rect,
            group: group.into(),
        })
        .collect()
    }

    fn next(from: &str, direction: NavDirection) -> Option<&'static str> {
        let layout = layout();
        let rect = layout.iter().find(|c| c.item == from).unwrap().rect;
        find_next(rect, &layout, direction).map(|c| c.item)
    }

    #[test]
    fn picks_the_nearest_neighbor_in_the_direction() {
        assert_eq!(next("r1c1", Right), Some("r1c2"));
        assert_eq!(next("r1c2", Left), Some("r1c1"));
        assert_eq!(next("r1c2", Down), Some("r2c2"));
        assert_eq!(next("r1c3", Up), Some("play"));
        assert_eq!(next("play", Down), Some("r1c1"));
    }

    #[test]
    fn returns_nothing_at_the_edge() {
        assert_eq!(next("r1c3", Right), None);
        assert_eq!(next("play", Up), None);
    }
}
