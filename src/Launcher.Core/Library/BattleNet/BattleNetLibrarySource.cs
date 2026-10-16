using System.Text.RegularExpressions;
using Launcher.Core.Library.Sources;

namespace Launcher.Core.Library.BattleNet;

/// <summary>
/// Fonte de biblioteca do Battle.net (STORE-007, ADR-0019). Lê o <c>product.db</c> do agente (protobuf, só os campos
/// necessários) e, se ele faltar ou não abrir, as chaves de desinstalação dos jogos (<c>--uid=</c>). Abre sempre pelo
/// cliente: <c>Battle.net.exe --exec="launch &lt;código&gt;"</c>; jogo sem código conhecido abre o Battle.net para o
/// usuário escolher (STORE-011). O fim da sessão vem dos processos na pasta do jogo.
/// </summary>
public sealed partial class BattleNetLibrarySource(
    IRegistryReader registry,
    string programData,
    Func<string, byte[]?>? readBytes = null,
    Func<string, bool>? exists = null,
    string? programFilesX86 = null) : ILibrarySource
{
    public const string MissingNotice = "Battle.net não encontrado: instale-o para abrir estes jogos";

    private const string Accent = "#0B74DE"; // store-battlenet (DESIGN.md)

    private readonly Func<string, byte[]?> _readBytes = readBytes ?? (path => File.Exists(path) ? File.ReadAllBytes(path) : null);
    private readonly Func<string, bool> _exists = exists ?? (path => Directory.Exists(path) || File.Exists(path));

    public GameStore Store => GameStore.BattleNet;

    public string ProductDbPath => WindowsPath.Combine(programData, "Battle.net", "Agent", "product.db");

    /// <summary><c>Battle.net.exe</c>: pela chave de desinstalação do cliente (<c>--uid=battle.net</c>) ou na pasta padrão.</summary>
    public string? ClientExecutable(IReadOnlyList<UninstallEntry>? entries = null)
    {
        entries ??= UninstallEntries.Read(registry);
        var candidates = entries
            .Where(e => e.InstallLocation is not null && UidOf(e.UninstallString) is { } uid && string.Equals(uid, "battle.net", StringComparison.OrdinalIgnoreCase))
            .Select(e => WindowsPath.Combine(e.InstallLocation!, "Battle.net.exe"))
            .ToList();
        if (!string.IsNullOrWhiteSpace(programFilesX86))
        {
            candidates.Add(WindowsPath.Combine(programFilesX86, "Battle.net", "Battle.net.exe"));
        }

        return candidates.FirstOrDefault(_exists);
    }

    public LibraryScan Scan()
    {
        var entries = UninstallEntries.Read(registry);
        string? client = ClientExecutable(entries);
        var installs = ReadProductDb() ?? FromUninstall(entries);
        var games = new List<Game>();
        foreach (var install in installs)
        {
            string path = WindowsPath.Normalize(install.InstallPath);
            if (BattleNetProducts.IsClient(install.Uid) || install.Installed == false || !_exists(path)
                || games.Exists(g => g.Id == IdFor(install.Uid)))
            {
                continue;
            }

            var product = BattleNetProducts.Find(install.Uid);
            string title = product?.Name ?? DisplayNameOf(entries, install.Uid) ?? WindowsPath.LastName(path);
            games.Add(ToGame(install.Uid, product, title, path, client, InstallDates.Of(path)));
        }

        if (games.Count == 0 && client is null)
        {
            return LibraryScan.Missing("Battle.net não encontrado neste PC");
        }

        return new LibraryScan(
            [.. games.OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase)],
            client is not null,
            client is null ? MissingNotice : null);
    }

    public static string IdFor(string uid) => $"battlenet:{uid.ToLowerInvariant()}";

    /// <summary>
    /// Pelo cliente: <c>--exec="launch Pro"</c>; sem código conhecido, só abre o Battle.net. Sem o cliente, o
    /// protocolo <c>battlenet://</c> (falha ao abrir e mostra o aviso de cliente ausente).
    /// </summary>
    public static Game ToGame(string uid, BattleNetProduct? product, string title, string folder, string? client, DateTimeOffset addedAt = default) => new()
    {
        Id = IdFor(uid),
        Title = title.Trim(),
        Store = GameStore.BattleNet,
        Accent = Accent,
        IsInstalled = true,
        AddedAt = addedAt,
        LaunchTarget = client ?? $"battlenet://{product?.LaunchCode ?? string.Empty}",
        LaunchArguments = client is not null && product is not null ? $"--exec=\"launch {product.LaunchCode}\"" : null,
        InstallPath = folder,
        StoreProductId = product?.LaunchCode,
    };

    /// <summary>O uid no comando de desinstalação: <c>"…\Blizzard Uninstaller.exe" --lang=enUS --uid=prometheus …</c>.</summary>
    public static string? UidOf(string? uninstallString) =>
        uninstallString is not null && UidPattern().Match(uninstallString) is { Success: true } match ? match.Groups[1].Value : null;

    private IReadOnlyList<BattleNetInstall>? ReadProductDb()
    {
        try
        {
            byte[]? data = _readBytes(ProductDbPath);
            return data is null ? null : ProductDb.Parse(data);
        }
        catch (InvalidDataException)
        {
            return null; // formato novo ou arquivo sendo gravado: o registro dá conta
        }
    }

    /// <summary>Plano B: os jogos que o Battle.net registra como programas instalados (sem testes e betas).</summary>
    private static List<BattleNetInstall> FromUninstall(IReadOnlyList<UninstallEntry> entries) =>
        [.. entries
            .Where(e => e.InstallLocation is not null
                && e.UninstallString?.Contains("Battle.net", StringComparison.OrdinalIgnoreCase) == true
                && e.DisplayName?.EndsWith("Test", StringComparison.OrdinalIgnoreCase) != true
                && e.DisplayName?.EndsWith("Beta", StringComparison.OrdinalIgnoreCase) != true)
            .Select(e => (Entry: e, Uid: UidOf(e.UninstallString)))
            .Where(x => x.Uid is not null)
            .Select(x => new BattleNetInstall(x.Uid!, null, x.Entry.InstallLocation!, Installed: true))];

    private static string? DisplayNameOf(IReadOnlyList<UninstallEntry> entries, string uid) =>
        entries.FirstOrDefault(e => string.Equals(UidOf(e.UninstallString), uid, StringComparison.OrdinalIgnoreCase))?.DisplayName;

    [GeneratedRegex(@"--uid=""?([^""\s]+)", RegexOptions.IgnoreCase)]
    private static partial Regex UidPattern();
}
