using System.Runtime.CompilerServices;
using Launcher.Core.Supervision;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf;

/// <summary>
/// Ponto de entrada do <c>Tako.exe</c> único (ADR-0012). Aberto normalmente (atalho, "iniciar com o Windows",
/// instalador), vira o supervisor leve sem janela, que abre o mesmo executável com <c>--ui</c>. Com <c>--ui</c>, ou fora
/// das versões publicadas (<c>dotnet run</c>, testes), roda a interface direto. <c>TAKO_SUPERVISOR=0/1</c> força.
/// </summary>
public static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        var role = SupervisorArguments.ResolveRole(args, Environment.GetEnvironmentVariable(SupervisorArguments.EnvironmentVariable), AppPaths.IsPublishedBuild);
        return role == ProcessRole.Supervisor && Environment.ProcessPath is { } executable
            ? SupervisorHost.Run(executable, args)
            : RunUi();
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
