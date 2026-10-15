using Launcher.Core.Resilience;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class ResilienceTests : IDisposable
{
    private static readonly DateTimeOffset Now = new(2026, 10, 15, 20, 0, 0, TimeSpan.Zero);
    private readonly string _directory = Path.Combine(Path.GetTempPath(), "tako-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_directory))
        {
            Directory.Delete(_directory, recursive: true);
        }
    }

    [Fact]
    public void First_start_is_normal() =>
        Assert.False(CrashLoopPolicy.Decide(null, safeModeRequested: false, Now).SafeMode);

    [Fact]
    public void Start_after_clean_exit_is_normal()
    {
        var decision = CrashLoopPolicy.Decide(CrashLoopPolicy.CleanExit(), safeModeRequested: false, Now);

        Assert.False(decision.SafeMode);
        Assert.Equal(0, decision.UncleanExits);
        Assert.True(decision.Marker.Running);
    }

    [Fact]
    public void One_unclean_exit_is_not_enough_for_safe_mode()
    {
        var decision = CrashLoopPolicy.Decide(new SessionMarker(true, 0, Now), safeModeRequested: false, Now);

        Assert.False(decision.SafeMode);
        Assert.Equal(1, decision.UncleanExits);
    }

    [Fact]
    public void Two_unclean_exits_in_a_row_start_in_safe_mode()
    {
        var decision = CrashLoopPolicy.Decide(new SessionMarker(true, 1, Now), safeModeRequested: false, Now);

        Assert.True(decision.SafeMode);
        Assert.Equal(StartupReason.CrashLoop, decision.Reason);
        Assert.Equal(2, decision.UncleanExits);
    }

    [Fact]
    public void Requested_safe_mode_wins_over_a_clean_history()
    {
        var decision = CrashLoopPolicy.Decide(CrashLoopPolicy.CleanExit(), safeModeRequested: true, Now);

        Assert.True(decision.SafeMode);
        Assert.Equal(StartupReason.Requested, decision.Reason);
    }

    [Fact]
    public void Journal_counts_crashes_and_clears_on_clean_exit()
    {
        string file = Path.Combine(_directory, "session.json");
        var time = new FixedTimeProvider(Now);

        // Duas aberturas que não fecharam: a terceira entra no modo seguro.
        Assert.False(new SessionJournal(file, time).Begin(false).SafeMode);
        Assert.False(new SessionJournal(file, time).Begin(false).SafeMode);
        var third = new SessionJournal(file, time);
        Assert.True(third.Begin(false).SafeMode);

        // Fechamento limpo no modo seguro: a próxima abertura é normal.
        third.EndClean();
        Assert.False(new SessionJournal(file, time).Begin(false).SafeMode);
    }

    [Fact]
    public void Leaving_safe_mode_forgives_previous_crashes()
    {
        string file = Path.Combine(_directory, "session.json");
        var time = new FixedTimeProvider(Now);
        new SessionJournal(file, time).Begin(false);
        new SessionJournal(file, time).Begin(false);
        var safe = new SessionJournal(file, time);
        Assert.True(safe.Begin(false).SafeMode);

        safe.Forgive();

        // Travou de novo depois de voltar ao normal: uma falha só, ainda não é loop.
        Assert.False(new SessionJournal(file, time).Begin(false).SafeMode);
    }

    [Fact]
    public void Corrupted_marker_counts_as_one_unclean_exit()
    {
        Directory.CreateDirectory(_directory);
        string file = Path.Combine(_directory, "session.json");
        File.WriteAllText(file, "{\"Running\": tr");

        var decision = new SessionJournal(file, new FixedTimeProvider(Now)).Begin(false);

        Assert.False(decision.SafeMode);
        Assert.Equal(1, decision.UncleanExits);
    }

    [Fact]
    public void Unwritable_folder_still_starts()
    {
        Directory.CreateDirectory(_directory);
        string blocker = Path.Combine(_directory, "arquivo");
        File.WriteAllText(blocker, string.Empty);

        // A "pasta" de dados é um arquivo: não dá para gravar, mas a abertura segue normal.
        var journal = new SessionJournal(Path.Combine(blocker, "session.json"), new FixedTimeProvider(Now));

        Assert.False(journal.Begin(false).SafeMode);
        journal.EndClean();
    }

    [Fact]
    public void Fault_budget_allows_isolated_errors()
    {
        var budget = new FaultBudget(3, TimeSpan.FromSeconds(10));

        Assert.True(budget.TryRecord(TimeSpan.FromSeconds(0)));
        Assert.True(budget.TryRecord(TimeSpan.FromSeconds(20)));
        Assert.True(budget.TryRecord(TimeSpan.FromSeconds(40)));
        Assert.True(budget.TryRecord(TimeSpan.FromSeconds(60)));
    }

    [Fact]
    public void Fault_budget_trips_on_a_burst()
    {
        var budget = new FaultBudget(3, TimeSpan.FromSeconds(10));

        Assert.True(budget.TryRecord(TimeSpan.FromSeconds(1)));
        Assert.True(budget.TryRecord(TimeSpan.FromSeconds(2)));
        Assert.True(budget.TryRecord(TimeSpan.FromSeconds(3)));
        Assert.False(budget.TryRecord(TimeSpan.FromSeconds(4)));
    }
}
