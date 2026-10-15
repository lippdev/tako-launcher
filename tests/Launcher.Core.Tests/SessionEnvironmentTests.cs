using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.Simulation;
using Launcher.Core.Session.Restore;
using Microsoft.Extensions.Logging.Abstractions;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class SessionEnvironmentTests : IDisposable
{
    private static readonly DateTimeOffset Now = new(2026, 10, 15, 20, 0, 0, TimeSpan.Zero);

    private static readonly MonitorState Tv = new(@"\\?\DISPLAY#TV", "TV", IsPrimary: true, 0, 0, 3840, 2160, new RefreshRate(60000, 1000), DisplayRotation.None);
    private static readonly MonitorState Side = new(@"\\?\DISPLAY#SIDE", "Lateral", IsPrimary: false, 3840, 0, 1920, 1080, RefreshRate.FromHz(144), DisplayRotation.None);

    private readonly string _directory = Path.Combine(Path.GetTempPath(), "tako-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_directory))
        {
            Directory.Delete(_directory, recursive: true);
        }
    }

    // ---------------------------------------------------------------- Diferença

    [Fact]
    public void Same_state_has_no_differences()
    {
        var before = Snapshot(new DisplayTopology([Tv, Side]), SimulatedAudioEndpoints.Default);
        var after = Snapshot(new DisplayTopology([Side, Tv]), SimulatedAudioEndpoints.Default);

        Assert.True(EnvironmentDiff.Between(before, after).IsEmpty);
        Assert.True(before.IsEquivalentTo(after));
    }

    [Fact]
    public void Equivalent_refresh_fractions_are_not_a_change()
    {
        var before = Snapshot(new DisplayTopology([Tv with { RefreshRate = new RefreshRate(60000, 1001) }]), null);
        var after = Snapshot(new DisplayTopology([Tv with { RefreshRate = new RefreshRate(59940, 1000) }]), null);

        Assert.True(EnvironmentDiff.Between(before, after).IsEmpty);
    }

    [Fact]
    public void Resolution_refresh_position_rotation_and_primary_changes_are_reported_per_monitor()
    {
        var before = Snapshot(new DisplayTopology([Tv, Side]), null);
        var after = Snapshot(new DisplayTopology(
        [
            Tv with { Width = 1920, Height = 1080, IsPrimary = false, X = 1920 },
            Side with { IsPrimary = true, X = 0, Rotation = DisplayRotation.Rotate90 },
        ]), null);

        var diff = EnvironmentDiff.Between(before, after);

        Assert.Equal(
            [MonitorChangeKind.Mode, MonitorChangeKind.Position, MonitorChangeKind.Primary],
            diff.Monitors.Where(m => m.MonitorId == Tv.Id).Select(m => m.Kind));
        Assert.Equal(
            [MonitorChangeKind.Position, MonitorChangeKind.Rotation, MonitorChangeKind.Primary],
            diff.Monitors.Where(m => m.MonitorId == Side.Id).Select(m => m.Kind));
    }

    [Fact]
    public void Monitor_turned_off_or_on_changes_the_topology()
    {
        var before = Snapshot(new DisplayTopology([Tv, Side]), null);
        var tvOnly = Snapshot(new DisplayTopology([Tv]), null);

        Assert.Equal(MonitorChangeKind.Missing, Assert.Single(EnvironmentDiff.Between(before, tvOnly).Monitors).Kind);
        Assert.Equal(MonitorChangeKind.Added, Assert.Single(EnvironmentDiff.Between(tvOnly, before).Monitors).Kind);
    }

    [Fact]
    public void Missing_part_in_either_snapshot_is_never_a_difference()
    {
        var full = Snapshot(new DisplayTopology([Tv]), SimulatedAudioEndpoints.Default);
        var noDisplay = Snapshot(null, SimulatedAudioEndpoints.Default with { Levels = [] });
        var noAudio = Snapshot(new DisplayTopology([Tv with { Width = 1280 }]), null);

        Assert.False(EnvironmentDiff.Between(full, noDisplay).HasDisplayChanges);
        Assert.False(EnvironmentDiff.Between(full, noAudio).HasAudioChanges);
    }

    [Fact]
    public void Audio_default_device_volume_and_mute_changes_are_reported()
    {
        var before = Snapshot(null, SimulatedAudioEndpoints.Default);
        var after = Snapshot(null, SimulatedAudioEndpoints.Default with
        {
            Defaults = [new(AudioRole.Console, "sim-headset"), new(AudioRole.Multimedia, "sim-headset"), new(AudioRole.Communications, "sim-speakers")],
            Levels = [new("sim-speakers", 0.2f, Muted: true), new("sim-headset", 0.5f, Muted: false)],
        });

        var diff = EnvironmentDiff.Between(before, after);

        Assert.Equal([AudioRole.Console, AudioRole.Multimedia], diff.AudioDefaults.Select(d => d.Role));
        var level = Assert.Single(diff.AudioLevels);
        Assert.Equal("sim-speakers", level.DeviceId);
        Assert.True(level.VolumeChanged);
        Assert.True(level.MuteChanged);
    }

    [Fact]
    public void Volume_within_one_percent_is_not_a_change()
    {
        var before = Snapshot(null, SimulatedAudioEndpoints.Default);
        var after = Snapshot(null, SimulatedAudioEndpoints.Default with { Levels = [new("sim-speakers", 0.703f, false), new("sim-headset", 0.5f, false)] });

        Assert.True(EnvironmentDiff.Between(before, after).IsEmpty);
    }

    // ---------------------------------------------------------------- Plano

    [Fact]
    public void Plan_restores_display_in_one_step_with_the_changed_monitors()
    {
        var before = Snapshot(new DisplayTopology([Tv, Side]), null);
        var after = Snapshot(new DisplayTopology([Tv with { Width = 1920, Height = 1080 }, Side]), null);

        var plan = RestorePlanner.Plan(before, after, canSetDefaultDevice: true);

        var step = Assert.IsType<RestoreDisplayStep>(Assert.Single(plan.Steps));
        Assert.Equal([Tv.Id], step.ChangedMonitorIds);
        Assert.Same(before.Display, step.Target);
    }

    [Fact]
    public void Plan_restores_only_what_changed_and_orders_display_before_audio()
    {
        var before = Snapshot(new DisplayTopology([Tv]), SimulatedAudioEndpoints.Default);
        var after = Snapshot(new DisplayTopology([Tv with { RefreshRate = RefreshRate.FromHz(120) }]), SimulatedAudioEndpoints.Default with
        {
            Defaults = [new(AudioRole.Console, "sim-headset"), new(AudioRole.Multimedia, "sim-speakers"), new(AudioRole.Communications, "sim-speakers")],
        });

        var plan = RestorePlanner.Plan(before, after, canSetDefaultDevice: true);

        Assert.Collection(
            plan.Steps,
            s => Assert.IsType<RestoreDisplayStep>(s),
            s => Assert.Equal(new SetDefaultAudioStep(AudioRole.Console, "sim-speakers", "Alto-falantes (simulado)"), s));
        Assert.Empty(plan.Skipped);
    }

    [Fact]
    public void Unplugged_device_is_skipped_with_a_reason()
    {
        var before = Snapshot(null, SimulatedAudioEndpoints.Default);
        var after = Snapshot(null, new AudioState(
            [new AudioDevice("sim-headset", "Fone (simulado)")],
            [new(AudioRole.Console, "sim-headset"), new(AudioRole.Multimedia, "sim-headset"), new(AudioRole.Communications, "sim-headset")],
            [new("sim-headset", 0.5f, false)]));

        var plan = RestorePlanner.Plan(before, after, canSetDefaultDevice: true);

        Assert.True(plan.IsEmpty);
        Assert.Equal(4, plan.Skipped.Count); // três papéis + volume dos alto-falantes
        Assert.All(plan.Skipped, s => Assert.Contains("não está conectado", s.Reason, StringComparison.Ordinal));
    }

    [Fact]
    public void Without_default_device_support_only_volume_and_mute_are_restored()
    {
        var before = Snapshot(null, SimulatedAudioEndpoints.Default);
        var after = Snapshot(null, SimulatedAudioEndpoints.Default with
        {
            Defaults = [new(AudioRole.Console, "sim-headset"), new(AudioRole.Multimedia, "sim-speakers"), new(AudioRole.Communications, "sim-speakers")],
            Levels = [new("sim-speakers", 0.7f, Muted: true), new("sim-headset", 0.5f, false)],
        });

        var plan = RestorePlanner.Plan(before, after, canSetDefaultDevice: false);

        Assert.Equal(new SetMuteStep("sim-speakers", "Alto-falantes (simulado)", false), Assert.Single(plan.Steps));
        Assert.Contains("não está disponível", Assert.Single(plan.Skipped).Reason, StringComparison.Ordinal);
    }

    [Fact]
    public void Unmuting_sets_the_volume_first_and_muting_comes_before_the_volume()
    {
        var loudMuted = Snapshot(null, SimulatedAudioEndpoints.Default with { Levels = [new("sim-speakers", 1f, Muted: true)] });
        var quiet = Snapshot(null, SimulatedAudioEndpoints.Default with { Levels = [new("sim-speakers", 0.1f, Muted: false)] });

        var unmute = RestorePlanner.Plan(quiet, loudMuted, canSetDefaultDevice: true);
        var mute = RestorePlanner.Plan(loudMuted, quiet, canSetDefaultDevice: true);

        Assert.Collection(unmute.Steps, s => Assert.IsType<SetVolumeStep>(s), s => Assert.False(Assert.IsType<SetMuteStep>(s).Muted));
        Assert.Collection(mute.Steps, s => Assert.True(Assert.IsType<SetMuteStep>(s).Muted), s => Assert.IsType<SetVolumeStep>(s));
    }

    [Fact]
    public void Recorded_state_without_primary_monitor_is_not_applied()
    {
        var bogus = Snapshot(new DisplayTopology([Side]), null);
        var now = Snapshot(new DisplayTopology([Tv]), null);

        var plan = RestorePlanner.Plan(bogus, now, canSetDefaultDevice: true);

        Assert.True(plan.IsEmpty);
        Assert.Single(plan.Skipped);
    }

    // ---------------------------------------------------------------- JSON

    [Fact]
    public void Snapshot_round_trips_through_json()
    {
        var snapshot = Snapshot(new DisplayTopology([Tv, Side], PlatformData: "AAEC"), SimulatedAudioEndpoints.Default);

        string json = SessionEnvironmentSnapshotJson.Serialize(snapshot);
        var back = SessionEnvironmentSnapshotJson.TryDeserialize(json);

        Assert.NotNull(back);
        Assert.True(back.IsEquivalentTo(snapshot));
        Assert.Equal("AAEC", back.Display!.PlatformData);
        Assert.Equal(new RefreshRate(60000, 1000), back.Display.Find(Tv.Id)!.RefreshRate);
        Assert.Contains("\"rotation\": \"None\"", json, StringComparison.Ordinal);
    }

    [Theory]
    [InlineData("")]
    [InlineData("{")]
    [InlineData("{\"schemaVersion\": 99, \"capturedAt\": \"2026-10-15T20:00:00Z\"}")]
    [InlineData("{\"schemaVersion\": 1, \"capturedAt\": \"2026-10-15T20:00:00Z\", \"display\": {\"monitors\": [{\"name\": \"sem id\"}]}}")]
    [InlineData("{\"schemaVersion\": 1, \"capturedAt\": \"2026-10-15T20:00:00Z\", \"audio\": {\"devices\": []}}")]
    public void Invalid_or_unknown_json_is_rejected(string json) => Assert.Null(SessionEnvironmentSnapshotJson.TryDeserialize(json));

    // ---------------------------------------------------------------- Sessão

    [Fact]
    public void Session_captures_before_the_game_and_restores_what_the_game_changed()
    {
        var (guard, display, audio, store) = Guard();
        guard.BeginSession("jogo-1", enabled: true);
        Assert.True(File.Exists(store.FilePath));

        // O jogo troca a resolução e manda o som para o fone.
        display.Current = new DisplayTopology([display.Current.Monitors[0] with { Width = 1280, Height = 720 }]);
        audio.SetDefaultDevice("sim-headset", AudioRole.Console);
        audio.Calls.Clear();

        var report = guard.EndSession();

        Assert.True(report.Succeeded);
        Assert.Equal(2, report.Applied.Count);
        Assert.Equal(1, display.AppliedCount);
        Assert.Equal(["default Console sim-speakers"], audio.Calls);
        Assert.True(display.Current.IsEquivalentTo(SimulatedDisplayConfiguration.Default));
        Assert.False(File.Exists(store.FilePath));
        Assert.Null(guard.Active);
    }

    [Fact]
    public void Nothing_changed_means_no_call_to_the_platform()
    {
        var (guard, display, audio, store) = Guard();
        guard.BeginSession("jogo-1", enabled: true);

        var report = guard.EndSession();

        Assert.True(report.NothingToDo);
        Assert.Equal(0, display.AppliedCount);
        Assert.Empty(audio.Calls);
        Assert.False(File.Exists(store.FilePath));
    }

    [Fact]
    public void Disabled_option_records_nothing_and_restores_nothing()
    {
        var (guard, display, _, store) = Guard();

        Assert.Null(guard.BeginSession("jogo-1", enabled: false));
        display.Current = new DisplayTopology([display.Current.Monitors[0] with { Width = 1280 }]);

        Assert.True(guard.EndSession().NothingToDo);
        Assert.Equal(0, display.AppliedCount);
        Assert.False(File.Exists(store.FilePath));
    }

    [Fact]
    public void Display_failure_does_not_stop_audio_restore_and_is_reported()
    {
        var (guard, display, audio, _) = Guard();
        guard.BeginSession("jogo-1", enabled: true);
        display.Current = new DisplayTopology([display.Current.Monitors[0] with { Width = 1280 }]);
        audio.SetMute("sim-speakers", true);
        var original = display.Current;

        // Leitura funciona, aplicação falha: simula trocando o serviço depois da leitura.
        var failing = new FailingApplyDisplay(display);
        var guardWithFailure = new SessionEnvironmentGuard(failing, audio, new PendingEnvironmentStore(Path.Combine(_directory, "b.json")), TimeProvider.System, NullLogger<SessionEnvironmentGuard>.Instance);
        var report = guardWithFailure.RestoreFrom(guard.Active!);

        Assert.False(report.Succeeded);
        Assert.Single(report.Failed);
        Assert.Contains(report.Applied, s => s is SetMuteStep { Muted: false });
        Assert.True(report.Remaining.HasDisplayChanges);
        Assert.False(report.Remaining.HasAudioChanges);
        Assert.Same(original, display.Current);
    }

    [Fact]
    public void Capture_failure_in_one_part_keeps_the_other()
    {
        var (guard, display, _, _) = Guard();
        display.FailNext = true;

        var snapshot = guard.BeginSession("jogo-1", enabled: true);

        Assert.NotNull(snapshot);
        Assert.Null(snapshot.Display);
        Assert.NotNull(snapshot.Audio);
    }

    [Fact]
    public void Capture_failure_in_both_parts_records_nothing()
    {
        var (guard, display, audio, store) = Guard();
        display.FailNext = true;
        audio.FailNext = true;

        Assert.Null(guard.BeginSession("jogo-1", enabled: true));
        Assert.False(File.Exists(store.FilePath));
    }

    [Fact]
    public void Discard_forgets_the_session()
    {
        var (guard, display, _, store) = Guard();
        guard.BeginSession("jogo-1", enabled: true);
        display.Current = new DisplayTopology([display.Current.Monitors[0] with { Width = 1280 }]);

        guard.Discard();

        Assert.True(guard.EndSession().NothingToDo);
        Assert.False(File.Exists(store.FilePath));
    }

    [Fact]
    public void Snapshot_left_by_a_crash_is_offered_on_the_next_start_and_restored()
    {
        var (guard, display, audio, store) = Guard();
        guard.BeginSession("jogo-1", enabled: true);
        display.Current = new DisplayTopology([display.Current.Monitors[0] with { Width = 1280 }]);
        // O Tako cai: nova instância (próxima abertura) com o mesmo arquivo.
        var next = new SessionEnvironmentGuard(display, audio, new PendingEnvironmentStore(store.FilePath), TimeProvider.System, NullLogger<SessionEnvironmentGuard>.Instance);

        var pending = next.PendingFromPreviousRun();

        Assert.NotNull(pending);
        Assert.Equal("jogo-1", pending.Context);
        Assert.True(next.RestoreFrom(pending).Succeeded);
        Assert.True(display.Current.IsEquivalentTo(SimulatedDisplayConfiguration.Default));
        Assert.False(File.Exists(store.FilePath));
    }

    [Fact]
    public void Snapshot_left_by_a_crash_without_differences_is_dropped_silently()
    {
        var (guard, _, _, store) = Guard();
        guard.BeginSession("jogo-1", enabled: true);

        Assert.Null(guard.PendingFromPreviousRun());
        Assert.False(File.Exists(store.FilePath));
    }

    [Fact]
    public void Unreadable_pending_file_is_dropped()
    {
        var (guard, _, _, store) = Guard();
        Directory.CreateDirectory(_directory);
        File.WriteAllText(store.FilePath, "{ quebrado");

        Assert.Null(guard.PendingFromPreviousRun());
        Assert.False(File.Exists(store.FilePath));
    }

    private static SessionEnvironmentSnapshot Snapshot(DisplayTopology? display, AudioState? audio) =>
        SessionEnvironmentSnapshot.Create(Now, "teste", display, audio);

    private (SessionEnvironmentGuard Guard, SimulatedDisplayConfiguration Display, SimulatedAudioEndpoints Audio, PendingEnvironmentStore Store) Guard()
    {
        var display = new SimulatedDisplayConfiguration();
        var audio = new SimulatedAudioEndpoints();
        var store = new PendingEnvironmentStore(Path.Combine(_directory, "session-environment.json"));
        var guard = new SessionEnvironmentGuard(display, audio, store, new FixedTimeProvider(Now), NullLogger<SessionEnvironmentGuard>.Instance);
        return (guard, display, audio, store);
    }

    private sealed class FailingApplyDisplay(SimulatedDisplayConfiguration inner) : IDisplayConfigurationService
    {
        public DisplayTopology Capture() => inner.Capture();

        public void Apply(DisplayTopology target, IReadOnlyCollection<string> changedMonitorIds) =>
            throw new RuntimeEnvironmentException("SetDisplayConfig falhou (simulado)");
    }
}
