namespace Launcher.Core.Themes;

/// <summary>Uma pasta de tema com o manifesto lido e validado.</summary>
public sealed record InstalledTheme(string Directory, ThemeManifestResult Manifest)
{
    public string Id => Manifest.Manifest.Id ?? Path.GetFileName(Directory);

    /// <summary>Temas importados numa pasta de temas: subpastas com <c>theme.yaml</c> (pastas ocultas e temporárias ficam de fora).</summary>
    public static IReadOnlyList<InstalledTheme> Scan(string themesRoot)
    {
        if (!System.IO.Directory.Exists(themesRoot))
        {
            return [];
        }

        return System.IO.Directory.EnumerateDirectories(themesRoot)
            .Where(d => !Path.GetFileName(d).StartsWith('.') && File.Exists(Path.Combine(d, ThemeManifest.FileName)))
            .Select(d => new InstalledTheme(d, ThemeManifest.Read(d)))
            .OrderBy(t => t.Manifest.Manifest.DisplayName, StringComparer.CurrentCultureIgnoreCase)
            .ToList();
    }
}
