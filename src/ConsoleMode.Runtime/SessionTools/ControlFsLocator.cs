// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/ControlFsLocator.cs
namespace ConsoleMode.Runtime.SessionTools;

/// <summary>
/// Onde o ControlFS (explorador de arquivos para controle, da mesma equipe) está instalado. Puro: registro e disco
/// entram como parâmetros, então a ordem de preferência é testável.
/// </summary>
public static class ControlFsLocator
{
    public const string ExeName = "ControlFS.exe";

    public const string ReleasesUrl = "https://github.com/nextestudios/ControlFS/releases/latest";

    /// <summary>Pastas onde procurar, da mais confiável para a menos, sem vazias nem repetidas.</summary>
    public static IReadOnlyList<string> Folders(string? registryInstallLocation, string? localAppData, string? programFiles)
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

        Add(registryInstallLocation);
        if (!string.IsNullOrWhiteSpace(localAppData))
        {
            Add(Path.Combine(localAppData, "Programs", "ControlFS"));
        }

        if (!string.IsNullOrWhiteSpace(programFiles))
        {
            Add(Path.Combine(programFiles, "ControlFS"));
        }

        return folders;
    }

    /// <summary>
    /// Tela cheia é a janela exatamente do tamanho da tela. Janela maximizada passa uns 16 px (a moldura sobra nas
    /// bordas) e não conta; um pixel de tolerância cobre arredondamentos.
    /// </summary>
    public static bool IsFullScreen(int left, int top, int width, int height, int screenX, int screenY, int screenWidth, int screenHeight) =>
        screenWidth > 0 && screenHeight > 0
        && Math.Abs(left - screenX) <= 1 && Math.Abs(top - screenY) <= 1
        && Math.Abs(width - screenWidth) <= 1 && Math.Abs(height - screenHeight) <= 1;

    /// <summary>A primeira pasta que tem mesmo o ControlFS.exe, ou nulo (não instalado).</summary>
    public static string? Resolve(IEnumerable<string> folders, Func<string, bool> fileExists)
    {
        ArgumentNullException.ThrowIfNull(folders);
        ArgumentNullException.ThrowIfNull(fileExists);
        foreach (string folder in folders)
        {
            string exe = Path.Combine(folder, ExeName);
            if (fileExists(exe))
            {
                return exe;
            }
        }

        return null;
    }
}
