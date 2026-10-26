//! Port de `Presentation/VirtualGrid.cs` (PERF-004): geometria de uma grade de itens do mesmo tamanho (capas), linha
//! a linha. Diz o tamanho total e quais itens tocam a área visível, para criar só esses. Uma fileira horizontal é
//! uma grade de uma linha (`columns == count`).

/// Faixa contínua de índices (vazia quando `last < first`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRange {
    pub first: i64,
    pub last: i64,
}

impl ItemRange {
    pub const EMPTY: ItemRange = ItemRange { first: 0, last: -1 };

    pub const fn new(first: i64, last: i64) -> ItemRange {
        ItemRange { first, last }
    }

    pub fn is_empty(&self) -> bool {
        self.last < self.first
    }

    pub fn count(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            (self.last - self.first + 1) as usize
        }
    }

    pub fn contains(&self, index: i64) -> bool {
        index >= self.first && index <= self.last
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualGrid {
    pub count: i64,
    pub columns: i64,
    pub item_width: f64,
    pub item_height: f64,
}

impl VirtualGrid {
    pub fn new(count: i64, columns: i64, item_width: f64, item_height: f64) -> VirtualGrid {
        VirtualGrid {
            count: count.max(0),
            columns: columns.max(1),
            item_width: item_width.max(0.0),
            item_height: item_height.max(0.0),
        }
    }

    pub fn rows(&self) -> i64 {
        (self.count + self.columns - 1) / self.columns
    }

    /// Largura ocupada (só as colunas usadas).
    pub fn width(&self) -> f64 {
        self.count.min(self.columns) as f64 * self.item_width
    }

    pub fn height(&self) -> f64 {
        self.rows() as f64 * self.item_height
    }

    /// Colunas que cabem na largura (pelo menos uma). Sem limite de largura, tudo numa linha só.
    pub fn columns_for(available_width: f64, item_width: f64, count: i64) -> i64 {
        if !available_width.is_finite() {
            return count.max(1);
        }
        if item_width <= 0.0 {
            return 1;
        }
        // Tolerância para larguras que são múltiplos exatos (como o WrapPanel).
        (((available_width + 1e-6) / item_width).floor() as i64).max(1)
    }

    /// Canto superior esquerdo do item na grade.
    pub fn position_of(&self, index: i64) -> (f64, f64) {
        (
            (index % self.columns) as f64 * self.item_width,
            (index / self.columns) as f64 * self.item_height,
        )
    }

    /// Itens que tocam a área visível (coordenadas da grade), mais `buffer_lines` linhas (ou itens, numa fileira de
    /// uma linha) de cada lado, para a navegação e a rolagem acharem o vizinho já criado.
    pub fn visible(
        &self,
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        buffer_lines: i64,
    ) -> ItemRange {
        if self.count == 0 || right <= left || bottom <= top {
            return ItemRange::EMPTY;
        }
        let buffer = buffer_lines.max(0);
        if self.rows() == 1 {
            if self.item_width <= 0.0 {
                return ItemRange::new(0, self.count - 1);
            }
            let first = ((left / self.item_width).floor() as i64 - buffer).max(0);
            let last = ((right / self.item_width).ceil() as i64 - 1 + buffer).min(self.count - 1);
            return if first > last {
                ItemRange::EMPTY
            } else {
                ItemRange::new(first, last)
            };
        }
        if self.item_height <= 0.0 {
            return ItemRange::new(0, self.count - 1);
        }
        if bottom <= 0.0 || top >= self.height() {
            // Fora da grade: só o buffer perto da borda mais próxima.
            return if bottom <= 0.0 {
                ItemRange::new(0, self.count.min(buffer * self.columns) - 1)
            } else {
                ItemRange::new(
                    ((self.rows() - buffer) * self.columns).max(0),
                    self.count - 1,
                )
            };
        }
        let first_row = ((top / self.item_height).floor() as i64 - buffer).max(0);
        let last_row =
            ((bottom / self.item_height).ceil() as i64 - 1 + buffer).min(self.rows() - 1);
        ItemRange::new(
            first_row * self.columns,
            (self.count - 1).min((last_row + 1) * self.columns - 1),
        )
    }

    /// Rolagem que deixa um item visível mexendo o mínimo (a regra do `ScrollContentPresenter` do WPF): já visível,
    /// não mexe; acima, alinha em cima; abaixo, alinha embaixo; maior que a área, alinha em cima.
    pub fn minimal_scroll(view_start: f64, view_size: f64, item_start: f64, item_size: f64) -> f64 {
        let view_end = view_start + view_size;
        let item_end = item_start + item_size;
        let before = item_start < view_start && item_end < view_end;
        let after = item_end > view_end && item_start > view_start;
        let larger = item_size > view_size;
        if (before && !larger) || (after && larger) {
            item_start
        } else if before || after {
            item_end - view_size
        } else {
            view_start
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Grade da biblioteca: capas de 240 × 440, sete colunas num palco 16:9.
    fn library() -> VirtualGrid {
        VirtualGrid::new(1000, 7, 240.0, 440.0)
    }

    #[test]
    fn columns_wrap_like_the_wrap_panel() {
        assert_eq!(VirtualGrid::columns_for(1784.0, 240.0, 1000), 7);
        assert_eq!(VirtualGrid::columns_for(1680.0, 240.0, 1000), 7); // múltiplo exato: cabe
        assert_eq!(VirtualGrid::columns_for(1679.9, 240.0, 1000), 6);
        assert_eq!(VirtualGrid::columns_for(100.0, 240.0, 1000), 1); // pelo menos uma coluna
        assert_eq!(VirtualGrid::columns_for(f64::INFINITY, 230.0, 12), 12); // sem limite: uma linha só
    }

    #[test]
    fn extent_covers_every_item_even_when_only_a_few_exist() {
        let lib = library();
        assert_eq!(lib.rows(), 143);
        assert_eq!(lib.width(), 7.0 * 240.0);
        assert_eq!(lib.height(), 143.0 * 440.0);
        assert_eq!(lib.position_of(17), (240.0 * 3.0, 440.0 * 2.0));
        let few = VirtualGrid::new(3, 7, 240.0, 440.0);
        assert_eq!(few.width(), 3.0 * 240.0); // só as colunas usadas
        assert_eq!(few.height(), 440.0);
    }

    #[test]
    fn only_rows_touching_the_viewport_plus_the_buffer_are_realized() {
        let range = library().visible(0.0, 4400.0, 1680.0, 4400.0 + 1080.0, 2);
        assert_eq!(range.first, 8 * 7);
        assert_eq!(range.last, 15 * 7 - 1);
        assert_eq!(range.count(), 7 * 7);
    }

    #[test]
    fn viewport_at_the_top_or_bottom_is_clamped_to_the_grid() {
        let lib = library();
        let top = lib.visible(0.0, -300.0, 1680.0, 780.0, 2);
        assert_eq!((top.first, top.last), (0, 4 * 7 - 1));
        let bottom = lib.visible(0.0, lib.height() - 500.0, 1680.0, lib.height() + 580.0, 2);
        assert_eq!(bottom.last, 999);
        assert_eq!(bottom.first, (141 - 2) * 7);
    }

    #[test]
    fn viewport_outside_the_grid_keeps_only_the_buffer_near_the_edge() {
        let lib = library();
        assert_eq!(
            lib.visible(0.0, -5000.0, 1680.0, -3000.0, 2),
            ItemRange::new(0, 13)
        );
        assert_eq!(
            lib.visible(0.0, lib.height() + 10.0, 1680.0, lib.height() + 1000.0, 2),
            ItemRange::new(141 * 7, 999)
        );
        assert!(lib.visible(0.0, -5000.0, 1680.0, -3000.0, 0).is_empty());
    }

    #[test]
    fn single_row_realizes_by_column() {
        let row = VirtualGrid::new(40, 40, 230.0, 450.0);
        let range = row.visible(2300.0, 0.0, 2300.0 + 1800.0, 450.0, 3);
        assert_eq!((range.first, range.last), (7, 17 + 3));
        assert!(row.visible(0.0, 0.0, 0.0, 450.0, 3).is_empty());
    }

    #[test]
    fn empty_grid_has_nothing_to_realize() {
        let empty = VirtualGrid::new(0, 7, 240.0, 440.0);
        assert!(empty.visible(0.0, 0.0, 1680.0, 1080.0, 2).is_empty());
        assert_eq!(empty.height(), 0.0);
    }

    #[test]
    fn range_reports_its_size_and_membership() {
        let range = ItemRange::new(10, 40);
        assert_eq!(range.count(), 31);
        assert!(range.contains(10));
        assert!(!range.contains(41));
        assert_eq!(ItemRange::EMPTY.count(), 0);
        assert!(!ItemRange::EMPTY.contains(0));
    }

    #[test]
    fn minimal_scroll_matches_the_wpf_scroll_presenter() {
        assert_eq!(VirtualGrid::minimal_scroll(0.0, 1000.0, 200.0, 300.0), 0.0); // visível
        assert_eq!(
            VirtualGrid::minimal_scroll(500.0, 1000.0, 200.0, 300.0),
            200.0
        ); // acima
        assert_eq!(
            VirtualGrid::minimal_scroll(0.0, 1000.0, 900.0, 300.0),
            200.0
        ); // abaixo
        assert_eq!(VirtualGrid::minimal_scroll(0.0, 500.0, 900.0, 800.0), 900.0); // maior que a área
    }
}
