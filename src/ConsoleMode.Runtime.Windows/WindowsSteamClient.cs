// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/SteamSession.cs
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security;
using ConsoleMode.Runtime.Steam;
using Microsoft.Win32;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Cliente da Steam no Windows: processos <c>steam</c>/<c>steamwebhelper</c>, <c>RunningAppID</c> no registro do
/// usuário e a janela do cliente (fechada com WM_CLOSE para a Steam ir para a bandeja; o processo nunca é encerrado).
/// </summary>
public sealed partial class WindowsSteamClient : ISteamClient
{
    private const string SteamKey = @"Software\Valve\Steam";
    private const uint GwOwner = 4;
    private const uint WmClose = 0x0010;

    public bool IsRunning()
    {
        var processes = SteamProcesses();
        try
        {
            return processes.Length > 0;
        }
        finally
        {
            Dispose(processes);
        }
    }

    public int? RunningAppId()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(SteamKey);
            return SteamClientRules.ParseRunningAppId(key?.GetValue("RunningAppID"));
        }
        catch (Exception ex) when (ex is SecurityException or IOException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    public unsafe bool CloseWindowToTray()
    {
        var processes = SteamProcesses();
        try
        {
            var names = new Dictionary<int, string>();
            foreach (var process in processes)
            {
                names[process.Id] = process.ProcessName;
            }

            nint target = 0;
            const int Capacity = 256;
            char* title = stackalloc char[Capacity];
            char* className = stackalloc char[Capacity];
            foreach (nint window in TopLevelWindows())
            {
                _ = GetWindowThreadProcessId(window, out uint processId);
                if (!names.TryGetValue(unchecked((int)processId), out string? processName))
                {
                    continue;
                }

                int titleLength = GetWindowTextW(window, title, Capacity);
                int classLength = GetClassNameW(window, className, Capacity);
                if (SteamClientRules.IsClientWindowCandidate(
                    processName,
                    new string(title, 0, Math.Max(0, titleLength)),
                    new string(className, 0, Math.Max(0, classLength)),
                    IsWindowVisible(window),
                    GetWindow(window, GwOwner) != 0))
                {
                    target = window;
                    break;
                }
            }

            return target != 0 && PostMessageW(target, WmClose, 0, 0);
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
        {
            return false;
        }
        finally
        {
            Dispose(processes);
        }
    }

    private static Process[] SteamProcesses() => [.. Process.GetProcessesByName("steam"), .. Process.GetProcessesByName("steamwebhelper")];

    private static void Dispose(Process[] processes)
    {
        foreach (var process in processes)
        {
            process.Dispose();
        }
    }

    /// <summary>Janelas de nível superior, na ordem Z (de cima para baixo), via GetWindow sem callback.</summary>
    private static List<nint> TopLevelWindows()
    {
        const uint GwHwndNext = 2;
        const uint GwChild = 5;
        var windows = new List<nint>();
        nint desktop = GetDesktopWindow();
        for (nint window = GetWindow(desktop, GwChild); window != 0 && windows.Count < 4096; window = GetWindow(window, GwHwndNext))
        {
            windows.Add(window);
        }

        return windows;
    }

    [LibraryImport("user32.dll")]
    private static partial nint GetDesktopWindow();

    [LibraryImport("user32.dll")]
    private static partial nint GetWindow(nint window, uint command);

    [LibraryImport("user32.dll")]
    private static partial uint GetWindowThreadProcessId(nint window, out uint processId);

    [LibraryImport("user32.dll")]
    private static unsafe partial int GetWindowTextW(nint window, char* text, int maxCount);

    [LibraryImport("user32.dll")]
    private static unsafe partial int GetClassNameW(nint window, char* className, int maxCount);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool IsWindowVisible(nint window);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool PostMessageW(nint window, uint message, nint wParam, nint lParam);
}
