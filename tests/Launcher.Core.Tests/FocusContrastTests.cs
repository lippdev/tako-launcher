using System.Xml.Linq;
using Launcher.Core.Presentation;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// Linhas focadas (MVP-029, UX-009): a linha focada é branca (surface-solid) e os controles dentro dela trocam para as
/// cores "OnSolid". Confere os tokens do tema padrão (o mesmo Tokens.xaml que o app carrega) contra o WCAG AA; na
/// alpha.10 a chave era branca sobre a linha branca e sumia.
/// </summary>
public class FocusContrastTests
{
    private static readonly Dictionary<string, (Rgb Color, double Alpha)> Tokens = LoadTokens();

    private static Rgb Solid => Opaque("SurfaceSolidBrush");

    /// <summary>Painel lateral (filtros, Central): PanelStrongBrush sobre o fundo do app.</summary>
    private static Rgb Panel
    {
        get
        {
            var panel = Tokens["PanelStrongBrush"];
            return ContrastRules.Over(panel.Color, panel.Alpha, Tokens["CanvasColor"].Color);
        }
    }

    [Theory]
    [InlineData("OnSurfaceSolidBrush", ContrastRules.Text)] // título e valor em destaque
    [InlineData("OnSurfaceSolidMutedBrush", ContrastRules.Text)] // contagem e valor no fim da linha
    [InlineData("ControlOnOnSolidBrush", ContrastRules.Text)] // bateria (texto verde) e trilho da chave ligada
    [InlineData("ControlOffOnSolidBrush", ContrastRules.Component)] // chave desligada (contorno e bolinha) e ○
    public void Controls_on_the_focused_row_contrast_with_the_white_surface(string token, double minimum)
    {
        double ratio = ContrastRules.Ratio(On(token, Solid), Solid);

        Assert.True(ratio >= minimum, $"{token} sobre surface-solid: {ratio:0.00}:1 (mínimo {minimum}:1)");
    }

    [Fact]
    public void Knob_of_a_switched_on_toggle_contrasts_with_its_track_on_the_white_row()
    {
        var track = On("ControlOnOnSolidBrush", Solid);

        Assert.True(ContrastRules.Ratio(On("ControlKnobBrush", track), track) >= ContrastRules.Component);
    }

    [Theory]
    [InlineData("OnAccentBrush", ContrastRules.Text)]
    [InlineData("OnAccentMutedBrush", ContrastRules.Text)]
    [InlineData("SuccessBrush", ContrastRules.Text)] // bateria; trilho da chave ligada
    [InlineData("ControlOffBrush", ContrastRules.Component)] // ○ desligado
    public void Controls_on_an_unfocused_row_contrast_with_the_panel(string token, double minimum)
    {
        double ratio = ContrastRules.Ratio(On(token, Panel), Panel);

        Assert.True(ratio >= minimum, $"{token} sobre o painel: {ratio:0.00}:1 (mínimo {minimum}:1)");
    }

    [Fact]
    public void Knob_of_a_switched_off_toggle_contrasts_with_its_track_on_the_panel()
    {
        var track = On("ControlTrackOffBrush", Panel);

        Assert.True(ContrastRules.Ratio(On("ControlKnobBrush", track), track) >= ContrastRules.Component);
    }

    [Fact]
    public void No_control_color_disappears_on_the_white_row()
    {
        // O defeito relatado: a chave e a bolinha eram brancas (ou quase) sobre a linha branca.
        foreach (string token in new[] { "OnSurfaceSolidMutedBrush", "ControlOnOnSolidBrush", "ControlOffOnSolidBrush" })
        {
            Assert.NotEqual(Solid, On(token, Solid));
        }
    }

    [Theory]
    [InlineData(18, 400, ContrastRules.Text)]
    [InlineData(20, 600, ContrastRules.Text)]
    [InlineData(20, 700, ContrastRules.LargeText)]
    [InlineData(24, 400, ContrastRules.LargeText)]
    public void Large_text_follows_the_wcag_size_rule(double size, int weight, double expected) =>
        Assert.Equal(expected, ContrastRules.ForText(size, weight));

    [Fact]
    public void Translucent_colors_are_blended_over_the_background()
    {
        var (color, alpha) = ContrastRules.ParseArgb("#8011141A");

        Assert.Equal(new Rgb(0x11, 0x14, 0x1A), color);
        Assert.Equal(new Rgb(0x88, 0x89, 0x8C), ContrastRules.Over(color, alpha, new Rgb(255, 255, 255)));
        Assert.Equal((new Rgb(255, 255, 255), 1d), ContrastRules.ParseArgb("#FFFFFF"));
        Assert.Throws<FormatException>(() => ContrastRules.ParseArgb("#12"));
    }

    private static Rgb Opaque(string key) => Tokens[key].Color;

    private static Rgb On(string key, Rgb background)
    {
        var (color, alpha) = Tokens[key];
        return ContrastRules.Over(color, alpha, background);
    }

    private static Dictionary<string, (Rgb, double)> LoadTokens()
    {
        XNamespace x = "http://schemas.microsoft.com/winfx/2006/xaml";
        var doc = XDocument.Load(Path.Combine(AppContext.BaseDirectory, "Fixtures", "Theme", "Tokens.xaml"));
        var result = new Dictionary<string, (Rgb, double)>(StringComparer.Ordinal);
        foreach (var element in doc.Root!.Elements())
        {
            string? key = (string?)element.Attribute(x + "Key");
            string? value = element.Name.LocalName switch
            {
                "SolidColorBrush" => (string?)element.Attribute("Color"),
                "Color" => element.Value,
                _ => null,
            };
            if (key is not null && value is not null)
            {
                result[key] = ContrastRules.ParseArgb(value.Trim());
            }
        }

        return result;
    }
}
