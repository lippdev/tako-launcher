using System.IO;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Onde o Tako guarda dados do usuário (biblioteca, cache de artes e metadados), no mesmo modelo do Console Mode:
/// instalado (o instalador grava <c>Tako.installed</c> ao lado do executável) usa <c>%LOCALAPPDATA%\Tako</c>; sem o
/// marcador (portable, um único .exe) usa a pasta <c>Tako_Data</c> ao lado do arquivo.
/// </summary>
public static class AppPaths
{
    public const string InstalledMarker = "Tako.installed";

    /// <summary>Pasta do executável real (no .exe único, <see cref="AppContext.BaseDirectory"/> é a pasta de extração).</summary>
    public static string ExecutableDirectory { get; } = Path.GetDirectoryName(Environment.ProcessPath) ?? AppContext.BaseDirectory;

    public static bool IsInstalled { get; } = File.Exists(Path.Combine(ExecutableDirectory, InstalledMarker));

    public static string DataRoot { get; } = IsInstalled
        ? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Tako")
        : Path.Combine(ExecutableDirectory, "Tako_Data");

    public static string LibraryFile => Path.Combine(DataRoot, "library.json");

    public static string ArtCache => Path.Combine(DataRoot, "cache", "art");

    public static string MetadataCache => Path.Combine(DataRoot, "cache", "metadata");
}
