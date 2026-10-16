//! Port de `Navigation/ScrollMath.cs` e `Navigation/RowReveal.cs`: rolagem que acompanha o foco.

use super::NavRect;

/// Folga abaixo do item quando só ele (e não a fileira inteira) cabe na faixa.
pub const BOTTOM_GAP: f64 = 24.0;
/// Folga acima do item quando só ele (e não a fileira inteira) cabe na faixa.
pub const TOP_GAP: f64 = 40.0;

/// Deslocamento vertical (≤ 0) para o item focado ficar inteiro na faixa visível (MVP-028/029, ADR-0020).
///
/// Tudo em coordenadas da tela (palco de 1080) com a lista sem rolagem; a faixa já desconta o cabeçalho, a barra de
/// dicas e a moldura. `current` é o destino atual (não o valor no meio de uma animação); `row` é a fileira (filho
/// direto da lista) e `item` o focado; `is_first` (primeira fileira ou item da lista) volta ao topo.
pub fn vertical_scroll(
    current: f64,
    row: NavRect,
    item: NavRect,
    is_first: bool,
    band_top: f64,
    band_bottom: f64,
) -> f64 {
    if is_first {
        return 0.0;
    }
    let mut target = current;
    if row.bottom() + current > band_bottom {
        target = band_bottom - row.bottom();
    } else if row.top + current < band_top {
        target = band_top - row.top;
    }
    // Fileira mais alta que a faixa (ex.: grade): ancora no próprio item.
    if item.bottom() + target > band_bottom || item.top + target < band_top {
        target = if item.bottom() + current > band_bottom {
            band_bottom - item.bottom() - BOTTOM_GAP
        } else {
            band_top - item.top + TOP_GAP
        };
    }
    target.min(0.0)
}

/// Rolagem horizontal "mínima" das fileiras da Central de Controle: o item fica inteiro na janela com `padding` de
/// folga dos dois lados e a fileira só anda o necessário. Devolve o novo deslocamento (≥ 0), em coordenadas da fileira.
pub fn row_reveal_offset(
    item_left: f64,
    item_width: f64,
    content_width: f64,
    visible_width: f64,
    padding: f64,
    current: f64,
) -> f64 {
    let max = (content_width - visible_width).max(0.0);
    let mut offset = current.clamp(0.0, max);
    if item_left - padding < offset {
        offset = item_left - padding;
    } else if item_left + item_width + padding > offset + visible_width {
        offset = item_left + item_width + padding - visible_width;
    }
    offset.clamp(0.0, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Faixa visível do palco de 1080: abaixo do cabeçalho (180) e acima da barra de dicas (950).
    const TOP: f64 = 180.0;
    const BOTTOM: f64 = 950.0;

    fn rect(top: f64, height: f64) -> NavRect {
        NavRect::new(0.0, top, 1000.0, height)
    }

    #[test]
    fn focused_row_below_the_band_scrolls_until_its_bottom_is_above_the_hint_bar() {
        let row = rect(140.0 + 800.0, 92.0); // a lista começa em 140
        assert_eq!(
            row.bottom() + vertical_scroll(0.0, row, row, false, TOP, BOTTOM),
            BOTTOM
        );
    }

    #[test]
    fn row_already_inside_the_band_does_not_move() {
        let row = rect(400.0, 92.0);
        assert_eq!(vertical_scroll(-50.0, row, row, false, TOP, BOTTOM), -50.0);
    }

    #[test]
    fn row_above_the_band_scrolls_down_to_the_band_top() {
        let row = rect(600.0, 92.0);
        assert_eq!(
            row.top + vertical_scroll(-600.0, row, row, false, TOP, BOTTOM),
            TOP
        );
    }

    #[test]
    fn first_item_brings_the_list_back_to_the_top() {
        let row = rect(140.0, 92.0);
        assert_eq!(vertical_scroll(-900.0, row, row, true, TOP, BOTTOM), 0.0);
    }

    #[test]
    fn item_in_a_row_taller_than_the_band_anchors_on_itself() {
        let grid = NavRect::new(0.0, 300.0, 1500.0, 2400.0);
        let cover = NavRect::new(0.0, 1500.0, 204.0, 306.0);
        let target = vertical_scroll(0.0, grid, cover, false, TOP, BOTTOM);
        assert!(cover.top + target >= TOP);
        assert!(cover.bottom() + target <= BOTTOM);
    }

    #[test]
    fn scrolling_never_goes_past_the_top() {
        let row = rect(100.0, 92.0);
        assert_eq!(vertical_scroll(0.0, row, row, false, TOP, BOTTOM), 0.0);
    }

    #[test]
    fn row_reveal_moves_only_what_is_needed_and_keeps_padding() {
        // Cartões de 300 com 20 de espaço; janela visível de 1000.
        assert_eq!(
            row_reveal_offset(640.0, 300.0, 3200.0, 1000.0, 16.0, 0.0),
            0.0
        );
        assert_eq!(
            row_reveal_offset(960.0, 300.0, 3200.0, 1000.0, 16.0, 0.0),
            276.0
        );
        assert_eq!(
            row_reveal_offset(320.0, 300.0, 3200.0, 1000.0, 16.0, 600.0),
            304.0
        );
        assert_eq!(
            row_reveal_offset(0.0, 300.0, 3200.0, 1000.0, 16.0, 600.0),
            0.0
        );
    }

    #[test]
    fn row_reveal_never_scrolls_past_the_end_or_a_short_row() {
        assert_eq!(
            row_reveal_offset(2880.0, 300.0, 3200.0, 1000.0, 16.0, 0.0),
            2196.0
        );
        assert_eq!(
            row_reveal_offset(2880.0, 300.0, 3200.0, 1000.0, 40.0, 0.0),
            2200.0
        );
        // Fileira menor que a janela: nada anda.
        assert_eq!(
            row_reveal_offset(320.0, 300.0, 640.0, 1000.0, 16.0, 50.0),
            0.0
        );
    }
}
