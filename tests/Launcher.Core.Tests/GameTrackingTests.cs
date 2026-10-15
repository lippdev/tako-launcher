using Launcher.Core.Library;
using Launcher.Core.Session;
using Xunit;

namespace Launcher.Core.Tests;

public class GameTrackingTests
{
    private const string Folder = @"C:\Steam\steamapps\common\Hades";
    private static readonly DateTimeOffset Launched = new(2026, 10, 15, 20, 0, 0, TimeSpan.Zero);

    private static Game SteamGame(GameTrackingStrategy? tracking = null) => new()
    {
        Id = "steam:1145360",
        Title = "Hades",
        Store = GameStore.Steam,
        Accent = "#000000",
        LaunchTarget = "steam://rungameid/1145360",
        InstallPath = Folder,
        Tracking = tracking,
    };

    // Montado à mão: caminhos do Windows não são caminhos de arquivo no CI de outros sistemas.
    private static Game ManualGame(string target, GameTrackingStrategy? tracking = null) => new()
    {
        Id = "manual:1",
        Title = "Indie",
        Store = GameStore.Manual,
        Accent = "#000000",
        LaunchTarget = target,
        InstallPath = target.Contains("://", StringComparison.Ordinal) ? null : target[..target.LastIndexOf('\\')],
        Tracking = tracking,
    };

    // ------------------------------------------------------------------ Escolha da estratégia

    [Fact]
    public void Steam_games_are_tracked_by_install_folder() =>
        Assert.Equal(new TrackingPlan(GameTrackingStrategy.InstallFolder, Folder), GameTracking.Plan(SteamGame(), hasProcess: false));

    [Fact]
    public void Manual_executables_are_tracked_by_the_started_process() =>
        Assert.Equal(GameTrackingStrategy.Process, GameTracking.Plan(ManualGame(@"C:\Jogos\Indie\jogo.exe"), hasProcess: true).Strategy);

    [Fact]
    public void Manual_executable_without_process_falls_back_to_its_folder() =>
        Assert.Equal(new TrackingPlan(GameTrackingStrategy.InstallFolder, @"C:\Jogos\Indie"), GameTracking.Plan(ManualGame(@"C:\Jogos\Indie\jogo.exe"), hasProcess: false));

    [Fact]
    public void Shortcuts_and_uris_without_process_are_not_tracked()
    {
        Assert.Equal(TrackingPlan.None, GameTracking.Plan(ManualGame(@"C:\Users\eu\Desktop\Jogo.url"), hasProcess: false));
        Assert.Equal(TrackingPlan.None, GameTracking.Plan(ManualGame("steam://rungameid/570"), hasProcess: false));
    }

    [Fact]
    public void The_user_choice_wins_when_it_is_possible()
    {
        Assert.Equal(GameTrackingStrategy.InstallFolder, GameTracking.Plan(ManualGame(@"C:\Jogos\Indie\launcher.exe", GameTrackingStrategy.InstallFolder), hasProcess: true).Strategy);
        Assert.Equal(GameTrackingStrategy.Process, GameTracking.Plan(SteamGame(GameTrackingStrategy.Process), hasProcess: true).Strategy);
        Assert.Equal(TrackingPlan.None, GameTracking.Plan(SteamGame(GameTrackingStrategy.None), hasProcess: false));
        // Processo escolhido, mas a Steam não devolveu processo: volta ao automático (pasta).
        Assert.Equal(GameTrackingStrategy.InstallFolder, GameTracking.Plan(SteamGame(GameTrackingStrategy.Process), hasProcess: false).Strategy);
    }

    [Theory]
    [InlineData(@"C:\Steam\steamapps\common\Hades\x64\Hades.exe", true)]
    [InlineData(@"c:\steam\STEAMAPPS\common\hades\Hades.exe", true)]
    [InlineData("C:/Steam/steamapps/common/Hades/Hades.exe", false)] // pasta com \ e caminho com /: prefixo difere
    [InlineData(@"C:\Steam\steamapps\common\Hades II\Hades2.exe", false)]
    [InlineData(@"C:\Steam\steamapps\common\Hades", false)]
    [InlineData(null, false)]
    public void Folder_match_requires_a_path_inside_the_folder(string? path, bool expected) =>
        Assert.Equal(expected, GameTracking.IsInside(path, Folder + @"\"));

    // ------------------------------------------------------------------ Acompanhamento por pasta

    private static ProcessEntry Proc(int id, string? path, int parent = 4) => new(id, parent, path);

    [Fact]
    public void Folder_tracking_waits_then_runs_then_exits_after_the_grace_period()
    {
        var tracker = new GameRunTracker(new TrackingPlan(GameTrackingStrategy.InstallFolder, Folder), Launched);
        ProcessEntry[] steamOnly = [Proc(10, @"C:\Steam\steam.exe"), Proc(11, null)]; // 11: acesso negado
        ProcessEntry[] playing = [.. steamOnly, Proc(20, Folder + @"\Hades.exe")];

        Assert.Equal(GameRunStatus.Waiting, tracker.Update(steamOnly, Launched.AddSeconds(30)));
        Assert.Equal(GameRunStatus.Running, tracker.Update(playing, Launched.AddSeconds(40)));
        Assert.Equal([20], tracker.GameProcessIds);
        Assert.Equal([Folder + @"\Hades.exe"], tracker.GameExecutablePaths); // perfil do limite de FPS no driver (ADR-0018)
        Assert.Equal(GameRunStatus.Running, tracker.Update(playing, Launched.AddMinutes(30)));
        Assert.Equal(GameRunStatus.Running, tracker.Update(steamOnly, Launched.AddMinutes(30).AddSeconds(2))); // ainda na tolerância
        Assert.Equal(GameRunStatus.Exited, tracker.Update(steamOnly, Launched.AddMinutes(30).AddSeconds(4)));
        Assert.Equal(TimeSpan.FromMinutes(30) - TimeSpan.FromSeconds(40), tracker.PlayTime);
    }

    [Fact]
    public void Folder_tracking_times_out_when_the_game_never_shows_up()
    {
        var tracker = new GameRunTracker(new TrackingPlan(GameTrackingStrategy.InstallFolder, Folder), Launched);

        Assert.Equal(GameRunStatus.Waiting, tracker.Update([], Launched.AddSeconds(119)));
        Assert.Equal(GameRunStatus.TimedOut, tracker.Update([], Launched.AddMinutes(2)));
        Assert.Equal(TimeSpan.Zero, tracker.PlayTime);
        // Terminado não muda mais.
        Assert.Equal(GameRunStatus.TimedOut, tracker.Update([Proc(20, Folder + @"\Hades.exe")], Launched.AddMinutes(3)));
    }

    [Fact]
    public void A_short_gap_while_the_game_restarts_does_not_end_the_session()
    {
        var tracker = new GameRunTracker(new TrackingPlan(GameTrackingStrategy.InstallFolder, Folder), Launched, exitGrace: TimeSpan.FromSeconds(4));
        tracker.Update([Proc(20, Folder + @"\launcher.exe")], Launched.AddSeconds(2));
        tracker.Update([], Launched.AddSeconds(4));

        Assert.Equal(GameRunStatus.Running, tracker.Update([Proc(21, Folder + @"\bin\game.exe")], Launched.AddSeconds(6)));
    }

    // ------------------------------------------------------------------ Acompanhamento por processo

    [Fact]
    public void Process_tracking_counts_as_running_from_launch_and_follows_children()
    {
        var tracker = new GameRunTracker(new TrackingPlan(GameTrackingStrategy.Process), Launched, rootProcessId: 100);
        Assert.Equal(GameRunStatus.Running, tracker.Status);

        // O launcher do jogo abre o jogo e fecha; o filho continua contando.
        tracker.Update([new(100, 1), new(200, 100, StartedAt: Launched.AddSeconds(1))], Launched.AddSeconds(2));
        Assert.Equal(GameRunStatus.Running, tracker.Update([new(200, 100), new(300, 200)], Launched.AddSeconds(4)));
        Assert.Equal([200, 300], tracker.GameProcessIds);
        Assert.Equal(GameRunStatus.Running, tracker.Update([new(300, 200)], Launched.AddMinutes(10)));
        tracker.Update([new(999, 1)], Launched.AddMinutes(10).AddSeconds(2));

        Assert.Equal(GameRunStatus.Exited, tracker.Update([new(999, 1)], Launched.AddMinutes(10).AddSeconds(4)));
        Assert.Equal(TimeSpan.FromMinutes(10), tracker.PlayTime);
    }

    [Fact]
    public void Process_tracking_ignores_reused_ids_and_older_processes()
    {
        var tracker = new GameRunTracker(new TrackingPlan(GameTrackingStrategy.Process), Launched, rootProcessId: 100);
        tracker.Update([new(1, 0)], Launched.AddSeconds(2));

        // O id 100 voltou, mas é outro processo; o 50 diz ser filho do 100 mas começou antes do jogo.
        tracker.Update([new(100, 1), new(50, 100, StartedAt: Launched.AddHours(-1))], Launched.AddSeconds(4));

        Assert.Equal(GameRunStatus.Exited, tracker.Status);
        Assert.Empty(tracker.GameProcessIds);
    }

    [Fact]
    public void Constructor_rejects_plans_without_what_they_need()
    {
        Assert.Throws<ArgumentException>(() => new GameRunTracker(TrackingPlan.None, Launched));
        Assert.Throws<ArgumentException>(() => new GameRunTracker(new TrackingPlan(GameTrackingStrategy.InstallFolder), Launched));
        Assert.Throws<ArgumentException>(() => new GameRunTracker(new TrackingPlan(GameTrackingStrategy.Process), Launched));
    }
}
