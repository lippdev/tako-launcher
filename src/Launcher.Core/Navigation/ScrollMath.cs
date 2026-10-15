namespace Launcher.Core.Navigation;

/// <summary>
/// Rolagem vertical que acompanha o foco (MVP-028/029, ADR-0020). Tudo em coordenadas da tela (palco de 1080) com a
/// lista na posição sem rolagem: a faixa visível já desconta o cabeçalho, a barra de dicas e a moldura da lista, então
/// o item focado nunca para atrás deles.
/// </summary>
public static class ScrollMath
{
    /// <summary>Folga abaixo do item quando só ele (e não a fileira inteira) cabe na faixa.</summary>
    public const double BottomGap = 24;

    /// <summary>Folga acima do item quando só ele (e não a fileira inteira) cabe na faixa.</summary>
    public const double TopGap = 40;

    /// <summary>
    /// Deslocamento vertical (≤ 0) para o item focado ficar inteiro dentro da faixa visível.
    /// </summary>
    /// <param name="current">Deslocamento que a lista já tem como destino (não o valor no meio de uma animação).</param>
    /// <param name="row">Fileira que contém o item (o filho direto da lista), sem rolagem.</param>
    /// <param name="item">O item focado, sem rolagem.</param>
    /// <param name="isFirst">Primeira fileira ou primeiro item navegável da lista: a lista volta ao topo.</param>
    /// <param name="bandTop">Topo da faixa visível (abaixo do cabeçalho e da borda da moldura).</param>
    /// <param name="bandBottom">Base da faixa visível (acima da barra de dicas e da borda da moldura).</param>
    public static double Vertical(double current, NavRect row, NavRect item, bool isFirst, double bandTop, double bandBottom)
    {
        if (isFirst)
        {
            return 0;
        }

        double target = current;
        if (row.Bottom + current > bandBottom)
        {
            target = bandBottom - row.Bottom;
        }
        else if (row.Top + current < bandTop)
        {
            target = bandTop - row.Top;
        }

        // Fileira mais alta que a faixa (ex.: grade): ancora no próprio item.
        if (item.Bottom + target > bandBottom || item.Top + target < bandTop)
        {
            target = item.Bottom + current > bandBottom ? bandBottom - item.Bottom - BottomGap : bandTop - item.Top + TopGap;
        }

        return Math.Min(0, target);
    }
}
