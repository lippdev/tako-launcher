using Launcher.Core.Library.Sources;
using Microsoft.Data.Sqlite;

namespace Launcher.Core.Library.Amazon;

/// <summary>Jogo instalado pelo Amazon Games (uma linha da tabela <c>DbSet</c>).</summary>
public sealed record AmazonInstall(string Id, string Title, string InstallDirectory);

/// <summary>
/// Fonte de biblioteca do Amazon Games (STORE-008, ADR-0019): o banco SQLite do cliente
/// (<c>%LOCALAPPDATA%\Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite</c>, tabela <c>DbSet</c>), aberto só para
/// leitura e sem o cliente aberto. Abre sempre pelo protocolo oficial <c>amazon-games://play/&lt;id&gt;</c> (login e
/// DRM ficam com o Amazon Games); o fim da sessão vem dos processos na pasta do jogo.
/// </summary>
public sealed class AmazonLibrarySource(IRegistryReader registry, string localAppData, Func<string, bool>? exists = null) : ILibrarySource
{
    public const string ProtocolKey = @"HKEY_CLASSES_ROOT\amazon-games\shell\open\command";

    public const string MissingNotice = "Amazon Games não encontrado: instale-o para abrir estes jogos";

    private const string Accent = "#B35C00"; // store-amazon (DESIGN.md)

    private readonly Func<string, bool> _exists = exists ?? (path => Directory.Exists(path) || File.Exists(path));

    public GameStore Store => GameStore.Amazon;

    public string DatabasePath => Path.Combine(localAppData, "Amazon Games", "Data", "Games", "Sql", "GameInstallInfo.sqlite");

    /// <summary>O Amazon Games está neste PC: protocolo registrado ou o executável na pasta padrão.</summary>
    public bool ClientInstalled() =>
        registry.GetString(ProtocolKey, string.Empty) is not null
        || _exists(Path.Combine(localAppData, "Amazon Games", "App", "Amazon Games.exe"));

    public LibraryScan Scan()
    {
        bool client = ClientInstalled();
        if (!File.Exists(DatabasePath))
        {
            return client ? new LibraryScan([], ClientInstalled: true) : LibraryScan.Missing("Amazon Games não encontrado neste PC");
        }

        var games = ReadInstalled(DatabasePath)
            .Select(i => i with { InstallDirectory = WindowsPath.Normalize(i.InstallDirectory) })
            .Where(i => _exists(i.InstallDirectory))
            .DistinctBy(i => i.Id, StringComparer.OrdinalIgnoreCase)
            .Select(i => ToGame(i, InstallDates.Of(i.InstallDirectory)))
            .OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase)
            .ToList();
        return new LibraryScan(games, client, client ? null : MissingNotice);
    }

    /// <summary>
    /// Linhas com <c>Installed = 1</c>. Banco inválido ou sem a tabela lança <see cref="IOException"/> (a fonte fica
    /// vazia com aviso, as outras lojas seguem).
    /// </summary>
    public static IReadOnlyList<AmazonInstall> ReadInstalled(string databasePath)
    {
        var builder = new SqliteConnectionStringBuilder { DataSource = databasePath, Mode = SqliteOpenMode.ReadOnly, Pooling = false };
        try
        {
            using var connection = new SqliteConnection(builder.ToString());
            connection.Open();
            using var command = connection.CreateCommand();
            command.CommandText = "SELECT Id, ProductTitle, InstallDirectory FROM DbSet WHERE Installed = 1";
            using var reader = command.ExecuteReader();
            var installs = new List<AmazonInstall>();
            while (reader.Read())
            {
                string? id = reader.IsDBNull(0) ? null : reader.GetString(0);
                string? title = reader.IsDBNull(1) ? null : reader.GetString(1);
                string? folder = reader.IsDBNull(2) ? null : reader.GetString(2);
                if (!string.IsNullOrWhiteSpace(id) && !string.IsNullOrWhiteSpace(folder))
                {
                    installs.Add(new AmazonInstall(id.Trim(), string.IsNullOrWhiteSpace(title) ? WindowsPath.LastName(folder) : title.Trim(), folder));
                }
            }

            return installs;
        }
        catch (SqliteException ex)
        {
            throw new IOException($"Banco do Amazon Games ilegível: {ex.Message}", ex);
        }
    }

    public static string IdFor(string productId) => $"amazon:{productId}";

    /// <summary>Abrir pelo protocolo oficial: <c>amazon-games://play/&lt;id&gt;</c>.</summary>
    public static string LaunchUri(string productId) => $"amazon-games://play/{productId}";

    public static Game ToGame(AmazonInstall install, DateTimeOffset addedAt = default)
    {
        ArgumentNullException.ThrowIfNull(install);
        return new Game
        {
            Id = IdFor(install.Id),
            Title = install.Title,
            Store = GameStore.Amazon,
            Accent = Accent,
            IsInstalled = true,
            AddedAt = addedAt,
            LaunchTarget = LaunchUri(install.Id),
            InstallPath = install.InstallDirectory,
            StoreProductId = install.Id,
        };
    }
}
