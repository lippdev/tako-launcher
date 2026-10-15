using ConsoleMode.Runtime.Ipc;
using Launcher.Core.Supervision;
using Microsoft.Extensions.Logging.Abstractions;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// Supervisor de ponta a ponta (MVP-006) com Named Pipe real e processos de UI simulados: cada "UI" conecta pelo IPC,
/// manda heartbeats e termina do jeito que o cenário pede.
/// </summary>
public sealed class UiSupervisorTests
{
    private static readonly SupervisorOptions Fast = new()
    {
        MaxRestarts = 2,
        HeartbeatInterval = TimeSpan.FromMilliseconds(30),
        HeartbeatTimeout = TimeSpan.FromMilliseconds(400),
        StartupTimeout = TimeSpan.FromSeconds(3),
        CheckInterval = TimeSpan.FromMilliseconds(25),
        ClosingGrace = TimeSpan.FromMilliseconds(300),
        RestartDelay = TimeSpan.FromMilliseconds(1),
    };

    [Fact]
    public async Task Clean_ui_exit_ends_the_supervisor_after_talking_over_ipc()
    {
        SupervisorStatus? status = null;
        var launcher = new FakeLauncher(async ui =>
        {
            await using var client = await ui.ConnectAsync();
            await ui.BeatAsync(client, 3);
            status = await client.RequestAsync<SupervisorStatus>(SupervisorMessages.Status, timeout: TimeSpan.FromSeconds(5));
            ui.Exit(0);
        });

        var (exitCode, supervisor) = await RunAsync(launcher);

        Assert.Equal(0, exitCode);
        Assert.Equal(1, supervisor.Launches);
        Assert.Equal(new SupervisorStatus(Environment.ProcessId, 0, UiLaunchMode.Normal), status);
        Assert.False(launcher.Processes[0].Killed);
    }

    [Fact]
    public async Task Crashing_ui_is_restarted_then_opened_in_safe_mode_then_abandoned()
    {
        var launcher = new FakeLauncher(async ui =>
        {
            await using var client = await ui.ConnectAsync();
            await ui.BeatAsync(client, 1);
            ui.Exit(1);
        });

        var (exitCode, supervisor) = await RunAsync(launcher);

        Assert.Equal(1, exitCode); // desistiu: o usuário fica no desktop
        Assert.Equal(4, supervisor.Launches); // 1 + 2 reinícios + 1 em modo seguro
        Assert.Equal([false, false, false, true], launcher.Launches.Select(args => args.Contains(SupervisorArguments.SafeMode)));
        Assert.All(launcher.Launches, args => Assert.Equal(SupervisorArguments.Ui, args[0]));
    }

    [Fact]
    public async Task Ui_that_stops_sending_heartbeats_is_killed_and_restarted()
    {
        var launcher = new FakeLauncher(async ui =>
        {
            await using var client = await ui.ConnectAsync();
            await ui.BeatAsync(client, 2);
            if (ui.Index == 0)
            {
                await ui.Exited; // "travou": não manda mais nada até ser encerrada
                return;
            }

            ui.Exit(0);
        });

        var (exitCode, supervisor) = await RunAsync(launcher);

        Assert.Equal(0, exitCode);
        Assert.Equal(2, supervisor.Launches);
        Assert.True(launcher.Processes[0].Killed);
        Assert.False(launcher.Processes[1].Killed);
    }

    [Fact]
    public async Task Ui_that_never_connects_counts_as_hung()
    {
        var launcher = new FakeLauncher(async ui =>
        {
            if (ui.Index == 0)
            {
                await ui.Exited; // abriu e nunca falou com o supervisor
                return;
            }

            ui.Exit(0);
        });

        var (exitCode, _) = await RunAsync(launcher, Fast with { StartupTimeout = TimeSpan.FromMilliseconds(300) });

        Assert.Equal(0, exitCode);
        Assert.True(launcher.Processes[0].Killed);
    }

    [Fact]
    public async Task Announced_update_exit_is_not_restarted_even_with_an_error_code()
    {
        var launcher = new FakeLauncher(async ui =>
        {
            await using var client = await ui.ConnectAsync();
            await ui.BeatAsync(client, 1);
            await client.WaitForConnectionAsync(TimeSpan.FromSeconds(5));
            await client.Current!.SendEventAsync(SupervisorMessages.Closing, new UiClosing(ui.Process.Id, UiCloseReason.Update));
            await Task.Delay(100);
            ui.Exit(-1); // ex.: fechado à força pelo instalador
        });

        var (exitCode, supervisor) = await RunAsync(launcher);

        Assert.Equal(0, exitCode);
        Assert.Equal(1, supervisor.Launches);
    }

    [Fact]
    public async Task Announced_exit_that_never_finishes_is_killed_after_the_grace_period()
    {
        var launcher = new FakeLauncher(async ui =>
        {
            await using var client = await ui.ConnectAsync();
            await client.WaitForConnectionAsync(TimeSpan.FromSeconds(5));
            await client.Current!.SendEventAsync(SupervisorMessages.Closing, new UiClosing(ui.Process.Id, UiCloseReason.Update));
            await ui.Exited;
        });

        var (exitCode, supervisor) = await RunAsync(launcher);

        Assert.Equal(0, exitCode);
        Assert.Equal(1, supervisor.Launches);
        Assert.True(launcher.Processes[0].Killed);
    }

    [Fact]
    public async Task Heartbeats_from_an_old_ui_process_do_not_keep_the_new_one_alive()
    {
        var launcher = new FakeLauncher(async ui =>
        {
            await using var client = await ui.ConnectAsync();
            if (ui.Index == 0)
            {
                await ui.BeatAsync(client, 1);
                ui.Exit(1);
                return;
            }

            // A segunda UI finge ser a primeira: o supervisor só aceita heartbeats do PID atual.
            using var stop = new CancellationTokenSource();
            var impostor = Task.Run(async () =>
            {
                while (!stop.IsCancellationRequested)
                {
                    await client.TrySendEventAsync(SupervisorMessages.Heartbeat, new UiHeartbeat(1000, 1));
                    await Task.Delay(20);
                }
            });
            await ui.Exited;
            await stop.CancelAsync();
            await impostor;
        });

        var (_, _) = await RunAsync(launcher, Fast with { StartupTimeout = TimeSpan.FromMilliseconds(400) });

        Assert.True(launcher.Processes[1].Killed);
    }

    [Fact]
    public async Task Session_ending_keeps_watching_but_does_not_restart()
    {
        var launcher = new FakeLauncher(async ui =>
        {
            await using var client = await ui.ConnectAsync();
            await ui.BeatAsync(client, 1);
            await client.Current!.SendEventAsync(SupervisorMessages.Closing, new UiClosing(ui.Process.Id, UiCloseReason.SessionEnding));
            await ui.BeatAsync(client, 15); // o Windows cancelou o logoff: a UI segue viva além do prazo de saída
            ui.Exit(1);
        });

        var (exitCode, supervisor) = await RunAsync(launcher);

        Assert.Equal(0, exitCode);
        Assert.Equal(1, supervisor.Launches);
        Assert.False(launcher.Processes[0].Killed);
    }

    private static async Task<(int ExitCode, UiSupervisor Supervisor)> RunAsync(FakeLauncher launcher, SupervisorOptions? options = null)
    {
        string pipe = NamedPipeIpc.NewPipeName("tk-sv");
        var supervisor = new UiSupervisor(launcher, new NamedPipeServerTransport(pipe), pipe, options ?? Fast, TimeProvider.System, NullLogger<UiSupervisor>.Instance);
        int exitCode = await supervisor.RunAsync(safeModeRequested: false, [], TestContext.Current.CancellationToken).WaitAsync(TimeSpan.FromSeconds(30), TestContext.Current.CancellationToken);
        await launcher.DrainAsync();
        return (exitCode, supervisor);
    }

    /// <summary>UI simulada: o cenário recebe o processo falso e os argumentos com que foi aberto.</summary>
    private sealed class FakeUi(int index, FakeProcess process, IReadOnlyList<string> args)
    {
        public int Index { get; } = index;

        public FakeProcess Process { get; } = process;

        public Task Exited => Process.ExitTask;

        public async Task<IpcClient> ConnectAsync()
        {
            string pipe = SupervisorArguments.PipeName(args) ?? throw new InvalidOperationException("sem pipe");
            var client = new IpcClient(new NamedPipeClientTransport(pipe), new IpcClientOptions
            {
                Connection = new IpcConnectionOptions { PeerName = SupervisorMessages.UiPeer },
                ReconnectDelay = TimeSpan.FromMilliseconds(20),
            });
            client.Start();
            await client.WaitForConnectionAsync(TimeSpan.FromSeconds(5));
            return client;
        }

        public async Task BeatAsync(IpcClient client, int count)
        {
            for (int i = 0; i < count; i++)
            {
                await client.TrySendEventAsync(SupervisorMessages.Heartbeat, new UiHeartbeat(Process.Id, i));
                await Task.Delay(30);
            }
        }

        public void Exit(int code) => Process.Finish(code);
    }

    private sealed class FakeProcess(int id) : IUiProcess
    {
        private readonly TaskCompletionSource<int> _exit = new(TaskCreationOptions.RunContinuationsAsynchronously);

        public int Id { get; } = id;

        public bool Killed { get; private set; }

        public Task ExitTask => _exit.Task;

        public Task<int> WaitForExitAsync(CancellationToken cancellationToken) => _exit.Task.WaitAsync(cancellationToken);

        public void Kill()
        {
            Killed = true;
            _exit.TrySetResult(-1);
        }

        public void Finish(int code) => _exit.TrySetResult(code);

        public void Dispose()
        {
        }
    }

    private sealed class FakeLauncher(Func<FakeUi, Task> scenario) : IUiProcessLauncher
    {
        private readonly List<Task> _running = [];

        public List<IReadOnlyList<string>> Launches { get; } = [];

        public List<FakeProcess> Processes { get; } = [];

        public IUiProcess Start(IReadOnlyList<string> arguments)
        {
            var process = new FakeProcess(1000 + Processes.Count);
            var ui = new FakeUi(Processes.Count, process, arguments);
            Launches.Add(arguments);
            Processes.Add(process);
            _running.Add(Task.Run(() => scenario(ui)));
            return process;
        }

        public Task DrainAsync() => Task.WhenAll(_running).WaitAsync(TimeSpan.FromSeconds(10));
    }
}
