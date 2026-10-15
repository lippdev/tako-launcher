namespace Launcher.Core.Presentation;

/// <summary>Faixa contínua de índices de itens (vazia quando <see cref="Last"/> &lt; <see cref="First"/>).</summary>
public readonly record struct ItemRange(int First, int Last)
{
    public static ItemRange Empty { get; } = new(0, -1);

    public bool IsEmpty => Last < First;

    public int Count => IsEmpty ? 0 : Last - First + 1;

    public bool Contains(int index) => index >= First && index <= Last;
}

/// <summary>
/// Geometria de uma grade de itens do mesmo tamanho (capas), linha a linha, da esquerda para a direita. Base dos painéis
/// virtualizados (PERF-004): diz o tamanho total da grade e quais itens tocam a área visível, para criar só esses.
/// Uma fileira horizontal é uma grade de uma linha (<see cref="Columns"/> = <see cref="Count"/>).
/// </summary>
public readonly record struct VirtualGrid
{
    public VirtualGrid(int count, int columns, double itemWidth, double itemHeight)
    {
        Count = Math.Max(0, count);
        Columns = Math.Max(1, columns);
        ItemWidth = Math.Max(0, itemWidth);
        ItemHeight = Math.Max(0, itemHeight);
    }

    public int Count { get; }

    public int Columns { get; }

    public double ItemWidth { get; }

    public double ItemHeight { get; }

    public int Rows => (Count + Columns - 1) / Columns;

    /// <summary>Largura ocupada (como o WrapPanel: só as colunas usadas).</summary>
    public double Width => Math.Min(Count, Columns) * ItemWidth;

    public double Height => Rows * ItemHeight;

    /// <summary>
    /// Colunas que cabem em <paramref name="availableWidth"/>, como o WrapPanel quebra a linha (pelo menos uma). Sem
    /// limite de largura, tudo fica numa linha só.
    /// </summary>
    public static int ColumnsFor(double availableWidth, double itemWidth, int count)
    {
        if (double.IsInfinity(availableWidth) || double.IsNaN(availableWidth))
        {
            return Math.Max(1, count);
        }

        if (itemWidth <= 0)
        {
            return 1;
        }

        // Mesma tolerância do WrapPanel para larguras que são múltiplos exatos.
        return Math.Max(1, (int)Math.Floor((availableWidth + 1e-6) / itemWidth));
    }

    /// <summary>Posição do item (canto superior esquerdo) na grade.</summary>
    public (double X, double Y) PositionOf(int index) => (index % Columns * ItemWidth, index / Columns * ItemHeight);

    /// <summary>
    /// Itens que tocam a área visível (coordenadas da grade), mais <paramref name="bufferLines"/> linhas (ou itens, numa
    /// fileira de uma linha só) de cada lado para a navegação e a rolagem acharem o vizinho já criado.
    /// </summary>
    public ItemRange Visible(double left, double top, double right, double bottom, int bufferLines)
    {
        if (Count == 0 || right <= left || bottom <= top)
        {
            return ItemRange.Empty;
        }

        bufferLines = Math.Max(0, bufferLines);
        if (Rows == 1)
        {
            if (ItemWidth <= 0)
            {
                return new(0, Count - 1);
            }

            int firstColumn = Math.Max(0, (int)Math.Floor(left / ItemWidth) - bufferLines);
            int lastColumn = Math.Min(Count - 1, (int)Math.Ceiling(right / ItemWidth) - 1 + bufferLines);
            return firstColumn > lastColumn ? ItemRange.Empty : new(firstColumn, lastColumn);
        }

        if (ItemHeight <= 0)
        {
            return new(0, Count - 1);
        }

        if (bottom <= 0 || top >= Height)
        {
            // Fora da grade: só o buffer perto da borda mais próxima.
            return bottom <= 0
                ? new(0, Math.Min(Count, bufferLines * Columns) - 1)
                : new(Math.Max(0, (Rows - bufferLines) * Columns), Count - 1);
        }

        int firstRow = Math.Max(0, (int)Math.Floor(top / ItemHeight) - bufferLines);
        int lastRow = Math.Min(Rows - 1, (int)Math.Ceiling(bottom / ItemHeight) - 1 + bufferLines);
        return new(firstRow * Columns, Math.Min(Count - 1, ((lastRow + 1) * Columns) - 1));
    }

    /// <summary>
    /// Deslocamento de rolagem que deixa um item visível mexendo o mínimo (mesma regra do <c>ScrollContentPresenter</c>
    /// do WPF, para a grade virtualizada rolar como a grade antiga): já visível, não mexe; acima, alinha em cima;
    /// abaixo, alinha embaixo; maior que a área, alinha em cima.
    /// </summary>
    public static double MinimalScroll(double viewStart, double viewSize, double itemStart, double itemSize)
    {
        double viewEnd = viewStart + viewSize;
        double itemEnd = itemStart + itemSize;
        bool before = itemStart < viewStart && itemEnd < viewEnd;
        bool after = itemEnd > viewEnd && itemStart > viewStart;
        bool larger = itemSize > viewSize;
        if ((before && !larger) || (after && larger))
        {
            return itemStart;
        }

        return before || after ? itemEnd - viewSize : viewStart;
    }
}
