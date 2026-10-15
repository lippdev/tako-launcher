using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Simulation;
using ConsoleMode.Runtime.Steam;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Library;
using Launcher.Core.Session;
using Launcher.Core.Session.Devices;
using Microsoft.Extensions.Logging.Abstractions;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>MVP-002 parte 2: sinal da Steam no acompanhamento e áudio, Steam e TV no início e no fim da sessão.</summary>
public class SessionDevicesTests
{
    private const string Folder = @"C:\Steam\steamapps\common\Hades";
    private const int AppId = 1145360;
    private static readonly DateTimeOffset Launched = new(2026, 10, 15, 20, 0, 0, TimeSpan.Zero);
    private static readonly AudioOutputChoice Headset = new("sim-headset", "Fone (simulado)");

    private static Game SteamGame(string? folder = Folder, int? appId = AppId, GameTrackingStrategy? tracking = null) => new()
    {
        Id = "steam:1145360",
        Title = "Hades",
        Store = GameStore.Steam,
        Accent = "#000000",
        LaunchTarget = "steam://rungameid/1145360",
        InstallPath = folder,
        SteamAppId = appId,
        Tracking = tracking,
    };

    private static ProcessEntry Proc(int id, string path) => new(id, 1, path);

    // ------------------------------------------------------------------ Plano com o sinal da Steam

    [Fact]
    public void Steam_games_add_the_running_app_id_signal_to_the_folder_plan() =>
        Assert.Equal(new TrackingPlan(GameTrackingStrategy.InstallFolder, Folder, AppId), GameTracking.Plan(SteamGame(), hasProcess: false));

    [Fact]
    public void Steam_games_without_a_known_folder_are_tracked_by_steam_alone()
    {
        Assert.Equal(new TrackingPlan(GameTrackingStrategy.SteamApp, SteamAppId: AppId), GameTracking.Plan(SteamGame(folder: null), hasProcess: false));
        Assert.Equal(TrackingPlan.None, GameTracking.Plan(SteamGame(folder: null, appId: null), hasProcess: false));
        Assert.Equal(TrackingPlan.None, GameTracking.Plan(SteamGame(tracking: GameTrackingStrategy.None), hasProcess: false));
    }

    [Fact]
    public void Non_steam_games_never_get_the_steam_signal()
    {
        var manual = new Game { Id = "manual:1", Title = "Hades", Store = GameStore.Manual, Accent = "#000000", LaunchTarget = @"C:\Jogos\Hades\Hades.exe", SteamAppId = AppId };
        Assert.Null(GameTracking.Plan(manual, hasProcess: true).SteamAppId);
    }

    [Fact]
    public void Steam_only_tracking_needs_the_app_id() =>
        Assert.Throws<ArgumentException>(() => new GameRunTracker(new TrackingPlan(GameTrackingStrategy.SteamApp), Launched));

    // ------------------------------------------------------------------ Acompanhamento com RunningAppID

    [Fact]
    public void Steam_confirms_the_start_even_when_no_game_process_is_visible()
    {
        var tracker = new GameRunTracker(new TrackingPlan(GameTrackingStrategy.InstallFolder, Folder, AppId), Launched);

        Assert.Equal(GameRunStatus.Waiting, tracker.Update([], Launched.AddSeconds(5), steamRunningAppId: 0));
        Assert.Equal(GameRunStatus.Running, tracker.Update([], Launched.AddSeconds(10), steamRunningAppId: AppId)); // exe fora da pasta
        Assert.True(tracker.SteamConfirmed);
    }

    [Fact]
    public void Once_confirmed_steam_decides_the_end_and_leftover_processes_do_not_hold_the_session()
    {
        var tracker = new GameRunTracker(new TrackingPlan(GameTrackingStrategy.InstallFolder, Folder, AppId), Launched);
        ProcessEntry[] crashHandler = [Proc(30, Folder + @"\UnityCrashHandler64.exe")];
        tracker.Update([Proc(20, Folder + @"\Hades.exe"), .. crashHandler], Launched.AddSeconds(10), AppId);
        tracker.Update(crashHandler, Launched.AddMinutes(30), AppId);

        Assert.Equal(GameRunStatus.Running, tracker.Update(crashHandler, Launched.AddMinutes(30).AddSeconds(2), steamRunningAppId: 0)); // tolerância
        Assert.Equal(GameRunStatus.Exited, tracker.Update(crashHandler, Launched.AddMinutes(30).AddSeconds(4), steamRunningAppId: 0));
        Assert.Equal(TimeSpan.FromMinutes(30) - TimeSpan.FromSeconds(10), tracker.PlayTime);
    }

    [Fact]
    public void Unreadable_steam_state_falls_back_to_the_processes()
    {
        var tracker = new GameRunTracker(new TrackingPlan(GameTrackingStrategy.InstallFolder, Folder, AppId), Launched);
        tracker.Update([Proc(20, Folder + @"\Hades.exe")], Launched.AddSeconds(10), AppId);

        Assert.Equal(GameRunStatus.Running, tracker.Update([Proc(20, Folder + @"\Hades.exe")], Launched.AddMinutes(5), steamRunningAppId: null));
        Assert.Equal(GameRunStatus.Exited, tracker.Update([], Launched.AddMinutes(5).AddSeconds(4), steamRunningAppId: null));
    }

    [Fact]
    public void Steam_only_tracking_waits_for_steam_and_times_out_without_it()
    {
        var plan = new TrackingPlan(GameTrackingStrategy.SteamApp, SteamAppId: AppId);
        var tracker = new GameRunTracker(plan, Launched);
        Assert.Equal(GameRunStatus.Running, tracker.Update([], Launched.AddSeconds(20), AppId));
        Assert.Equal(GameRunStatus.Exited, tracker.Update([], Launched.AddSeconds(30), steamRunningAppId: 570)); // outro jogo: não é o nosso

        var never = new GameRunTracker(plan, Launched);
        Assert.Equal(GameRunStatus.TimedOut, never.Update([], Launched.AddMinutes(2), steamRunningAppId: 0));
    }

    // ------------------------------------------------------------------ Áudio, Steam e TV

    private static (GameSessionDevices Devices, SimulatedAudioEndpoints Audio, SimulatedSteamClient Steam, SimulatedTvControl Tv) Create()
    {
        var audio = new SimulatedAudioEndpoints();
        var steam = new SimulatedSteamClient { Running = true, AppId = 0 };
        var tv = new SimulatedTvControl();
        return (new GameSessionDevices(audio, steam, tv, TimeProvider.System, NullLogger<GameSessionDevices>.Instance), audio, steam, tv);
    }

    [Fact]
    public async Task Audio_switches_at_the_start_and_comes_back_at_the_end()
    {
        var (devices, audio, _, _) = Create();

        await devices.BeginSession("steam:1", new SessionDeviceOptions(Headset, SteamToTray: false, TvControlConfig.Default));
        Assert.Equal("sim-headset", audio.Current.DefaultFor(AudioRole.Console));

        var report = devices.EndSession("steam:1", steamGame: true, AppId, gameEnded: true);
        Assert.True(report.AudioRestored);
        Assert.True(audio.Current.IsEquivalentTo(SimulatedAudioEndpoints.Default));
    }

    [Fact]
    public async Task Nothing_happens_when_everything_is_off()
    {
        var (devices, audio, steam, tv) = Create();

        await devices.BeginSession("steam:1", SessionDeviceOptions.Off);
        var report = devices.EndSession("steam:1", steamGame: true, AppId, gameEnded: true);

        Assert.Empty(audio.Calls);
        Assert.Empty(tv.Calls);
        Assert.Equal(0, steam.TrayRequests);
        Assert.Equal(SteamTrayDecision.SettingOff, report.Steam);
    }

    [Fact]
    public async Task Steam_goes_to_the_tray_after_its_game_even_while_the_registry_still_names_it()
    {
        var (devices, _, steam, _) = Create();
        steam.AppId = AppId; // a Steam ainda não zerou o RunningAppID

        await devices.BeginSession("steam:1", new SessionDeviceOptions(null, SteamToTray: true, TvControlConfig.Default));
        var report = devices.EndSession("steam:1", steamGame: true, AppId, gameEnded: true);

        Assert.Equal(SteamTrayDecision.CloseToTray, report.Steam);
        Assert.True(report.SteamSentToTray);
        Assert.Equal(1, steam.TrayRequests);
    }

    [Fact]
    public void Steam_stays_open_when_another_game_runs_or_the_game_never_started()
    {
        var (devices, _, steam, _) = Create();
        steam.AppId = 570;

        _ = devices.BeginSession("steam:1", new SessionDeviceOptions(null, SteamToTray: true, TvControlConfig.Default));
        Assert.Equal(SteamTrayDecision.GameRunning, devices.EndSession("steam:1", steamGame: true, AppId, gameEnded: true).Steam);

        _ = devices.BeginSession("steam:1", new SessionDeviceOptions(null, SteamToTray: true, TvControlConfig.Default));
        Assert.Equal(SteamTrayDecision.NotSteamGame, devices.EndSession("steam:1", steamGame: true, AppId, gameEnded: false).Steam);
        Assert.Equal(0, steam.TrayRequests);
    }

    [Fact]
    public async Task Tv_turns_on_at_the_start_and_goes_to_standby_only_when_asked()
    {
        var (devices, _, _, tv) = Create();
        var config = new TvControlConfig { Enabled = true, Host = "192.168.0.50", StandbyAfterGame = true };

        await devices.BeginSession("steam:1", new SessionDeviceOptions(null, false, config));
        devices.EndSession("steam:1", steamGame: true, AppId, gameEnded: true);
        await WaitForAsync(() => tv.Calls.Count == 2);

        Assert.Equal(["ligar:192.168.0.50", "espera:192.168.0.50"], tv.Calls);
    }

    [Fact]
    public async Task Tv_woken_before_the_game_screen_is_not_turned_on_twice()
    {
        var (devices, audio, _, tv) = Create();
        var config = new TvControlConfig { Enabled = true, Host = "192.168.0.50", StandbyAfterGame = true };

        await devices.WakeTvAsync("steam:1", config);
        await devices.BeginSession("steam:1", new SessionDeviceOptions(Headset, false, config), tvReady: true);
        Assert.Equal(["ligar:192.168.0.50"], tv.Calls);
        Assert.Equal("sim-headset", audio.Current.DefaultFor(AudioRole.Console));

        devices.EndSession("steam:1", steamGame: true, AppId, gameEnded: true);
        await WaitForAsync(() => tv.Calls.Count == 2);
        Assert.Equal("espera:192.168.0.50", tv.Calls[^1]);
    }

    [Fact]
    public async Task A_tv_that_does_not_answer_never_breaks_the_session_and_audio_still_switches()
    {
        var (devices, audio, _, tv) = Create();
        tv.FailWith = "A TV não respondeu.";
        var config = new TvControlConfig { Enabled = true, Host = "192.168.0.50" };

        await devices.BeginSession("steam:1", new SessionDeviceOptions(Headset, false, config));

        Assert.Equal("sim-headset", audio.Current.DefaultFor(AudioRole.Console));
        devices.EndSession("steam:1", steamGame: true, AppId, gameEnded: true);
        Assert.Single(tv.Calls); // sem espera: StandbyAfterGame desligado
    }

    [Fact]
    public async Task Tv_test_requires_an_address_and_reports_failures_for_the_user()
    {
        var (devices, _, _, tv) = Create();

        await Assert.ThrowsAsync<TvControlException>(() => devices.TestTvAsync(new TvControlConfig { Enabled = true }, TestContext.Current.CancellationToken));
        tv.FailWith = "A TV não autorizou este PC.";
        var error = await Assert.ThrowsAsync<TvControlException>(() => devices.TestTvAsync(new TvControlConfig { Host = "tv" }, TestContext.Current.CancellationToken));
        Assert.Equal("A TV não autorizou este PC.", error.Message);
    }

    [Fact]
    public async Task Tv_detection_goes_to_the_detector_and_ignores_empty_addresses()
    {
        var (devices, _, _, tv) = Create();
        tv.DetectedKind = TvKind.Samsung;

        Assert.Null(await devices.DetectTvAsync("  ", TestContext.Current.CancellationToken));
        Assert.Equal(TvKind.Samsung, await devices.DetectTvAsync(" 192.168.0.70 ", TestContext.Current.CancellationToken));
        Assert.Equal(["detectar:192.168.0.70"], tv.Calls);
    }

    private static async Task WaitForAsync(Func<bool> condition)
    {
        for (int i = 0; i < 100 && !condition(); i++)
        {
            await Task.Delay(20, TestContext.Current.CancellationToken);
        }
    }
}
