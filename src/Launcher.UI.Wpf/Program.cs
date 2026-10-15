using System.IO;
using System.Runtime.CompilerServices;
using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Windows.SessionTools;
using Launcher.Core.Persistence;
using Launcher.Core.Supervision;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf;

/// <summary>
/// Ponto de entrada do <c>Tako.exe</c> único (ADR-0012). Aberto normalmente (atalho, "iniciar com o Windows",
/// instalador), vira o supervisor leve sem janela, que abre o mesmo executável com <c>--ui</c>. Com <c>--ui</c>, ou fora
/// das versões publicadas (<c>dotnet run</c>, testes), roda a interface direto. <c>TAKO_SUPERVISOR=0/1</c> força.
/// Três modos sem janela servem ao contador de FPS (ADR-0017): <c>--grant-perf-log-access &lt;SID&gt;</c> e
/// <c>--revoke-perf-log-access &lt;SID&gt;</c> (o passo elevado, aberto pelo UAC) e <c>--setup-fps-counter</c> (o
/// instalador: pede o UAC para o usuário que instalou e liga "Mostrar FPS").
/// </summary>
public static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        if (PerfLogAccess.TryParse(args, out bool invalid) is { } command)
        {
            return WindowsPerfLogAccess.RunElevated(command);
        }

        if (invalid)
        {
            return PerfLogAccess.ExitInvalidArguments;
        }

        if (args.Contains(PerfLogAccess.SetupArgument, StringComparer.OrdinalIgnoreCase))
        {
            return SetupFpsCounter();
        }

        var role = SupervisorArguments.ResolveRole(args, Environment.GetEnvironmentVariable(SupervisorArguments.EnvironmentVariable), AppPaths.IsPublishedBuild);
        return role == ProcessRole.Supervisor && Environment.ProcessPath is { } executable
            ? SupervisorHost.Run(executable, args)
            : RunUi();
    }

    /// <summary>
    /// Instalador com "Ativar contador de FPS" marcado: roda como o usuário que instalou (o instalador é por usuário,
    /// sem administrador), então o SID lido aqui é o dele; o UAC abre só para o passo elevado.
    /// </summary>
    private static int SetupFpsCounter()
    {
        var access = new WindowsPerfLogAccess(Environment.ProcessPath ?? "Tako.exe", Microsoft.Extensions.Logging.Abstractions.NullLogger<WindowsPerfLogAccess>.Instance);
        var result = access.GrantAsync().GetAwaiter().GetResult();
        if (result != PerfLogChange.Done)
        {
            return result == PerfLogChange.Cancelled ? PerfLogAccess.UacCancelled : PerfLogAccess.ExitFailed;
        }

        try
        {
            new SettingsStore(new TakoDatabase(AppPaths.DatabaseFile)).Set(SessionToolsService.EnabledKey, "true");
            return PerfLogAccess.ExitOk;
        }
        catch (Exception ex) when (ex is Microsoft.Data.Sqlite.SqliteException or IOException or UnauthorizedAccessException)
        {
            return PerfLogAccess.ExitFailed; // a permissão ficou; "Mostrar FPS" é ligado depois nas Configurações
        }
    }

    /// <summary>Separado para o supervisor não carregar o WPF (o JIT só resolve <see cref="App"/> se este método rodar).</summary>
    [MethodImpl(MethodImplOptions.NoInlining)]
    private static int RunUi()
    {
        var app = new App();
        app.InitializeComponent();
        return app.Run();
    }
}
