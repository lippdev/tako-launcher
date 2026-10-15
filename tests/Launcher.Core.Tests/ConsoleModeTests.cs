using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.Simulation;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Session;
using Launcher.Core.Session.Devices;
using Launcher.Core.Session.GameScreen;
using Launcher.Core.Session.Restore;
using Microsoft.Extensions.Logging.Abstractions;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Modo console ao abrir (MVP-002/012/036): quando focar e restaurar em cada modo, e a recuperação depois de uma queda.</summary>
public sealed class ConsoleModeTests : IDisposable
{
    private const string Desk = "SIM\\DESK";
    private const string Tv = "SIM\\TV";
    private static readonly AudioOutputChoice Headset = new("sim-headset", "Fone (simulado)");
    private static readonly TvControlConfig TvOn = new() { Enabled = true, Host = "192.168.0.50", StandbyAfterGame = true };

    private readonly string _directory = Path.Combine(Path.GetTempPath(), "tako-tests-" + Guid.NewGuid().ToString("N"));
    private readonly SimulatedDisplayDesk _desk = new();
    private readonly SimulatedScreenCurtain _curtain = new();
    private readonly SimulatedAudioEndpoints _audio = new();
    private readonly SimulatedTvControl _tv = new();

    private string MarkerFile => Path.Combine(_directory, "console-mode.json");

    private string EnvironmentFile => Path.Combine(_directory, "console-mode-environment.json");

    public void Dispose()
    {
        if (Directory.Exists(_directory))
        {
            Directory.Delete(_directory, recursive: true);
        }
    }

    // ------------------------------------------------------------------ Decisões

    [Theory]
    [InlineData(false, true, false, ConsoleModeStart.Off)]
    [InlineData(true, false, false, ConsoleModeStart.NoGameScreen)]
    [InlineData(true, true, true, ConsoleModeStart.SafeMode)]
    [InlineData(true, true, false, ConsoleModeStart.Enter)]
    [InlineData(false, true, true, ConsoleModeStart.Off)]
    public void Startup_enters_only_with_the_option_a_game_screen_and_outside_safe_mode(bool option, bool hasScreen, bool safeMode, ConsoleModeStart expected) =>
        Assert.Equal(expected, ConsoleModePolicy.AtStartup(option, hasScreen ? Settings() : null, safeMode));

    [Fact]
    public void Outside_console_mode_each_game_focuses_and_restores_the_screen_with_tv_and_audio()
    {
        var devices = new SessionDeviceOptions(Headset, SteamToTray: true, TvOn);

        var steps = ConsoleModePolicy.ForGame(consoleModeActive: false, Effective(), devices, Settings(), Headset);

        Assert.True(steps.WakeTv);
        Assert.True(steps.FocusScreen);
        Assert.Same(devices, steps.Devices);
        Assert.Null(steps.Note);
    }

    [Fact]
    public void Outside_console_mode_a_game_without_the_game_screen_wakes_the_tv_in_background()
    {
        var steps = ConsoleModePolicy.ForGame(false, Effective(new GameSettingsOverride { UseGameScreen = false }), new SessionDeviceOptions(null, false, TvOn), Settings(), null);

        Assert.False(steps.WakeTv);
        Assert.False(steps.FocusScreen);
        Assert.True(steps.Devices.Tv.IsActive);
    }

    [Fact]
    public void Inside_console_mode_games_never_focus_restore_switch_audio_or_touch_the_tv_again()
    {
        var steps = ConsoleModePolicy.ForGame(consoleModeActive: true, Effective(), new SessionDeviceOptions(Headset, SteamToTray: true, TvOn), Settings(), Headset);

        Assert.False(steps.WakeTv);
        Assert.False(steps.FocusScreen);
        Assert.Null(steps.Devices.GameAudio); // o modo console já trocou para esta saída
        Assert.False(steps.Devices.Tv.IsActive); // a TV só vai para a espera ao sair do modo console
        Assert.True(steps.Devices.SteamToTray); // a Steam na bandeja continua sendo do fim de cada jogo
        Assert.Null(steps.Note);
    }

    [Fact]
    public void Inside_console_mode_a_game_that_turns_the_game_screen_off_still_opens_on_it_with_a_note()
    {
        var steps = ConsoleModePolicy.ForGame(true, Effective(new GameSettingsOverride { UseGameScreen = false }), new SessionDeviceOptions(null, false, TvOn), Settings(), null);

        Assert.False(steps.FocusScreen);
        Assert.False(steps.WakeTv);
        Assert.Contains("Voltar ao PC", steps.Note, StringComparison.Ordinal);
    }

    [Fact]
    public void Inside_console_mode_a_game_resolution_or_hdr_is_not_applied_and_says_so()
    {
        var own = Effective(new GameSettingsOverride { ScreenMode = "1920x1080@60", Hdr = true });

        var steps = ConsoleModePolicy.ForGame(true, own, new SessionDeviceOptions(null, false, TvControlConfig.Default), Settings(), null);

        Assert.False(steps.FocusScreen);
        Assert.Contains("resolução e o HDR", steps.Note, StringComparison.Ordinal);
    }

    [Fact]
    public void Inside_console_mode_a_game_with_its_own_audio_output_switches_and_comes_back()
    {
        var speakers = new AudioOutputChoice("sim-speakers", "Alto-falantes (simulado)");
        var effective = Effective(new GameSettingsOverride { AudioDeviceId = speakers.DeviceId, AudioDeviceName = speakers.Name });

        var steps = ConsoleModePolicy.ForGame(true, effective, new SessionDeviceOptions(effective.GameAudio, false, TvOn), Settings(), Headset);

        Assert.Equal(speakers, steps.Devices.GameAudio);
        Assert.False(steps.Devices.Tv.IsActive);
    }

    // ------------------------------------------------------------------ Entrar e sair

    [Fact]
    public async Task Enter_wakes_the_tv_focuses_the_screen_switches_audio_and_leaves_a_way_back_on_disk()
    {
        using var console = Console();

        var result = await console.EnterAsync(Settings(hdr: true), new SessionDeviceOptions(Headset, false, TvOn), confirmedSetup: null, TestContext.Current.CancellationToken);

        Assert.Equal(GameScreenOutcome.Applied, result.Outcome);
        Assert.True(result.NeedsConfirmation);
        Assert.True(console.IsActive);
        Assert.Equal(["ligar:192.168.0.50"], _tv.Calls);
        var during = _desk.ListDisplays();
        Assert.True(during.Single(d => d.Id == Tv) is { IsActive: true, IsPrimary: true, Hdr: HdrStatus.On });
        Assert.False(during.Single(d => d.Id == Desk).IsActive);
        await WaitAsync(() => _audio.Current.DefaultFor(AudioRole.Console) == "sim-headset");
        Assert.True(File.Exists(MarkerFile));
        Assert.True(File.Exists(EnvironmentFile));
        Assert.Equal(Tv, new ConsoleModeMarkerStore(MarkerFile).Read()?.HdrDisplayId);
    }

    [Fact]
    public async Task Exit_puts_the_desk_audio_and_tv_back_and_clears_the_disk()
    {
        var before = _desk.Capture();
        using var console = Console();
        await console.EnterAsync(Settings(hdr: true), new SessionDeviceOptions(Headset, false, TvOn), null, TestContext.Current.CancellationToken);
        await WaitAsync(() => _audio.Current.DefaultFor(AudioRole.Console) == "sim-headset");

        var report = console.Exit(standbyTv: true);

        Assert.True(report.WasActive);
        Assert.True(report.Succeeded);
        Assert.False(console.IsActive);
        Assert.True(_desk.Capture().IsEquivalentTo(before));
        Assert.Contains($"hdr {Tv} off", _desk.Calls);
        Assert.Equal("sim-speakers", _audio.Current.DefaultFor(AudioRole.Console));
        await WaitAsync(() => _tv.Calls.Contains("espera:192.168.0.50"));
        Assert.False(File.Exists(MarkerFile));
        Assert.False(File.Exists(EnvironmentFile));
    }

    [Fact]
    public async Task Exit_without_standby_keeps_the_tv_on_and_a_second_exit_does_nothing()
    {
        using var console = Console();
        await console.EnterAsync(Settings(), new SessionDeviceOptions(null, false, TvOn), Settings().SetupKey, TestContext.Current.CancellationToken);

        Assert.True(console.Exit(standbyTv: false).WasActive);
        var again = console.Exit(standbyTv: true);

        Assert.False(again.WasActive);
        Assert.True(again.Succeeded);
        await Task.Delay(100, TestContext.Current.CancellationToken);
        Assert.DoesNotContain(_tv.Calls, c => c.StartsWith("espera", StringComparison.Ordinal));
    }

    [Fact]
    public async Task Confirmed_setup_is_not_asked_again_on_launch()
    {
        using var console = Console();

        var result = await console.EnterAsync(Settings(), SessionDeviceOptions.Off, Settings().SetupKey, TestContext.Current.CancellationToken);

        Assert.False(result.NeedsConfirmation);
    }

    [Fact]
    public async Task Missing_screen_does_not_enter_and_leaves_nothing_behind()
    {
        using var console = Console();

        var result = await console.EnterAsync(new GameScreenSettings("SIM\\GONE", HideOthersMode.Disconnect, null, false), SessionDeviceOptions.Off, null, TestContext.Current.CancellationToken);

        Assert.Equal(GameScreenOutcome.Skipped, result.Outcome);
        Assert.False(console.IsActive);
        Assert.False(File.Exists(MarkerFile));
        Assert.False(File.Exists(EnvironmentFile));
        Assert.False(console.HasPendingRecovery);
    }

    [Fact]
    public async Task Tv_that_never_wakes_reverts_and_never_hides_the_desk()
    {
        _desk.NeverWake.Add(Tv);
        using var console = Console();

        var result = await console.EnterAsync(Settings(), new SessionDeviceOptions(Headset, false, TvOn), null, TestContext.Current.CancellationToken);

        Assert.Equal(GameScreenOutcome.Reverted, result.Outcome);
        Assert.False(console.IsActive);
        Assert.True(_desk.ListDisplays().Single(d => d.Id == Desk).IsActive);
        Assert.Equal("sim-speakers", _audio.Current.DefaultFor(AudioRole.Console)); // o áudio só troca com a tela em foco
        Assert.False(File.Exists(MarkerFile));
    }

    [Fact]
    public async Task Entering_twice_keeps_one_console_mode()
    {
        using var console = Console();
        await console.EnterAsync(Settings(), SessionDeviceOptions.Off, null, TestContext.Current.CancellationToken);

        var second = await console.EnterAsync(Settings(), SessionDeviceOptions.Off, null, TestContext.Current.CancellationToken);

        Assert.Equal(GameScreenOutcome.Skipped, second.Outcome);
        Assert.True(console.IsActive);
    }

    // ------------------------------------------------------------------ Recuperação

    [Fact]
    public void Nothing_pending_does_nothing()
    {
        using var console = Console();

        Assert.False(console.HasPendingRecovery);
        Assert.Equal(ConsoleModeRecoveryOutcome.NothingPending, console.RecoverFromPreviousRun());
        Assert.Empty(_desk.Calls);
    }

    [Fact]
    public async Task After_a_crash_in_console_mode_the_next_opening_puts_the_desk_back_without_asking()
    {
        var before = _desk.Capture();
        var crashed = Console();
        await crashed.EnterAsync(Settings(hdr: true), new SessionDeviceOptions(Headset, false, TvOn), null, TestContext.Current.CancellationToken);
        await WaitAsync(() => _audio.Current.DefaultFor(AudioRole.Console) == "sim-headset");
        _curtain.Close(); // a cortina preta morre com o processo
        _desk.Calls.Clear();

        // Próxima abertura: tudo novo em memória, só os arquivos sobraram.
        using var next = Console();
        Assert.True(next.HasPendingRecovery);
        var outcome = next.RecoverFromPreviousRun();

        Assert.Equal(ConsoleModeRecoveryOutcome.Restored, outcome);
        Assert.Equal($"hdr {Tv} off", _desk.Calls[0]); // HDR antes: depois a TV pode nem estar ligada
        Assert.True(_desk.Capture().IsEquivalentTo(before));
        Assert.Equal("sim-speakers", _audio.Current.DefaultFor(AudioRole.Console));
        Assert.False(File.Exists(MarkerFile));
        Assert.False(File.Exists(EnvironmentFile));
        Assert.False(next.HasPendingRecovery);
        crashed.Dispose();
    }

    [Fact]
    public async Task Recovery_with_the_desk_already_back_only_clears_the_disk()
    {
        var crashed = Console();
        await crashed.EnterAsync(Settings(), SessionDeviceOptions.Off, null, TestContext.Current.CancellationToken);
        Assert.True(crashed.Exit(standbyTv: false).WasActive); // o usuário arrumou as telas, mas os arquivos ficaram
        new ConsoleModeMarkerStore(MarkerFile).TryWrite(new ConsoleModeMarker(DateTimeOffset.UnixEpoch, null));

        using var next = Console();
        var outcome = next.RecoverFromPreviousRun();

        Assert.Equal(ConsoleModeRecoveryOutcome.AlreadyAtDesk, outcome);
        Assert.False(File.Exists(MarkerFile));
        crashed.Dispose();
    }

    [Fact]
    public void Unreadable_marker_still_triggers_recovery()
    {
        Directory.CreateDirectory(_directory);
        File.WriteAllText(MarkerFile, "{ truncado");
        using var console = Console();

        Assert.True(console.HasPendingRecovery);
        Assert.Equal(ConsoleModeRecoveryOutcome.AlreadyAtDesk, console.RecoverFromPreviousRun());
        Assert.False(File.Exists(MarkerFile));
    }

    [Fact]
    public async Task Recovery_is_skipped_while_console_mode_is_active_in_this_run()
    {
        using var console = Console();
        await console.EnterAsync(Settings(), SessionDeviceOptions.Off, null, TestContext.Current.CancellationToken);

        Assert.False(console.HasPendingRecovery);
        Assert.Equal(ConsoleModeRecoveryOutcome.NothingPending, console.RecoverFromPreviousRun());
        Assert.True(console.IsActive);
    }

    // ------------------------------------------------------------------ Apoio

    private static GameScreenSettings Settings(bool hdr = false) => new(Tv, HideOthersMode.Disconnect, null, hdr);

    /// <summary>O efetivo de um jogo pelo resolvedor (UX-007), com a Tela de jogo ligada nas Configurações do Tako.</summary>
    private static EffectiveSessionSettings Effective(GameSettingsOverride? game = null) =>
        GameSettingsResolver.Resolve(new GlobalSessionSettings(true, Tv, HideOthersMode.Disconnect, null, false, Headset, 0, false), game);

    private ConsoleModeSession Console()
    {
        var store = new PendingEnvironmentStore(EnvironmentFile);
        var environment = new SessionEnvironmentGuard(_desk, _audio, store, TimeProvider.System, NullLogger<SessionEnvironmentGuard>.Instance);
        var markers = new ConsoleModeMarkerStore(MarkerFile);
        var recovery = new ConsoleModeRecovery(_desk, environment, store, markers, NullLogger<ConsoleModeRecovery>.Instance);
        var screen = new GameScreenSession(_desk, _desk, _curtain, NullLogger<GameScreenSession>.Instance, _ => { });
        var devices = new GameSessionDevices(_audio, new SimulatedSteamClient(), _tv, TimeProvider.System, NullLogger<GameSessionDevices>.Instance);
        return new ConsoleModeSession(screen, devices, environment, markers, recovery, TimeProvider.System, NullLogger<ConsoleModeSession>.Instance);
    }

    private static async Task WaitAsync(Func<bool> condition)
    {
        for (int i = 0; i < 100 && !condition(); i++)
        {
            await Task.Delay(20, TestContext.Current.CancellationToken);
        }

        Assert.True(condition());
    }
}
