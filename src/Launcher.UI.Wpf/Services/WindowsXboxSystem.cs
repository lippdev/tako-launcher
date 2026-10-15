using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Launcher.Core.Library.Xbox;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Acesso real ao Windows para a fonte Xbox (STORE-004), sem WinRT nem administrador: pacotes do usuário pelo
/// repositório do AppModel no registro (HKCU) com a pasta pela API <c>GetPackagePathByFullName</c>, jogos dos Serviços
/// de Jogos pelo registro (HKLM, só leitura) e nomes <c>ms-resource:</c> por <c>SHLoadIndirectString</c>.
/// </summary>
public sealed partial class WindowsXboxSystem : IXboxSystem
{
    private const string RepositoryPackages = @"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";
    private const string GamingServicesRepository = @"SOFTWARE\Microsoft\GamingServices\PackageRepository";
    private const int ErrorInsufficientBuffer = 122;

    public IReadOnlyList<string> DriveRoots() =>
        [.. DriveInfo.GetDrives().Where(d => d.DriveType == DriveType.Fixed && d.IsReady).Select(d => d.RootDirectory.FullName)];

    public IReadOnlyList<InstalledPackage>? UserPackages()
    {
        using var key = Registry.CurrentUser.OpenSubKey(RepositoryPackages);
        if (key is null)
        {
            return null;
        }

        var packages = new List<InstalledPackage>();
        foreach (string fullName in key.GetSubKeyNames())
        {
            string? folder = PackagePath(fullName);
            if (folder is null)
            {
                using var package = key.OpenSubKey(fullName);
                folder = package?.GetValue("PackageRootFolder") as string;
            }

            if (!string.IsNullOrWhiteSpace(folder))
            {
                packages.Add(new InstalledPackage(fullName, folder));
            }
        }

        return packages;
    }

    public IReadOnlyCollection<string> GamingServicesPackages()
    {
        // PackageRepository\Package\<nome completo> e PackageRepository\Root\<guid>\<nome completo>.
        using var repository = Registry.LocalMachine.OpenSubKey(GamingServicesRepository);
        if (repository is null)
        {
            return [];
        }

        var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        using (var packages = repository.OpenSubKey("Package"))
        {
            names.UnionWith(packages?.GetSubKeyNames() ?? []);
        }

        using var roots = repository.OpenSubKey("Root");
        foreach (string root in roots?.GetSubKeyNames() ?? [])
        {
            using var entry = roots!.OpenSubKey(root);
            names.UnionWith(entry?.GetSubKeyNames() ?? []);
        }

        return names;
    }

    public bool GamingServicesInstalled()
    {
        using var service = Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Services\GamingServices");
        return service is not null;
    }

    public string? LoadIndirectString(string indirect)
    {
        var buffer = new char[1024];
        int result = SHLoadIndirectString(indirect, buffer, buffer.Length, IntPtr.Zero);
        if (result != 0)
        {
            return null;
        }

        int length = Array.IndexOf(buffer, '\0');
        return new string(buffer, 0, length < 0 ? buffer.Length : length);
    }

    private static string? PackagePath(string fullName)
    {
        uint length = 0;
        int result = GetPackagePathByFullName(fullName, ref length, null);
        if (result != ErrorInsufficientBuffer || length == 0)
        {
            return null;
        }

        var buffer = new char[length];
        result = GetPackagePathByFullName(fullName, ref length, buffer);
        return result == 0 ? new string(buffer, 0, (int)Math.Max(0, length - 1)) : null;
    }

    [LibraryImport("kernel32.dll", StringMarshalling = StringMarshalling.Utf16)]
    private static partial int GetPackagePathByFullName(string packageFullName, ref uint pathLength, [Out] char[]? path);

    [LibraryImport("shlwapi.dll", StringMarshalling = StringMarshalling.Utf16)]
    private static partial int SHLoadIndirectString(string pszSource, [Out] char[] pszOutBuf, int cchOutBuf, IntPtr ppvReserved);
}
