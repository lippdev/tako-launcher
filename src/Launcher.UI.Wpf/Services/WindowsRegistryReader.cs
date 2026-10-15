using System.IO;
using System.Security;
using Launcher.Core.Library.Sources;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.Services;

/// <summary>Registro do Windows (visão de 64 bits) para as fontes de biblioteca; chave ausente ou sem permissão = vazio.</summary>
public sealed class WindowsRegistryReader : IRegistryReader
{
    public IReadOnlyList<string> SubKeyNames(string keyPath)
    {
        try
        {
            using var key = Open(keyPath);
            return key?.GetSubKeyNames() ?? [];
        }
        catch (Exception ex) when (ex is SecurityException or UnauthorizedAccessException or IOException)
        {
            return [];
        }
    }

    public string? GetString(string keyPath, string valueName)
    {
        try
        {
            using var key = Open(keyPath);
            return key?.GetValue(valueName) is { } value ? Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture) : null;
        }
        catch (Exception ex) when (ex is SecurityException or UnauthorizedAccessException or IOException)
        {
            return null;
        }
    }

    private static RegistryKey? Open(string keyPath)
    {
        int slash = keyPath.IndexOf('\\', StringComparison.Ordinal);
        string hive = slash < 0 ? keyPath : keyPath[..slash];
        string sub = slash < 0 ? string.Empty : keyPath[(slash + 1)..];
        RegistryHive? which = hive.ToUpperInvariant() switch
        {
            "HKEY_LOCAL_MACHINE" or "HKLM" => RegistryHive.LocalMachine,
            "HKEY_CURRENT_USER" or "HKCU" => RegistryHive.CurrentUser,
            "HKEY_CLASSES_ROOT" or "HKCR" => RegistryHive.ClassesRoot,
            _ => null,
        };
        if (which is null)
        {
            return null;
        }

        using var root = RegistryKey.OpenBaseKey(which.Value, RegistryView.Registry64);
        return sub.Length == 0 ? root.OpenSubKey(string.Empty) : root.OpenSubKey(sub);
    }
}
