using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Navigation;
using Launcher.Core.Presentation;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Configurações em categorias, menu Modo Console e layout que não corta nada (ADR-0020).</summary>
public class SettingsLayoutTests
{
    // Faixa visível do palco de 1080: abaixo do cabeçalho (180) e acima da barra de dicas (950).
    private const double BandTop = 180;
    private const double BandBottom = 950;

    [Fact]
    public void Focused_row_below_the_band_scrolls_until_its_bottom_is_above_the_hint_bar()
    {
        // A lista começa em 140 (abaixo do título): a linha está em 140 + 800 na tela, não em 800.
        var row = new NavRect(0, 140 + 800, 1000, 92);

        double target = ScrollMath.Vertical(0, row, row, isFirst: false, BandTop, BandBottom);

        Assert.Equal(BandBottom, row.Bottom + target);
    }

    [Fact]
    public void Row_already_inside_the_band_does_not_move()
    {
        var row = new NavRect(0, 400, 1000, 92);

        Assert.Equal(-50, ScrollMath.Vertical(-50, row, row, isFirst: false, BandTop, BandBottom));
    }

    [Fact]
    public void Row_above_the_band_scrolls_down_to_the_band_top()
    {
        var row = new NavRect(0, 600, 1000, 92);

        double target = ScrollMath.Vertical(-600, row, row, isFirst: false, BandTop, BandBottom);

        Assert.Equal(BandTop, row.Top + target);
    }

    [Fact]
    public void First_item_brings_the_list_back_to_the_top()
    {
        var row = new NavRect(0, 140, 1000, 92);

        Assert.Equal(0, ScrollMath.Vertical(-900, row, row, isFirst: true, BandTop, BandBottom));
    }

    [Fact]
    public void Item_in_a_row_taller_than_the_band_anchors_on_itself()
    {
        var grid = new NavRect(0, 300, 1500, 2400);
        var cover = new NavRect(0, 1500, 204, 306);

        double target = ScrollMath.Vertical(0, grid, cover, isFirst: false, BandTop, BandBottom);

        Assert.True(cover.Top + target >= BandTop);
        Assert.True(cover.Bottom + target <= BandBottom);
    }

    [Fact]
    public void Scrolling_never_goes_past_the_top()
    {
        var row = new NavRect(0, 100, 1000, 92);

        Assert.Equal(0, ScrollMath.Vertical(0, row, row, isFirst: false, BandTop, BandBottom));
    }

    [Fact]
    public void Hints_that_fit_stay_as_they_are()
    {
        var layout = HintFit.Plan([200, 150, 180], [120, 90, 100], [9, 8, 1], available: 600);

        Assert.False(layout.Compact);
        Assert.Empty(layout.Hidden);
    }

    [Fact]
    public void Hints_abbreviate_before_anything_is_hidden()
    {
        var layout = HintFit.Plan([200, 150, 180], [120, 90, 100], [9, 8, 1], available: 400);

        Assert.True(layout.Compact);
        Assert.Empty(layout.Hidden);
    }

    [Fact]
    public void Least_important_hints_hide_first_and_the_most_important_always_stays()
    {
        var layout = HintFit.Plan([200, 150, 180, 160], [120, 90, 100, 100], [9, 8, 1, 1], available: 200);

        Assert.True(layout.Compact);
        Assert.Equal([1, 2, 3], layout.Hidden.Order());

        var tiny = HintFit.Plan([200, 150], [120, 90], [9, 8], available: 10);
        Assert.Equal([1], tiny.Hidden);
    }

    [Fact]
    public void Ties_hide_the_rightmost_hint_first()
    {
        var layout = HintFit.Plan([100, 100, 100], [100, 100, 100], [5, 1, 1], available: 200);

        Assert.Equal([2], layout.Hidden);
    }

    [Theory]
    [InlineData(false, "", null, FeatureState.Off)]
    [InlineData(true, "", null, FeatureState.NeedsSetup)]
    [InlineData(true, "TV da sala", false, FeatureState.Problem)]
    [InlineData(true, "TV da sala", true, FeatureState.On)]
    [InlineData(true, "TV da sala", null, FeatureState.On)]
    public void Game_screen_card_state(bool enabled, string screen, bool? connected, FeatureState expected)
    {
        var status = ConsoleFeatures.GameScreen(enabled, screen, connected);

        Assert.Equal(expected, status.State);
        Assert.DoesNotContain("MVP", status.Summary, StringComparison.Ordinal);
    }

    [Fact]
    public void Tv_card_asks_for_the_address_when_it_is_missing()
    {
        Assert.Equal(FeatureState.Off, ConsoleFeatures.Tv(TvControlConfig.Default).State);
        Assert.Equal(FeatureState.NeedsSetup, ConsoleFeatures.Tv(TvControlConfig.Default with { Enabled = true }).State);

        var on = ConsoleFeatures.Tv(TvControlConfig.Default with { Enabled = true, Host = "192.168.0.50", Kind = TvKind.LgWebOs, HdmiInput = 2, StandbyAfterGame = true });
        Assert.Equal(FeatureState.On, on.State);
        Assert.Equal("LG (webOS) · HDMI 2 · espera ao sair", on.Summary);
        Assert.Equal("Ligado", on.Badge);
    }

    [Fact]
    public void Fps_counter_card_follows_the_permission()
    {
        Assert.Equal(FeatureState.Problem, ConsoleFeatures.FpsCounter(false, true, PerfLogStatus.Ready, "Só FPS").State);
        Assert.Equal(FeatureState.Off, ConsoleFeatures.FpsCounter(true, false, PerfLogStatus.Ready, "Só FPS").State);
        Assert.Equal(FeatureState.NeedsSetup, ConsoleFeatures.FpsCounter(true, true, PerfLogStatus.NeedsSignOut, "Só FPS").State);
        Assert.Equal(FeatureState.NeedsSetup, ConsoleFeatures.FpsCounter(true, true, PerfLogStatus.NotGranted, "Só FPS").State);
        Assert.Equal("Estilo: Compacto", ConsoleFeatures.FpsCounter(true, true, PerfLogStatus.Ready, "Compacto").Summary);
    }

    [Fact]
    public void Fps_limit_card_explains_an_unsupported_driver()
    {
        var none = FrameRateLimitCapabilities.None;

        Assert.Equal(FeatureState.Problem, ConsoleFeatures.FpsLimit(none, 60, 120).State);
        Assert.Equal(none.Explanation, ConsoleFeatures.FpsLimit(none, 60, 120).Summary);
        Assert.Equal(FeatureState.Off, ConsoleFeatures.FpsLimit(null, 0, null).State);
        Assert.Equal(FeatureState.On, ConsoleFeatures.FpsLimit(null, 60, null).State);
    }

    [Fact]
    public void Game_audio_card_warns_when_the_output_is_gone()
    {
        Assert.Equal(FeatureState.Off, ConsoleFeatures.GameAudio(null, true).State);
        Assert.Equal(FeatureState.Problem, ConsoleFeatures.GameAudio("TV", false).State);
        Assert.Equal(FeatureState.On, ConsoleFeatures.GameAudio("TV", null).State);
    }

    [Fact]
    public void Wizard_suggests_the_hdmi_audio_of_the_chosen_screen()
    {
        (string, string)[] outputs =
        [
            ("speakers", "Alto-falantes (Realtek High Definition Audio)"),
            ("hdmi", "LG TV SSCR2 (NVIDIA High Definition Audio)"),
        ];

        Assert.Equal("hdmi", ConsoleFeatures.SuggestAudioOutput("LG TV SSCR2", outputs));
        Assert.Equal("hdmi", ConsoleFeatures.SuggestAudioOutput("LG TV", outputs));
        Assert.Null(ConsoleFeatures.SuggestAudioOutput("Monitor genérico", outputs));
        Assert.Null(ConsoleFeatures.SuggestAudioOutput(string.Empty, outputs));
    }
}
