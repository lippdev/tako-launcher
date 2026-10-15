// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/ControlFsService.cs
using System.ComponentModel;
using System.Diagnostics;
using System.Security;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;
using Microsoft.Win32;
using static ConsoleMode.Runtime.Windows.SessionTools.SessionToolsNative;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Abre o ControlFS pela Central de Controle. O ControlFS é um app à parte: encontrado como o Windows o lista,
/// iniciado com o próprio <c>--start</c> (abre, ou traz para a frente o que já está aberto) e nunca encerrado daqui.
/// </summary>
public sealed partial class WindowsControlFs(IWindowSwitcher windows, ILogger<WindowsControlFs> logger) : IControlFsService
{
    public bool IsInstalled => FindExe() is not null;

    public bool Open()
    {
        string? exe = FindExe();
        if (exe is null)
        {
            return false;
        }

        try
        {
            // Já aberto: traz aquela janela. Abrir de novo pode deixar uma segunda cópia rodando.
            nint open = FindOpenWindow();
            if (open != 0)
            {
                ShowWindow(open, SwRestore);
                return windows.Activate(open);
            }

            var start = new ProcessStartInfo { FileName = exe, UseShellExecute = false, WorkingDirectory = Path.GetDirectoryName(exe)! };
            start.ArgumentList.Add("--start");
            Process.Start(start)?.Dispose();
            LogOpened(logger);
            return true;
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException or IOException)
        {
            LogOpenFailed(logger, ex);
            return false;
        }
    }

    private string? FindExe()
    {
        try
        {
            var folders = ControlFsLocator.Folders(
                ReadInstallLocation(),
                Environment.GetEnvironmentVariable("LOCALAPPDATA"),
                Environment.GetEnvironmentVariable("ProgramFiles"));
            return ControlFsLocator.Resolve(folders, File.Exists);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or SecurityException)
        {
            LogLookupFailed(logger, ex);
            return null;
        }
    }

    private static nint FindOpenWindow()
    {
        foreach (var process in Process.GetProcessesByName("ControlFS"))
        {
            using (process)
            {
                process.Refresh();
                nint hwnd = process.MainWindowHandle;
                if (hwnd != 0 && IsWindow(hwnd) && IsWindowVisible(hwnd))
                {
                    return hwnd;
                }
            }
        }

        return 0;
    }

    private static string? ReadInstallLocation()
    {
        const string Uninstall = @"Software\Microsoft\Windows\CurrentVersion\Uninstall";
        foreach (var (hive, view) in new[] { (RegistryHive.CurrentUser, RegistryView.Default), (RegistryHive.LocalMachine, RegistryView.Registry64), (RegistryHive.LocalMachine, RegistryView.Registry32) })
        {
            try
            {
                using var root = RegistryKey.OpenBaseKey(hive, view).OpenSubKey(Uninstall);
                if (root is null)
                {
                    continue;
                }

                foreach (string name in root.GetSubKeyNames())
                {
                    using var key = root.OpenSubKey(name);
                    if (key?.GetValue("DisplayName") is string display
                        && display.Equals("ControlFS", StringComparison.OrdinalIgnoreCase)
                        && key.GetValue("InstallLocation") is string location
                        && !string.IsNullOrWhiteSpace(location))
                    {
                        return location;
                    }
                }
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or SecurityException)
            {
                // Esta parte do registro não pode ser lida: tenta a próxima.
            }
        }

        return null;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "ControlFS aberto pela Central de Controle")]
    private static partial void LogOpened(ILogger logger);

    [LoggerMessage(Level = LogLevel.Warning, Message = "ControlFS não pôde ser aberto")]
    private static partial void LogOpenFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível procurar o ControlFS")]
    private static partial void LogLookupFailed(ILogger logger, Exception exception);
}
