using Launcher.Core.Input;
using Launcher.Core.Navigation;
using Xunit;

namespace Launcher.Core.Tests;

public class ControlCenterShortcutTests
{
    private static readonly TimeSpan Hold = ControlCenterChord.HoldTime;

    private static TimeSpan Ms(int ms) => TimeSpan.FromMilliseconds(ms);

    [Fact]
    public void Default_chord_is_view_plus_y_named_per_controller()
    {
        Assert.Equal(GamepadButtons.View | GamepadButtons.Y, ControlCenterChord.Default);
        Assert.Equal("View + Y", ControlCenterChord.Format(ControlCenterChord.Default, GamepadFamily.Xbox));
        Assert.Equal("Create + △", ControlCenterChord.Format(ControlCenterChord.Default, GamepadFamily.PlayStation5));
        Assert.Equal("Share + △", ControlCenterChord.Format(ControlCenterChord.Default, GamepadFamily.PlayStation4));
    }

    [Fact]
    public void Chord_fires_once_after_holding_half_a_second()
    {
        var chord = new ButtonChordDetector(ControlCenterChord.Default);
        var both = GamepadButtons.View | GamepadButtons.Y;

        Assert.False(chord.Update(both, Ms(0)));
        Assert.False(chord.Update(both, Ms(400)));
        Assert.True(chord.Update(both, Hold));
        Assert.False(chord.Update(both, Ms(2000))); // continua segurando: não fecha logo depois de abrir

        Assert.False(chord.Update(GamepadButtons.View, Ms(2100)));
        Assert.False(chord.Update(both, Ms(2200)));
        Assert.True(chord.Update(both, Ms(2200) + Hold));
    }

    [Fact]
    public void Quick_press_does_not_open()
    {
        var chord = new ButtonChordDetector(ControlCenterChord.Default);
        var both = GamepadButtons.View | GamepadButtons.Y;

        Assert.False(chord.Update(both, Ms(0)));
        Assert.False(chord.Update(both, Ms(300)));
        Assert.False(chord.Update(GamepadButtons.None, Ms(350)));
        Assert.False(chord.Update(both, Ms(400)));
        Assert.False(chord.Update(both, Ms(800))); // recomeçou a contar em 400 ms
    }

    [Fact]
    public void Extra_buttons_do_not_block_the_chord()
    {
        var chord = new ButtonChordDetector(ControlCenterChord.Default);
        var withStick = GamepadButtons.View | GamepadButtons.Y | GamepadButtons.DPadUp;

        Assert.False(chord.Update(withStick, Ms(0)));
        Assert.True(chord.Update(withStick, Hold));
    }

    [Fact]
    public void Changing_the_chord_waits_for_release()
    {
        var chord = new ButtonChordDetector(ControlCenterChord.Default);
        var newChord = GamepadButtons.View | GamepadButtons.X;
        chord.Chord = newChord;

        Assert.False(chord.Update(newChord, Ms(0)));
        Assert.False(chord.Update(newChord, Ms(2000))); // ainda segurando os botões da tela de captura
        Assert.False(chord.Update(GamepadButtons.None, Ms(2100)));
        Assert.False(chord.Update(newChord, Ms(2200)));
        Assert.True(chord.Update(newChord, Ms(2200) + Hold));
    }

    [Fact]
    public void Free_buttons_held_suppress_the_rest_of_the_chord()
    {
        var chord = new ButtonChordDetector(ControlCenterChord.Default);

        Assert.Equal(GamepadButtons.Y, chord.Suppressed(GamepadButtons.View));
        Assert.Equal(GamepadButtons.Y, chord.Suppressed(GamepadButtons.View | GamepadButtons.Y));
        Assert.Equal(GamepadButtons.None, chord.Suppressed(GamepadButtons.Y)); // Y sozinho continua sendo a busca
    }

    [Fact]
    public void Chord_buttons_still_count_as_held_for_the_steam_keyboard_echo_filter()
    {
        // Steam aberta: o Y segurado da combinação também chega como tecla; o filtro de eco precisa vê-lo mantido.
        var pad = new GamepadInterpreter();
        var echo = new InputEchoFilter();
        var chord = new ButtonChordDetector(ControlCenterChord.Default);
        var both = GamepadButtons.View | GamepadButtons.Y;

        Assert.Empty(pad.Update(new GamepadSnapshot(both, 0, 0), Ms(0), chord.Suppressed(both)));
        Assert.Contains(AppCommand.Search, pad.Pressed);
        Assert.False(echo.AcceptKeyboard(AppCommand.Search, Ms(20), pad.Pressed));
    }

    [Fact]
    public void Capture_ignores_buttons_held_when_it_opens_and_takes_the_stable_set()
    {
        var capture = new ChordCapture();

        Assert.Equal(ChordCaptureState.Releasing, capture.Update(GamepadButtons.A, Ms(0))); // o A que abriu a tela
        Assert.Equal(ChordCaptureState.Releasing, capture.Update(GamepadButtons.A, Ms(2000)));
        Assert.Equal(ChordCaptureState.Waiting, capture.Update(GamepadButtons.None, Ms(2100)));

        Assert.Equal(ChordCaptureState.Holding, capture.Update(GamepadButtons.View, Ms(2200)));
        Assert.Equal(ChordCaptureState.Holding, capture.Update(GamepadButtons.View | GamepadButtons.X, Ms(2500)));
        Assert.Equal(ChordCaptureState.Holding, capture.Update(GamepadButtons.View | GamepadButtons.X, Ms(3400))); // contagem recomeçou em 2500
        Assert.Equal(ChordCaptureState.Captured, capture.Update(GamepadButtons.View | GamepadButtons.X, Ms(3500)));
        Assert.Equal(GamepadButtons.View | GamepadButtons.X, capture.Result);
    }

    [Fact]
    public void Tapping_back_cancels_the_capture_but_holding_it_captures()
    {
        var tap = new ChordCapture();
        tap.Update(GamepadButtons.None, Ms(0));
        tap.Update(GamepadButtons.B, Ms(100));
        Assert.Equal(ChordCaptureState.Cancelled, tap.Update(GamepadButtons.None, Ms(300)));

        var held = new ChordCapture();
        held.Update(GamepadButtons.None, Ms(0));
        held.Update(GamepadButtons.B, Ms(100));
        Assert.Equal(ChordCaptureState.Captured, held.Update(GamepadButtons.B, Ms(1100)));
        Assert.Equal(GamepadButtons.B, held.Result); // as regras recusam depois, e a captura recomeça
    }

    [Theory]
    [InlineData(GamepadButtons.View | GamepadButtons.Y, true)]
    [InlineData(GamepadButtons.Guide, true)]
    [InlineData(GamepadButtons.View | GamepadButtons.DPadUp, true)]
    [InlineData(GamepadButtons.A, false)]
    [InlineData(GamepadButtons.LeftShoulder | GamepadButtons.RightShoulder, false)]
    [InlineData(GamepadButtons.Start | GamepadButtons.Y, false)]
    [InlineData(GamepadButtons.View | GamepadButtons.A | GamepadButtons.B | GamepadButtons.X, false)]
    [InlineData(GamepadButtons.None, false)]
    public void Chord_rules_reject_combinations_that_already_navigate(GamepadButtons chord, bool valid) =>
        Assert.Equal(valid, ShortcutRules.CheckChord(chord, GamepadFamily.Xbox).IsValid);

    [Fact]
    public void Chord_rules_explain_with_the_controller_names()
    {
        var check = ShortcutRules.CheckChord(GamepadButtons.LeftShoulder | GamepadButtons.RightShoulder, GamepadFamily.PlayStation5);
        Assert.Equal("L1 + R1 já servem para navegar no Tako · inclua Create ou PS", check.Error);
        Assert.Contains("Barra de Jogo", ShortcutRules.CheckChord(GamepadButtons.Guide, GamepadFamily.Xbox).Warning, StringComparison.Ordinal);
        Assert.NotNull(ShortcutRules.CheckChord(GamepadButtons.View, GamepadFamily.Xbox).Warning);
        Assert.Equal(ShortcutCheck.Ok, ShortcutRules.CheckChord(ControlCenterChord.Default, GamepadFamily.Xbox));
    }

    [Fact]
    public void Default_keyboard_shortcut_has_no_known_conflict()
    {
        Assert.Equal("Ctrl+Shift+T", KeyShortcut.Default.Display);
        Assert.Equal(ShortcutCheck.Ok, ShortcutRules.CheckKeyboard(KeyShortcut.Default));
    }

    [Theory]
    [InlineData(KeyModifiers.Shift, "Tab")] // Steam
    [InlineData(KeyModifiers.None, "F")] // favoritar no Tako
    [InlineData(KeyModifiers.None, "F10")]
    [InlineData(KeyModifiers.Alt, "Tab")] // Windows
    [InlineData(KeyModifiers.Alt, "F4")]
    [InlineData(KeyModifiers.Ctrl | KeyModifiers.Shift, "Escape")]
    [InlineData(KeyModifiers.Win, "G")] // Barra de Jogo
    [InlineData(KeyModifiers.Ctrl, "LeftShift")] // só modificadores
    public void Keyboard_rules_reject_tako_and_windows_shortcuts(KeyModifiers modifiers, string key) =>
        Assert.False(ShortcutRules.CheckKeyboard(new KeyShortcut(modifiers, key)).IsValid);

    [Fact]
    public void Keyboard_rules_explain_the_conflict()
    {
        Assert.Equal("Shift+Tab abre a sobreposição da Steam e do GOG Galaxy · use Ctrl ou Alt junto", ShortcutRules.CheckKeyboard(new KeyShortcut(KeyModifiers.Shift, "Tab")).Error);
        Assert.Equal("F já serve para favoritar no Tako · use Ctrl ou Alt junto", ShortcutRules.CheckKeyboard(new KeyShortcut(KeyModifiers.None, "F")).Error);
    }

    [Fact]
    public void Another_tako_shortcut_cannot_be_reused()
    {
        var tako = new Dictionary<string, string> { ["Ctrl+Alt+Shift+D"] = "voltar as telas para a mesa" };
        var check = ShortcutRules.CheckKeyboard(new KeyShortcut(KeyModifiers.Ctrl | KeyModifiers.Alt | KeyModifiers.Shift, "D"), tako);

        Assert.Equal("Ctrl+Alt+Shift+D já serve para voltar as telas para a mesa no Tako", check.Error);
    }

    [Theory]
    [InlineData(KeyModifiers.Ctrl | KeyModifiers.Shift, "O")] // AMD Adrenalin
    [InlineData(KeyModifiers.Alt, "Z")] // NVIDIA
    [InlineData(KeyModifiers.Alt, "Return")] // tela cheia nos jogos
    [InlineData(KeyModifiers.Ctrl | KeyModifiers.Alt, "Q")] // AltGr no ABNT2
    public void Keyboard_rules_warn_about_known_conflicts(KeyModifiers modifiers, string key)
    {
        var check = ShortcutRules.CheckKeyboard(new KeyShortcut(modifiers, key));
        Assert.True(check.IsValid);
        Assert.NotNull(check.Warning);
    }

    [Fact]
    public void Shortcuts_round_trip_through_settings_text()
    {
        var key = new KeyShortcut(KeyModifiers.Ctrl | KeyModifiers.Alt, "F10");
        Assert.Equal(key, KeyShortcut.Parse(key.Serialize()));
        Assert.Equal(KeyShortcut.Default, KeyShortcut.Parse("lixo"));
        Assert.Equal(KeyShortcut.Default, KeyShortcut.Parse(null));

        var chord = GamepadButtons.View | GamepadButtons.X;
        Assert.Equal(chord, ControlCenterChord.Parse(ControlCenterChord.Serialize(chord)));
        Assert.Equal(ControlCenterChord.Default, ControlCenterChord.Parse("0"));
        Assert.Equal(ControlCenterChord.Default, ControlCenterChord.Parse("64")); // L3 sozinho: botão desconhecido
    }

    [Theory]
    [InlineData("D5", "5")]
    [InlineData("NumPad3", "Num 3")]
    [InlineData("F10", "F10")]
    [InlineData("Escape", "Esc")]
    public void Key_names_are_readable(string key, string expected) => Assert.Equal(expected, KeyShortcut.KeyName(key));

    [Fact]
    public void Row_reveal_moves_only_what_is_needed_and_keeps_padding()
    {
        // Cartões de 300 com 20 de espaço; janela visível de 1000.
        Assert.Equal(0, RowReveal.Offset(itemLeft: 640, itemWidth: 300, contentWidth: 3200, visibleWidth: 1000, padding: 16, current: 0));
        Assert.Equal(276, RowReveal.Offset(itemLeft: 960, itemWidth: 300, contentWidth: 3200, visibleWidth: 1000, padding: 16, current: 0));
        Assert.Equal(304, RowReveal.Offset(itemLeft: 320, itemWidth: 300, contentWidth: 3200, visibleWidth: 1000, padding: 16, current: 600));
        Assert.Equal(0, RowReveal.Offset(itemLeft: 0, itemWidth: 300, contentWidth: 3200, visibleWidth: 1000, padding: 16, current: 600));
    }

    [Fact]
    public void Row_reveal_never_scrolls_past_the_end_or_a_short_row()
    {
        // Último cartão: inteiro, com a folga, sem passar do fim da fileira.
        Assert.Equal(2196, RowReveal.Offset(itemLeft: 2880, itemWidth: 300, contentWidth: 3200, visibleWidth: 1000, padding: 16, current: 0));
        Assert.Equal(2200, RowReveal.Offset(itemLeft: 2880, itemWidth: 300, contentWidth: 3200, visibleWidth: 1000, padding: 40, current: 0));
        // Fileira menor que a janela: nada anda.
        Assert.Equal(0, RowReveal.Offset(itemLeft: 320, itemWidth: 300, contentWidth: 640, visibleWidth: 1000, padding: 16, current: 50));
    }
}
