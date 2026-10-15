using System.IO;
using System.Runtime.InteropServices;

namespace Launcher.FseProbe;

/// <summary>
/// Ferramenta do spike MVP-007 (FSE / Xbox mode).
/// <c>--report</c> (padrão): coleta dados do ambiente. <c>--home</c>: tela registrada como gaming home app que registra o ciclo de vida.
/// Os logs ficam em Documentos, que não é virtualizado para apps empacotados.
/// </summary>
internal static partial class Program
{
    private const int AttachParentProcess = -1;

    public static string LogDirectory { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments), "UltraLauncher-FseProbe");

    [STAThread]
    private static int Main(string[] args)
    {
        Directory.CreateDirectory(LogDirectory);
        if (args.Contains("--home", StringComparer.OrdinalIgnoreCase))
        {
            return HomeMode.Run(args);
        }

        // WinExe não tem console próprio; reaproveita o do terminal que chamou.
        AttachConsole(AttachParentProcess);
        return EnvironmentReport.Run();
    }

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool AttachConsole(int processId);
}
