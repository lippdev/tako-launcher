// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/RtssService.cs
namespace ConsoleMode.Runtime.SessionTools;

/// <summary>
/// Onde o RivaTuner Statistics Server está instalado. Puro: registro e pastas entram como parâmetros.
/// O Tako não distribui o RTSS; sem ele, contador e limite de FPS só explicam como instalar.
/// </summary>
public static class RtssLocator
{
    public const string ExeName = "RTSS.exe";

    /// <summary>Biblioteca do SDK do RTSS que lê e grava perfis (limite de FPS) no processo de quem chama.</summary>
    public const string HooksLibrary = "RTSSHooks64.dll";

    public const string DownloadUrl = "https://www.guru3d.com/download/rtss-rivatuner-statistics-server-download/";

    /// <summary>Pastas onde procurar: a do registro (<c>HKLM\SOFTWARE\WOW6432Node\Unwinder\RTSS\InstallDir</c>) e as padrão.</summary>
    public static IReadOnlyList<string> Folders(string? registryInstallDir, string? programFilesX86, string? programFiles)
    {
        var folders = new List<string>();
        void Add(string? path)
        {
            if (string.IsNullOrWhiteSpace(path))
            {
                return;
            }

            string clean = path.Trim().Trim('"').TrimEnd('\\', '/');
            if (clean.Length > 0 && !folders.Contains(clean, StringComparer.OrdinalIgnoreCase))
            {
                folders.Add(clean);
            }
        }

        Add(registryInstallDir);
        if (!string.IsNullOrWhiteSpace(programFilesX86))
        {
            Add(Path.Combine(programFilesX86, "RivaTuner Statistics Server"));
        }

        if (!string.IsNullOrWhiteSpace(programFiles))
        {
            Add(Path.Combine(programFiles, "RivaTuner Statistics Server"));
        }

        return folders;
    }

    /// <summary>A primeira pasta com o RTSS.exe, ou nulo (não instalado).</summary>
    public static string? Resolve(IEnumerable<string> folders, Func<string, bool> fileExists)
    {
        ArgumentNullException.ThrowIfNull(folders);
        ArgumentNullException.ThrowIfNull(fileExists);
        foreach (string folder in folders)
        {
            if (fileExists(Path.Combine(folder, ExeName)))
            {
                return folder;
            }
        }

        return null;
    }
}
