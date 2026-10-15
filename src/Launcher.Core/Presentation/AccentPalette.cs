using System.Globalization;

namespace Launcher.Core.Presentation;

/// <summary>Cor RGB de 8 bits por canal.</summary>
public readonly record struct Rgb(byte R, byte G, byte B)
{
    public static Rgb Parse(string hex)
    {
        ArgumentNullException.ThrowIfNull(hex);
        var value = hex.TrimStart('#');
        if (value.Length != 6 || !int.TryParse(value, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out int n))
        {
            throw new FormatException($"Cor inválida: '{hex}'.");
        }

        return new((byte)(n >> 16), (byte)(n >> 8), (byte)n);
    }

    /// <summary>Luminância relativa (WCAG).</summary>
    public double Luminance
    {
        get
        {
            static double Channel(byte c)
            {
                double v = c / 255d;
                return v <= 0.03928 ? v / 12.92 : Math.Pow((v + 0.055) / 1.055, 2.4);
            }

            return (0.2126 * Channel(R)) + (0.7152 * Channel(G)) + (0.0722 * Channel(B));
        }
    }

    public Rgb Scale(double factor) => new(Clamp(R * factor), Clamp(G * factor), Clamp(B * factor));

    public override string ToString() => $"#{R:X2}{G:X2}{B:X2}";

    private static byte Clamp(double v) => (byte)Math.Round(Math.Clamp(v, 0, 255));
}

/// <summary>Accent do jogo em foco e seu tom profundo (DESIGN.md › Colors).</summary>
public readonly record struct AccentPalette(Rgb Accent, Rgb Deep)
{
    public static Rgb Fallback { get; } = Rgb.Parse("#2B3A55");

    /// <summary>Escurece o accent até garantir contraste 4.5:1 com texto branco; o tom profundo é 55% dele.</summary>
    public static AccentPalette For(string? accentHex, bool dynamicColor)
    {
        var accent = dynamicColor && accentHex is not null ? Rgb.Parse(accentHex) : Fallback;
        while (accent.Luminance > 0.18)
        {
            accent = accent.Scale(0.92);
        }

        return new(accent, accent.Scale(0.55));
    }

    /// <summary>Razão de contraste WCAG entre duas cores.</summary>
    public static double Contrast(Rgb a, Rgb b)
    {
        double la = a.Luminance + 0.05, lb = b.Luminance + 0.05;
        return Math.Max(la, lb) / Math.Min(la, lb);
    }
}
