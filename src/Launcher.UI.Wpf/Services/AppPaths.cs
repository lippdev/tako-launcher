using System.IO;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Onde o Tako guarda dados do usuário (biblioteca, cache de artes e metadados). Instalado: <c>%LOCALAPPDATA%\Tako</c>.
/// Portable: com <c>portable.txt</c> ao lado do executável, tudo fica em <c>Data\</c> na própria pasta, para levar num
/// pendrive ou HD externo sem deixar nada no PC.
/// </summary>
public static class AppPaths
{
    public const string PortableMarker = "portable.txt";

    public static bool IsPortable { get; } = File.Exists(Path.Combine(AppContext.BaseDirectory, PortableMarker));

    public static string DataRoot { get; } = IsPortable
        ? Path.Combine(AppContext.BaseDirectory, "Data")
        : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Tako");

    public static string LibraryFile => Path.Combine(DataRoot, "library.json");

    public static string ArtCache => Path.Combine(DataRoot, "cache", "art");

    public static string MetadataCache => Path.Combine(DataRoot, "cache", "metadata");
}
