using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using Microsoft.Win32;

namespace Launcher.FseProbe;

/// <summary>
/// Tela aberta pelo Windows quando a probe é o gaming home app. Registra em log o ciclo de vida:
/// início, foco, resolução, lançamento e retorno de um processo de teste, encerramento e heartbeat
/// (a ausência de heartbeats mostra quando o processo foi morto sem aviso).
/// Teclas: Enter inicia o processo de teste (padrão: notepad.exe, ou <c>--launch caminho</c>); Esc sai.
/// </summary>
internal static partial class HomeMode
{
    private static readonly string LogPath = Path.Combine(Program.LogDirectory, "home.log");

    public static int Run(string[] args)
    {
        string testTarget = ArgumentAfter(args, "--launch") ?? "notepad.exe";
        Log($"INICIO pid={Environment.ProcessId} args=[{string.Join(' ', args)}] pacote={CurrentPackageFullName() ?? "(sem identidade)"}");

        var app = new Application { ShutdownMode = ShutdownMode.OnMainWindowClose };
        app.SessionEnding += (_, e) => Log($"SESSAO_ENCERRANDO motivo={e.ReasonSessionEnding}");
        app.Exit += (_, e) => Log($"SAIDA codigo={e.ApplicationExitCode}");
        SystemEvents.DisplaySettingsChanged += (_, _) => Log($"RESOLUCAO {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight}");

        var status = new TextBlock
        {
            Foreground = Brushes.White,
            FontSize = 28,
            Margin = new Thickness(64),
            TextWrapping = TextWrapping.Wrap,
            Text = $"Tako FseProbe (modo home)\n\nEnter: iniciar {testTarget}\nEsc: sair\n\nLog: {LogPath}",
        };
        var window = new Window
        {
            Title = "Tako FseProbe",
            WindowStyle = WindowStyle.None,
            ResizeMode = ResizeMode.NoResize,
            WindowState = WindowState.Maximized,
            Background = new SolidColorBrush(Color.FromRgb(0x10, 0x14, 0x1C)),
            Content = status,
        };
        window.Activated += (_, _) => Log("FOCO_GANHO");
        window.Deactivated += (_, _) => Log("FOCO_PERDIDO");
        window.StateChanged += (_, _) => Log($"ESTADO_JANELA {window.WindowState}");
        window.Closed += (_, _) => Log("JANELA_FECHADA");
        window.KeyDown += (_, e) =>
        {
            if (e.Key == Key.Escape)
            {
                window.Close();
            }
            else if (e.Key == Key.Enter)
            {
                LaunchTestProcess(testTarget, window);
            }
        };

        var heartbeat = new DispatcherTimer { Interval = TimeSpan.FromSeconds(5) };
        heartbeat.Tick += (_, _) => Log("HEARTBEAT");
        heartbeat.Start();

        return app.Run(window);
    }

    private static void LaunchTestProcess(string target, Window window)
    {
        try
        {
            var process = Process.Start(new ProcessStartInfo(target) { UseShellExecute = true });
            if (process is null)
            {
                Log($"JOGO_INICIADO {target} (sem processo rastreável)");
                return;
            }

            Log($"JOGO_INICIADO {target} pid={process.Id}");
            process.EnableRaisingEvents = true;
            process.Exited += (_, _) => window.Dispatcher.Invoke(() =>
            {
                Log($"JOGO_ENCERRADO pid={process.Id} codigo={process.ExitCode}");
                bool activated = window.Activate();
                Log($"RETORNO Activate()={activated}");
            });
        }
        catch (Exception ex) when (ex is System.ComponentModel.Win32Exception or InvalidOperationException or FileNotFoundException)
        {
            Log($"JOGO_FALHOU {target}: {ex.Message}");
        }
    }

    private static string? ArgumentAfter(string[] args, string name)
    {
        int index = Array.FindIndex(args, a => string.Equals(a, name, StringComparison.OrdinalIgnoreCase));
        return index >= 0 && index + 1 < args.Length ? args[index + 1] : null;
    }

    private static void Log(string message) =>
        File.AppendAllText(LogPath, $"{DateTimeOffset.Now.ToString("yyyy-MM-dd HH:mm:ss.fff zzz", CultureInfo.InvariantCulture)} {message}{Environment.NewLine}");

    private static unsafe string? CurrentPackageFullName()
    {
        const int AppModelErrorNoPackage = 15700;
        int length = 0;
        if (GetCurrentPackageFullName(ref length, null) == AppModelErrorNoPackage)
        {
            return null;
        }

        var buffer = new char[length];
        fixed (char* pointer = buffer)
        {
            return GetCurrentPackageFullName(ref length, pointer) == 0 ? new string(buffer, 0, length - 1) : null;
        }
    }

    [LibraryImport("kernel32.dll")]
    private static unsafe partial int GetCurrentPackageFullName(ref int packageFullNameLength, char* packageFullName);
}
