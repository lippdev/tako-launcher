using System.ComponentModel;
using System.Diagnostics;
using ConsoleMode.Runtime.Ipc;
using Microsoft.Extensions.Logging;

namespace Launcher.Core.Supervision;

/// <summary>Processo da UI visto pelo supervisor (abstrato para testar sem abrir processos).</summary>
public interface IUiProcess : IDisposable
{
    int Id { get; }

    /// <summary>Espera o processo terminar e devolve o código de saída.</summary>
    Task<int> WaitForExitAsync(CancellationToken cancellationToken);

    /// <summary>Encerra só a UI (os jogos iniciados por ela continuam).</summary>
    void Kill();
}

public interface IUiProcessLauncher
{
    /// <exception cref="Win32Exception">O executável não pôde ser iniciado.</exception>
    IUiProcess Start(IReadOnlyList<string> arguments);
}

/// <summary>Inicia o próprio <c>Tako.exe</c> com <c>--ui</c>.</summary>
public sealed class ProcessUiLauncher(string executablePath) : IUiProcessLauncher
{
    public IUiProcess Start(IReadOnlyList<string> arguments)
    {
        ArgumentNullException.ThrowIfNull(arguments);
        var info = new ProcessStartInfo(executablePath)
        {
            UseShellExecute = false,
            WorkingDirectory = Path.GetDirectoryName(executablePath) ?? Environment.CurrentDirectory,
        };
        foreach (string argument in arguments)
        {
            info.ArgumentList.Add(argument);
        }

        return new SystemProcess(Process.Start(info) ?? throw new Win32Exception("Process.Start não iniciou a UI."));
    }

    private sealed class SystemProcess(Process process) : IUiProcess
    {
        public int Id => process.Id;

        public async Task<int> WaitForExitAsync(CancellationToken cancellationToken)
        {
            await process.WaitForExitAsync(cancellationToken).ConfigureAwait(false);
            return process.ExitCode;
        }

        public void Kill()
        {
            try
            {
                process.Kill(entireProcessTree: false);
            }
            catch (InvalidOperationException)
            {
                // Já terminou.
            }
            catch (Win32Exception)
            {
                // Terminando agora; WaitForExitAsync confirma.
            }
        }

        public void Dispose() => process.Dispose();
    }
}

/// <summary>
/// Supervisor da UI (MVP-006, ADR-0012): abre a UI com <c>--ui</c>, escuta os heartbeats pelo IPC e aplica a
/// <see cref="SupervisorPolicy"/> quando ela cai ou trava. Não tem janela e não carrega WPF.
/// </summary>
public sealed partial class UiSupervisor(
    IUiProcessLauncher launcher,
    IIpcServerTransport transport,
    string pipeName,
    SupervisorOptions options,
    TimeProvider time,
    ILogger<UiSupervisor> logger)
{
    private readonly object _gate = new();
    private readonly SupervisorPolicy _policy = new(options);
    private readonly HeartbeatMonitor _heartbeats = new(options);
    private readonly long _started = time.GetTimestamp();
    private int _currentPid;
    private UiCloseReason? _announced;
    private TimeSpan _announcedAt;

    /// <summary>Aberturas da UI nesta execução (diagnóstico e testes).</summary>
    public int Launches { get; private set; }

    private TimeSpan Now => time.GetElapsedTime(_started);

    /// <summary>
    /// Roda até a UI fechar de vez. Devolve o código de saída do supervisor: 0 quando a UI fechou normalmente, 1 quando
    /// o supervisor desistiu (o usuário fica no desktop).
    /// </summary>
    public async Task<int> RunAsync(bool safeModeRequested, IReadOnlyList<string> passthrough, CancellationToken cancellationToken = default)
    {
        await using var server = new IpcServer(transport, new IpcConnectionOptions
        {
            PeerName = SupervisorMessages.SupervisorPeer,
            Handler = new IpcHandlerMap()
                .OnEvent(SupervisorMessages.Heartbeat, payload => OnHeartbeat(IpcConnection.FromElement<UiHeartbeat>(payload)))
                .OnEvent(SupervisorMessages.Closing, payload => OnClosing(IpcConnection.FromElement<UiClosing>(payload)))
                .OnRequest(SupervisorMessages.Status, _ => new SupervisorStatus(Environment.ProcessId, _policy.Restarts, _policy.Mode)),
            HandlerError = ex => LogIpcHandlerFailed(logger, ex),
        });
        server.HandshakeFailed += (_, e) => LogHandshakeFailed(logger, e.Error.Message);
        server.Start();

        var mode = _policy.Begin(safeModeRequested);
        LogStarted(logger, pipeName, mode);
        while (true)
        {
            IUiProcess process;
            try
            {
                process = StartUi(mode, passthrough);
            }
            catch (Exception ex) when (ex is Win32Exception or IOException or UnauthorizedAccessException)
            {
                LogStartFailed(logger, ex);
                return 1;
            }

            using (process)
            {
                var (exitCode, kind) = await WatchAsync(process, cancellationToken).ConfigureAwait(false);
                if (cancellationToken.IsCancellationRequested)
                {
                    return 0;
                }

                var decision = _policy.OnUiExited(kind, Now);
                LogUiExited(logger, process.Id, exitCode, kind, decision.Action, decision.Reason);
                switch (decision.Action)
                {
                    case SupervisorAction.Exit:
                        return 0;
                    case SupervisorAction.GiveUp:
                        return 1;
                }

                mode = decision.Mode;
            }

            await Task.Delay(options.RestartDelay, time, cancellationToken).ConfigureAwait(ConfigureAwaitOptions.SuppressThrowing);
            if (cancellationToken.IsCancellationRequested)
            {
                return 0;
            }
        }
    }

    private IUiProcess StartUi(UiLaunchMode mode, IReadOnlyList<string> passthrough)
    {
        var process = launcher.Start(SupervisorArguments.ForUi(pipeName, mode, passthrough));
        lock (_gate)
        {
            _currentPid = process.Id;
            _announced = null;
            _heartbeats.Reset(Now);
        }

        Launches++;
        LogUiStarted(logger, process.Id, mode);
        return process;
    }

    private async Task<(int ExitCode, UiExitKind Kind)> WatchAsync(IUiProcess process, CancellationToken cancellationToken)
    {
        var exit = process.WaitForExitAsync(cancellationToken);
        while (true)
        {
            var tick = Task.Delay(options.CheckInterval, time, cancellationToken);
            if (await Task.WhenAny(exit, tick).ConfigureAwait(false) == exit)
            {
                int code = exit.IsCompletedSuccessfully ? exit.Result : -1; // já terminou (ou foi cancelado)
                lock (_gate)
                {
                    return (code, SupervisorPolicy.Classify(code, hung: false, _announced));
                }
            }

            if (cancellationToken.IsCancellationRequested)
            {
                return (-1, UiExitKind.Clean);
            }

            bool hung;
            bool overdueClose;
            lock (_gate)
            {
                // Atualização anunciada: a UI tem um prazo para sair. Fim de sessão pode ser cancelado pelo Windows, então
                // a UI continua vigiada normalmente; só não é reaberta quando sair.
                overdueClose = _announced == UiCloseReason.Update && Now - _announcedAt > options.ClosingGrace;
                hung = _announced != UiCloseReason.Update && _heartbeats.IsHung(Now);
            }

            if (hung || overdueClose)
            {
                if (hung)
                {
                    LogUiHung(logger, process.Id, _heartbeats.Beats);
                }

                process.Kill();
                await ((Task)exit).WaitAsync(TimeSpan.FromSeconds(10), cancellationToken).ConfigureAwait(ConfigureAwaitOptions.SuppressThrowing);
                lock (_gate)
                {
                    return (exit.IsCompletedSuccessfully ? exit.Result : -1, SupervisorPolicy.Classify(-1, hung, _announced));
                }
            }
        }
    }

    private void OnHeartbeat(UiHeartbeat? beat)
    {
        lock (_gate)
        {
            if (beat is not null && beat.ProcessId == _currentPid)
            {
                _heartbeats.Beat(Now);
            }
        }
    }

    private void OnClosing(UiClosing? closing)
    {
        if (closing is null)
        {
            return;
        }

        lock (_gate)
        {
            if (closing.ProcessId != _currentPid)
            {
                return;
            }

            _announced = closing.Reason;
            _announcedAt = Now;
        }

        LogUiClosing(logger, closing.ProcessId, closing.Reason);
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Supervisor iniciado (pipe {Pipe}, modo {Mode})")]
    private static partial void LogStarted(ILogger logger, string pipe, UiLaunchMode mode);

    [LoggerMessage(Level = LogLevel.Information, Message = "UI iniciada: PID {Pid}, modo {Mode}")]
    private static partial void LogUiStarted(ILogger logger, int pid, UiLaunchMode mode);

    [LoggerMessage(Level = LogLevel.Warning, Message = "UI {Pid} terminou (código {ExitCode}, {Kind}): {Action}, {Reason}")]
    private static partial void LogUiExited(ILogger logger, int pid, int exitCode, UiExitKind kind, SupervisorAction action, string reason);

    [LoggerMessage(Level = LogLevel.Error, Message = "UI {Pid} sem heartbeat no prazo ({Beats} recebidos): encerrando")]
    private static partial void LogUiHung(ILogger logger, int pid, long beats);

    [LoggerMessage(Level = LogLevel.Information, Message = "UI {Pid} anunciou a saída: {Reason}")]
    private static partial void LogUiClosing(ILogger logger, int pid, UiCloseReason reason);

    [LoggerMessage(Level = LogLevel.Critical, Message = "Não foi possível iniciar a UI; o supervisor sai")]
    private static partial void LogStartFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Cliente IPC recusado: {Reason}")]
    private static partial void LogHandshakeFailed(ILogger logger, string reason);

    [LoggerMessage(Level = LogLevel.Error, Message = "Falha ao tratar mensagem IPC da UI")]
    private static partial void LogIpcHandlerFailed(ILogger logger, Exception exception);
}
