namespace Launcher.Core.Navigation;

/// <summary>
/// Rolagem horizontal "mínima" das fileiras da Central de Controle: o item focado fica inteiro na área visível, com
/// <c>padding</c> de folga dos dois lados (anel de foco e sombra), e a fileira só anda o necessário. Diferente das
/// fileiras da biblioteca, que levam o item focado para a primeira posição.
/// </summary>
public static class RowReveal
{
    /// <summary>
    /// Novo deslocamento (≥ 0, conteúdo andando para a esquerda) para mostrar o item que começa em <paramref name="itemLeft"/>
    /// com largura <paramref name="itemWidth"/>, numa fileira de largura <paramref name="contentWidth"/> vista por uma
    /// janela de <paramref name="visibleWidth"/>. Tudo em coordenadas da fileira.
    /// </summary>
    public static double Offset(double itemLeft, double itemWidth, double contentWidth, double visibleWidth, double padding, double current)
    {
        double max = Math.Max(0, contentWidth - visibleWidth);
        double offset = Math.Clamp(current, 0, max);
        if (itemLeft - padding < offset)
        {
            offset = itemLeft - padding;
        }
        else if (itemLeft + itemWidth + padding > offset + visibleWidth)
        {
            offset = itemLeft + itemWidth + padding - visibleWidth;
        }

        return Math.Clamp(offset, 0, max);
    }
}
