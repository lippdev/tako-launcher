using Launcher.Core.Supervision;
using Launcher.Core.Updates;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Política de reinício, detecção de travamento e linha de comando do supervisor (MVP-006).</summary>
public sealed class SupervisorPolicyTests
{
    private static readonly SupervisorOptions Options = new() { MaxRestarts = 3, RestartWindow = TimeSpan.FromMinutes(5) };

    private static TimeSpan At(double seconds) => TimeSpan.FromSeconds(seconds);

    [Fact]
    public void Clean_exit_ends_the_supervisor()
    {
        var policy = new SupervisorPolicy(Options);
        policy.Begin(safeModeRequested: false);

        var decision = policy.OnUiExited(UiExitKind.Clean, At(1));

        Assert.Equal(SupervisorAction.Exit, decision.Action);
        Assert.Equal(0, policy.Restarts);
    }

    [Fact]
    public void Failures_restart_then_escalate_to_safe_mode_then_give_up()
    {
        var policy = new SupervisorPolicy(Options);
        Assert.Equal(UiLaunchMode.Normal, policy.Begin(safeModeRequested: false));

        var decisions = Enumerable.Range(1, 5).Select(i => policy.OnUiExited(i % 2 == 0 ? UiExitKind.Hung : UiExitKind.Crashed, At(i))).ToList();

        Assert.Equal(
            [
                (SupervisorAction.Restart, UiLaunchMode.Normal),
                (SupervisorAction.Restart, UiLaunchMode.Normal),
                (SupervisorAction.Restart, UiLaunchMode.Normal),
                (SupervisorAction.Restart, UiLaunchMode.SafeMode),
                (SupervisorAction.GiveUp, UiLaunchMode.SafeMode),
            ],
            decisions.Select(d => (d.Action, d.Mode)));
        Assert.Contains("travou", decisions[1].Reason, StringComparison.Ordinal);
        Assert.Contains("modo seguro", decisions[3].Reason, StringComparison.Ordinal);
        Assert.Equal(4, policy.Restarts);
    }

    [Fact]
    public void Failures_outside_the_window_do_not_add_up()
    {
        var policy = new SupervisorPolicy(Options);
        policy.Begin(safeModeRequested: false);

        // Uma queda a cada 10 minutos: nunca passa de 1 na janela de 5 minutos.
        var decisions = Enumerable.Range(0, 10).Select(i => policy.OnUiExited(UiExitKind.Crashed, TimeSpan.FromMinutes(i * 10))).ToList();

        Assert.All(decisions, d => Assert.Equal((SupervisorAction.Restart, UiLaunchMode.Normal, 1), (d.Action, d.Mode, d.FailuresInWindow)));
    }

    [Fact]
    public void Requested_safe_mode_gives_up_on_the_first_failure()
    {
        var policy = new SupervisorPolicy(Options);
        Assert.Equal(UiLaunchMode.SafeMode, policy.Begin(safeModeRequested: true));

        Assert.Equal(SupervisorAction.GiveUp, policy.OnUiExited(UiExitKind.Crashed, At(1)).Action);
    }

    [Theory]
    [InlineData(0, false, null, UiExitKind.Clean)]
    [InlineData(1, false, null, UiExitKind.Crashed)]
    [InlineData(-1, true, null, UiExitKind.Hung)]
    [InlineData(-1, false, UiCloseReason.Update, UiExitKind.Clean)]
    [InlineData(-1, true, UiCloseReason.SessionEnding, UiExitKind.Clean)]
    public void Exit_classification(int exitCode, bool hung, UiCloseReason? announced, UiExitKind expected) =>
        Assert.Equal(expected, SupervisorPolicy.Classify(exitCode, hung, announced));

    [Fact]
    public void Heartbeat_monitor_uses_startup_timeout_then_heartbeat_timeout()
    {
        var options = new SupervisorOptions { StartupTimeout = At(60), HeartbeatTimeout = At(20), CheckInterval = At(1) };
        var monitor = new HeartbeatMonitor(options);
        monitor.Reset(At(0));

        for (int s = 1; s <= 59; s++)
        {
            Assert.False(monitor.IsHung(At(s)));
        }

        monitor.Beat(At(59));
        for (int s = 60; s <= 79; s++)
        {
            Assert.False(monitor.IsHung(At(s)));
        }

        Assert.True(monitor.IsHung(At(80)));
        Assert.Equal(1, monitor.Beats);
    }

    [Fact]
    public void Heartbeat_monitor_never_connected_hangs_after_startup_timeout()
    {
        var monitor = new HeartbeatMonitor(new SupervisorOptions { StartupTimeout = At(3), CheckInterval = At(1) });
        monitor.Reset(At(0));

        Assert.False(monitor.IsHung(At(1)));
        Assert.False(monitor.IsHung(At(2)));
        Assert.False(monitor.IsHung(At(3)));
        Assert.True(monitor.IsHung(At(4)));
    }

    [Fact]
    public void Heartbeat_monitor_forgives_a_suspended_pc()
    {
        var monitor = new HeartbeatMonitor(new SupervisorOptions { HeartbeatTimeout = At(20), CheckInterval = At(1) });
        monitor.Reset(At(0));
        monitor.Beat(At(1));
        Assert.False(monitor.IsHung(At(2)));

        // PC dormiu por uma hora: a primeira conferência ao acordar não mata a UI; o prazo recomeça.
        Assert.False(monitor.IsHung(At(3600)));
        for (int s = 3601; s <= 3620; s++)
        {
            Assert.False(monitor.IsHung(At(s)));
        }

        // Sem heartbeat depois de acordar: trava detectada no prazo normal.
        Assert.True(monitor.IsHung(At(3621)));
    }

    [Theory]
    [InlineData(new string[0], null, true, ProcessRole.Supervisor)]
    [InlineData(new string[0], null, false, ProcessRole.Ui)]
    [InlineData(new[] { "--ui", "--supervisor-pipe", "p" }, null, true, ProcessRole.Ui)]
    [InlineData(new[] { "--no-supervisor" }, null, true, ProcessRole.Ui)]
    [InlineData(new string[0], "0", true, ProcessRole.Ui)]
    [InlineData(new string[0], "1", false, ProcessRole.Supervisor)]
    [InlineData(new[] { "--UI" }, "1", true, ProcessRole.Ui)]
    [InlineData(new[] { "--safe-mode" }, null, true, ProcessRole.Supervisor)]
    public void Role_comes_from_arguments_environment_and_build(string[] args, string? environment, bool published, ProcessRole expected) =>
        Assert.Equal(expected, SupervisorArguments.ResolveRole(args, environment, published));

    [Fact]
    public void Ui_command_line_carries_pipe_mode_and_user_arguments()
    {
        string[] userArgs = ["--safe-mode", "--extra", "valor", "--no-supervisor"];
        var passthrough = SupervisorArguments.Passthrough(userArgs);

        var normal = SupervisorArguments.ForUi("tako-sv-1", UiLaunchMode.Normal, passthrough);
        var safe = SupervisorArguments.ForUi("tako-sv-1", UiLaunchMode.SafeMode, passthrough);

        Assert.Equal(["--ui", "--supervisor-pipe", "tako-sv-1", "--extra", "valor"], normal);
        Assert.Equal(["--ui", "--supervisor-pipe", "tako-sv-1", "--safe-mode", "--extra", "valor"], safe);
        Assert.Equal("tako-sv-1", SupervisorArguments.PipeName(safe));
        Assert.Null(SupervisorArguments.PipeName(["--ui"]));
        Assert.Equal(["--extra"], SupervisorArguments.Passthrough(["--ui", "--supervisor-pipe", "x", "--extra"]));
    }

    [Fact]
    public void Portable_swap_waits_for_ui_and_supervisor_before_replacing_the_exe()
    {
        string script = PortableSwapScript.Build(@"C:\Temp\TakoUpdate\Tako-Portable-x64.exe", @"D:\Jogos\Tako.exe", [4242, 77, 4242], relaunch: true);

        string[] lines = script.Split("\r\n", StringSplitOptions.RemoveEmptyEntries);
        Assert.Equal("@echo off", lines[0]);
        Assert.Contains("PID eq 4242", lines[2], StringComparison.Ordinal);
        Assert.Contains("goto wait0", lines[2], StringComparison.Ordinal);
        Assert.Contains("PID eq 77", lines[4], StringComparison.Ordinal);
        Assert.Contains("goto wait1", lines[4], StringComparison.Ordinal);
        Assert.Equal(@"move /y ""C:\Temp\TakoUpdate\Tako-Portable-x64.exe"" ""D:\Jogos\Tako.exe"" >nul", lines[5]);
        Assert.Equal(@"start """" ""D:\Jogos\Tako.exe""", lines[6]);
        Assert.DoesNotContain("start", PortableSwapScript.Build("a.exe", "b.exe", [1], relaunch: false), StringComparison.Ordinal);
    }
}
