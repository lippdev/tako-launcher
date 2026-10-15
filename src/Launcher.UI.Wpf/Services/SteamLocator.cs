using System.IO;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.Services;

/// <summary>Pasta de instalação da Steam, pelo registro (a Steam grava o caminho ao instalar).</summary>
public static class SteamLocator
{
    public static string? FindRoot()
    {
        string?[] candidates =
        [
            Registry.GetValue(@"HKEY_CURRENT_USER\Software\Valve\Steam", "SteamPath", null) as string,
            Registry.GetValue(@"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath", null) as string,
            Registry.GetValue(@"HKEY_LOCAL_MACHINE\SOFTWARE\Valve\Steam", "InstallPath", null) as string,
        ];
        return candidates
            .Where(p => !string.IsNullOrWhiteSpace(p))
            .Select(p => Path.GetFullPath(p!.Replace('/', '\\')))
            .FirstOrDefault(Directory.Exists);
    }
}
