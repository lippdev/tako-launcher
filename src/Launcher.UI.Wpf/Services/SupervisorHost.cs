using System.IO;
using ConsoleMode.Runtime.Ipc;
using ConsoleMode.Runtime.Windows;
using Launcher.Core.Diagnostics;
using Launcher.Core.Session.GameScreen;
using Launcher.Core.Session.Restore;
using Launcher.Core.Supervision;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Processo supervisor do <c>Tako.exe</c> (MVP-006): sem janela e sem WPF; só abre a UI e a vigia (<see cref="UiSupervisor"/>).
/// Os logs vão para <c>logs/supervisor/</c>, separados dos da UI (dois processos não escrevem no mesmo arquivo).
/// </summary>
public static class SupervisorHost
{
    public static int Run(string executable, IReadOnlyList<string> args)
    {
        using var fileLog = new FileLoggerProvider(Path.Combine(AppPaths.LogsDirectory, "supervisor"), TimeProvider.System);
        using var loggers = LoggerFactory.Create(logging => logging.AddProvider(fileLog).AddDebug().SetMinimumLevel(LogLevel.Information));
        string pipe = NamedPipeIpc.NewPipeName("tako-sv");
        var supervisor = new UiSupervisor(
            new ProcessUiLauncher(executable),
            new NamedPipeServerTransport(pipe),
            pipe,
            new SupervisorOptions(),
            TimeProvider.System,
            loggers.CreateLogger<UiSupervisor>());

        // Main sem contexto de sincronização: esperar aqui não trava nada.
        int exitCode = supervisor.RunAsync(SupervisorArguments.Has(args, SupervisorArguments.SafeMode), SupervisorArguments.Passthrough(args)).GetAwaiter().GetResult();
        RecoverConsoleMode(loggers);
        fileLog.Flush(TimeSpan.FromSeconds(2));
        return exitCode;
    }

    /// <summary>
    /// A UI não volta mais (o supervisor desistiu, ou ela foi encerrada à força) e deixou o modo console ativo: as telas
    /// voltam para a mesa aqui, para ninguém ficar no desktop com as telas desligadas (MVP-036). Numa saída normal o
    /// marcador já não existe e nada é lido do Windows.
    /// </summary>
    private static void RecoverConsoleMode(ILoggerFactory loggers)
    {
        var store = new PendingEnvironmentStore(AppPaths.ConsoleModeEnvironmentFile);
        var markers = new ConsoleModeMarkerStore(AppPaths.ConsoleModeMarkerFile);
        if (!markers.Exists() && !store.Exists())
        {
            return;
        }

        var environment = new SessionEnvironmentGuard(new WindowsDisplayConfiguration(), new WindowsAudioEndpoints(), store, TimeProvider.System, loggers.CreateLogger<SessionEnvironmentGuard>());
        new ConsoleModeRecovery(new WindowsDisplayControl(), environment, store, markers, loggers.CreateLogger<ConsoleModeRecovery>()).Recover();
    }
}
