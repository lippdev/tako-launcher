using System.Globalization;

namespace Launcher.Core.Presentation;

/// <summary>
/// Contraste mínimo WCAG 2.x AA (UX-009, DESIGN.md › Foco): texto 4.5:1, texto grande 3:1 e componentes (chave,
/// marcador, ícone, contorno) 3:1. Usado pelos testes que conferem os tokens do tema e as linhas focadas na tela real.
/// </summary>
public static class ContrastRules
{
    public const double Text = 4.5;

    public const double LargeText = 3.0;

    public const double Component = 3.0;

    /// <summary>
    /// Mínimo para um texto: grande é 18pt (24px) ou mais, ou 14pt (18.66px) em negrito (peso 700+). Os tamanhos são os do
    /// palco de 1080 unidades, que na TV ficam maiores; a regra é a do WCAG mesmo assim.
    /// </summary>
    public static double ForText(double fontSize, int fontWeight) =>
        fontSize >= 24 || (fontSize >= 18.66 && fontWeight >= 700) ? LargeText : Text;

    public static double Ratio(Rgb a, Rgb b) => AccentPalette.Contrast(a, b);

    /// <summary>Cor com transparência (<paramref name="alpha"/> de 0 a 1) desenhada sobre um fundo opaco.</summary>
    public static Rgb Over(Rgb color, double alpha, Rgb background)
    {
        double a = Math.Clamp(alpha, 0, 1);
        return new(Mix(color.R, background.R, a), Mix(color.G, background.G, a), Mix(color.B, background.B, a));
    }

    /// <summary>Lê "#AARRGGBB" (formato do WPF) ou "#RRGGBB" (alpha 1).</summary>
    public static (Rgb Color, double Alpha) ParseArgb(string hex)
    {
        ArgumentNullException.ThrowIfNull(hex);
        var value = hex.TrimStart('#');
        if (value.Length == 6)
        {
            return (Rgb.Parse(value), 1);
        }

        if (value.Length != 8 || !byte.TryParse(value.AsSpan(0, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out byte alpha))
        {
            throw new FormatException($"Cor inválida: '{hex}'.");
        }

        return (Rgb.Parse(value[2..]), alpha / 255d);
    }

    private static byte Mix(byte top, byte bottom, double alpha) => (byte)Math.Round((alpha * top) + ((1 - alpha) * bottom));
}
