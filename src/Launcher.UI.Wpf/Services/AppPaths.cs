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

    /// <summary>
    /// Versão publicada: instalada (marcador) ou portátil (.exe único, sem <see cref="System.Reflection.Assembly.Location"/>).
    /// Rodando pelo <c>dotnet run</c>/Visual Studio, não: o executável não se atualiza e a UI abre sem supervisor.
    /// </summary>
    public static bool IsPublishedBuild => IsInstalled || string.IsNullOrEmpty(typeof(AppPaths).Assembly.Location);

    public static string DataRoot { get; } = IsInstalled
        ? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Tako")
        : Path.Combine(ExecutableDirectory, "Tako_Data");

    /// <summary>Banco SQLite com a biblioteca e as configurações (MVP-005).</summary>
    public static string DatabaseFile => Path.Combine(DataRoot, "tako.db");

    /// <summary>Biblioteca no formato antigo; importada para o <see cref="DatabaseFile"/> na primeira abertura.</summary>
    public static string LibraryFile => Path.Combine(DataRoot, "library.json");

    /// <summary>Logs de diagnóstico <c>tako-AAAAMMDD.log</c> (MVP-033).</summary>
    public static string LogsDirectory => Path.Combine(DataRoot, "logs");

    public static string ArtCache => Path.Combine(DataRoot, "cache", "art");

    public static string MetadataCache => Path.Combine(DataRoot, "cache", "metadata");

    /// <summary>Marcador de sessão em andamento (proteção contra loop de travamento, MVP-036).</summary>
    public static string SessionFile => Path.Combine(DataRoot, "session.json");

    /// <summary>Monitores e áudio registrados antes do jogo, até a restauração (MVP-012).</summary>
    public static string EnvironmentSnapshotFile => Path.Combine(DataRoot, "session-environment.json");

    /// <summary>Marcador de ponteiro escondido pela Central de Controle; existe só enquanto ele está escondido.</summary>
    public static string CursorHiddenMarker => Path.Combine(DataRoot, "cursor-hidden");

    /// <summary>Limite de FPS do RTSS que o usuário tinha antes da sessão, até ser devolvido.</summary>
    public static string RtssLimitBackupFile => Path.Combine(DataRoot, "rtss-fps-limit.txt");
    /// <summary>Chave ADB deste PC para controlar a TV, cifrada com DPAPI (MVP-002).</summary>
    public static string AdbKeyFile => Path.Combine(DataRoot, "adbkey.pem");

    /// <summary>Temas importados de pacotes <c>.pthm</c>, um por pasta (<c>themes/&lt;Id&gt;</c>).</summary>
    public static string ThemesRoot => Path.Combine(DataRoot, "themes");
}
