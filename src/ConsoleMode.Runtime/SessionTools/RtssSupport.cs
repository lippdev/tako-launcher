// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/RtssService.cs e Services/SessionMenuMath.cs
using System.Globalization;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>Limites de FPS oferecidos na Central de Controle. 0 = sem limite.</summary>
public static class FpsLimits
{
    public static IReadOnlyList<int> Options { get; } = [0, 30, 40, 45, 60, 75, 90, 120, 144, 165];

    /// <summary>Um valor salvo que não está na lista vira o mais próximo (negativo = sem limite).</summary>
    public static int Normalize(int fps) =>
        fps <= 0 ? 0 : Options.Where(o => o > 0).OrderBy(o => Math.Abs(o - fps)).ThenBy(o => o).First();

    /// <summary>Próximo valor para ← → (<paramref name="direction"/> negativo volta), parando nas pontas.</summary>
    public static int Step(int current, int direction)
    {
        int index = Options.ToList().IndexOf(Normalize(current));
        int next = Math.Clamp(index + Math.Sign(direction), 0, Options.Count - 1);
        return Options[next];
    }

    /// <summary>Próximo valor para A, voltando ao começo depois do último.</summary>
    public static int Cycle(int current)
    {
        int index = Options.ToList().IndexOf(Normalize(current));
        return Options[(index + 1) % Options.Count];
    }

    public static string Label(int fps) => fps <= 0 ? "Sem limite" : string.Create(CultureInfo.InvariantCulture, $"{fps} FPS");
}

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
