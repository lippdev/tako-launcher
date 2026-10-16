using System.Globalization;
using Launcher.Core.Library.Sources;

namespace Launcher.Core.Library.Ubisoft;

/// <summary>
/// Fonte de biblioteca da Ubisoft Connect (STORE-005, ADR-0019): jogos instalados lidos do registro
/// (<see cref="InstallsKeys"/>, <c>InstallDir</c> por id), sem o cliente aberto. O nome vem da chave de desinstalação
/// <c>Uplay Install &lt;id&gt;</c> ou, sem ela, da pasta. Abre sempre pelo protocolo oficial (<see cref="LaunchUri"/>):
/// a Ubisoft Connect faz login, DRM e atualização. O fim da sessão vem dos processos na pasta do jogo.
/// </summary>
public sealed class UbisoftLibrarySource(IRegistryReader registry, Func<string, bool>? exists = null) : ILibrarySource
{
    /// <summary>Jogos instalados (o cliente é 32 bits; a chave sem WOW6432Node fica como plano B).</summary>
    public static readonly IReadOnlyList<string> InstallsKeys =
    [
        @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs",
        @"HKEY_LOCAL_MACHINE\SOFTWARE\Ubisoft\Launcher\Installs",
    ];

    /// <summary>Pasta da Ubisoft Connect (<c>InstallDir</c>).</summary>
    public const string LauncherKey = @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Ubisoft\Launcher";

    /// <summary>Protocolo <c>uplay://</c>: sem ele, nada abre os jogos.</summary>
    public const string ProtocolKey = @"HKEY_CLASSES_ROOT\uplay\shell\open\command";

    public const string MissingNotice = "Ubisoft Connect não encontrada: instale-a para abrir estes jogos";

    private const string Accent = "#2E3192"; // store-ubisoft (DESIGN.md)

    private readonly Func<string, bool> _exists = exists ?? Directory.Exists;

    public GameStore Store => GameStore.Ubisoft;

    /// <summary>A Ubisoft Connect está neste PC (protocolo registrado ou executável na pasta do cliente).</summary>
    public bool ClientInstalled()
    {
        if (registry.GetString(ProtocolKey, string.Empty) is not null)
        {
            return true;
        }

        string? folder = registry.GetString(LauncherKey, "InstallDir");
        return !string.IsNullOrWhiteSpace(folder)
            && (_exists(WindowsPath.Combine(folder, "UbisoftConnect.exe")) || _exists(WindowsPath.Combine(folder, "upc.exe")));
    }

    public LibraryScan Scan()
    {
        bool client = ClientInstalled();
        var games = new List<Game>();
        foreach (string root in InstallsKeys)
        {
            foreach (string id in registry.SubKeyNames(root))
            {
                string? folder = registry.GetString($@"{root}\{id}", "InstallDir");
                if (string.IsNullOrWhiteSpace(folder) || !int.TryParse(id, NumberStyles.None, CultureInfo.InvariantCulture, out _))
                {
                    continue;
                }

                string path = WindowsPath.Normalize(folder);
                if (!_exists(path) || games.Exists(g => g.Id == IdFor(id)))
                {
                    continue; // desinstalado sem limpar o registro, ou já lido na outra visão
                }

                games.Add(ToGame(id, TitleFor(id, path), path, InstallDates.Of(path)));
            }
        }

        if (games.Count == 0 && !client)
        {
            return LibraryScan.Missing("Ubisoft Connect não encontrada neste PC");
        }

        return new LibraryScan(
            [.. games.OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase)],
            client,
            client ? null : MissingNotice);
    }

    /// <summary>Nome do programa instalado ("Uplay Install 635" → "Far Cry 5"); sem ele, o nome da pasta.</summary>
    public string TitleFor(string id, string folder)
    {
        string? name = UninstallEntries.Find(registry, $"Uplay Install {id}")?.DisplayName;
        return string.IsNullOrWhiteSpace(name) ? WindowsPath.LastName(folder) : name;
    }

    public static string IdFor(string installId) => $"ubisoft:{installId}";

    /// <summary>Abrir pelo protocolo oficial: <c>uplay://launch/&lt;id&gt;/0</c> (0 = a primeira opção de jogar).</summary>
    public static string LaunchUri(string installId) => $"uplay://launch/{installId}/0";

    public static Game ToGame(string installId, string title, string folder, DateTimeOffset addedAt = default) => new()
    {
        Id = IdFor(installId),
        Title = title.Trim(),
        Store = GameStore.Ubisoft,
        Accent = Accent,
        IsInstalled = true,
        AddedAt = addedAt,
        LaunchTarget = LaunchUri(installId),
        InstallPath = folder,
        StoreProductId = installId,
    };
}
