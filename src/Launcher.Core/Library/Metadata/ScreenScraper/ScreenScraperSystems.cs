namespace Launcher.Core.Library.Metadata.ScreenScraper;

/// <summary>Um sistema (console, portátil, arcade) que o Tako conhece: chave em <see cref="Game.Platform"/>, nome e id no ScreenScraper.</summary>
/// <param name="Key">Chave curta do Tako, a mesma de <see cref="Igdb.IgdbPlatforms"/> ("snes", "ps2").</param>
/// <param name="Name">Nome curto para mostrar ("SNES").</param>
/// <param name="ScreenScraperId">Id do sistema no ScreenScraper (<c>systemeid</c>).</param>
/// <param name="Extensions">Extensões típicas das ROMs/imagens do sistema (minúsculas, com ponto), para sugerir o sistema ao adicionar.</param>
/// <param name="ArchiveIsRom">O .zip é a própria ROM (conjuntos do MAME/Neo Geo): o hash é do .zip, não do arquivo de dentro.</param>
public sealed record EmulatedSystem(string Key, string Name, int ScreenScraperId, IReadOnlyList<string> Extensions, bool ArchiveIsRom = false);

/// <summary>
/// Sistemas do ScreenScraper ↔ <see cref="Game.Platform"/> (ADR-0022). Tabela pura e testada com os sistemas mais comuns;
/// ids conferidos com a lista <c>systemesListe.php</c> da API v2 (os mesmos usados pelo Skyscraper e pelo ES-DE).
/// </summary>
public static class ScreenScraperSystems
{
    /// <summary>Arquivos compactados que podem conter a ROM de qualquer sistema de cartucho.</summary>
    public static readonly IReadOnlyList<string> ArchiveExtensions = [".zip", ".7z"];

    /// <summary>Imagens de disco compactadas: o hash do contêiner não identifica o jogo (só nome e tamanho).</summary>
    public static readonly IReadOnlyList<string> CompressedDiscExtensions = [".chd", ".rvz", ".wia", ".gcz", ".cso", ".wbfs", ".pbp", ".nsz", ".xcz"];

    /// <summary>Em ordem de exibição (o seletor de sistema mostra nesta ordem).</summary>
    public static IReadOnlyList<EmulatedSystem> All { get; } =
    [
        new("nes", "NES", 3, [".nes", ".fds", ".unf", ".unif"]),
        new("snes", "SNES", 4, [".sfc", ".smc", ".fig", ".swc", ".bs"]),
        new("n64", "Nintendo 64", 14, [".n64", ".z64", ".v64"]),
        new("gamecube", "GameCube", 13, [".gcm", ".iso", ".rvz", ".gcz", ".ciso"]),
        new("wii", "Wii", 16, [".wbfs", ".iso", ".rvz", ".wia", ".wad"]),
        new("wiiu", "Wii U", 18, [".wud", ".wux", ".rpx", ".wua"]),
        new("switch", "Nintendo Switch", 225, [".nsp", ".xci", ".nsz", ".xcz"]),
        new("gb", "Game Boy", 9, [".gb"]),
        new("gbc", "Game Boy Color", 10, [".gbc"]),
        new("gba", "Game Boy Advance", 12, [".gba"]),
        new("nds", "Nintendo DS", 15, [".nds", ".dsi"]),
        new("3ds", "Nintendo 3DS", 17, [".3ds", ".cia", ".cci", ".cxi", ".3dsx"]),
        new("mastersystem", "Master System", 2, [".sms"]),
        new("megadrive", "Mega Drive", 1, [".md", ".gen", ".smd", ".bin"]),
        new("segacd", "Sega CD", 20, [".cue", ".chd", ".iso"]),
        new("32x", "Sega 32X", 19, [".32x"]),
        new("gamegear", "Game Gear", 21, [".gg"]),
        new("saturn", "Saturn", 22, [".cue", ".chd", ".iso", ".mdf"]),
        new("dreamcast", "Dreamcast", 23, [".gdi", ".cdi", ".chd"]),
        new("ps1", "PlayStation", 57, [".cue", ".chd", ".pbp", ".iso", ".m3u", ".ecm"]),
        new("ps2", "PlayStation 2", 58, [".iso", ".chd", ".cso"]),
        new("ps3", "PlayStation 3", 59, [".pkg", ".iso"]),
        new("psp", "PSP", 61, [".iso", ".cso", ".pbp", ".chd"]),
        new("psvita", "PS Vita", 62, [".vpk"]),
        new("xbox", "Xbox", 32, [".iso", ".xiso"]),
        new("xbox360", "Xbox 360", 33, [".iso", ".xex"]),
        new("arcade", "Arcade (MAME)", 75, [".zip", ".7z"], ArchiveIsRom: true),
        new("neogeo", "Neo Geo", 142, [".zip", ".7z"], ArchiveIsRom: true),
        new("pcengine", "PC Engine", 31, [".pce", ".sgx"]),
        new("atari2600", "Atari 2600", 26, [".a26"]),
    ];

    private static readonly Dictionary<string, EmulatedSystem> ByKey = All.ToDictionary(s => s.Key, StringComparer.OrdinalIgnoreCase);

    /// <summary>Sistema pela chave do Tako; nulo para PC ou chave desconhecida.</summary>
    public static EmulatedSystem? ForKey(string? key) =>
        string.IsNullOrWhiteSpace(key) ? null : ByKey.GetValueOrDefault(key.Trim());

    /// <summary>Sistema pelo id do ScreenScraper (o primeiro da tabela, quando dois usam o mesmo id).</summary>
    public static EmulatedSystem? ForScreenScraperId(int id) => All.FirstOrDefault(s => s.ScreenScraperId == id);

    /// <summary>
    /// Sistemas prováveis para o arquivo, pela extensão: os que a usam, na ordem da tabela. Um .zip pode ser de qualquer
    /// sistema de cartucho, então volta o arcade, o Neo Geo e os de cartucho. Desconhecida = lista vazia.
    /// </summary>
    public static IReadOnlyList<EmulatedSystem> CandidatesFor(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        string extension = Path.GetExtension(path).ToLowerInvariant();
        if (extension.Length == 0)
        {
            return [];
        }

        var direct = All.Where(s => s.Extensions.Contains(extension)).ToList();
        if (ArchiveExtensions.Contains(extension))
        {
            // Arcade primeiro (o .zip é a ROM), depois os sistemas de cartucho, que costumam vir compactados.
            direct.AddRange(All.Where(s => !direct.Contains(s) && IsCartridge(s)));
        }

        return direct;
    }

    /// <summary>Arquivo com cara de ROM ou imagem de disco (o "Adicionar jogo" pergunta o sistema).</summary>
    public static bool LooksLikeRom(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        string extension = Path.GetExtension(path).ToLowerInvariant();
        return extension.Length > 0 && !NotRoms.Contains(extension) && (All.Any(s => s.Extensions.Contains(extension)) || CompressedDiscExtensions.Contains(extension));
    }

    /// <summary>Filtro do diálogo de arquivo ("*.sfc;*.smc;…").</summary>
    public static string FileDialogPattern() =>
        string.Join(';', All.SelectMany(s => s.Extensions).Concat(CompressedDiscExtensions).Concat(ArchiveExtensions).Distinct().Select(e => "*" + e));

    // Executáveis e atalhos nunca são ROMs, mesmo que algum sistema use a mesma extensão no futuro.
    private static readonly string[] NotRoms = [".exe", ".lnk", ".url", ".bat", ".cmd"];

    // Sistemas de cartucho, cujas ROMs costumam circular compactadas em .zip/.7z.
    private static readonly HashSet<string> Cartridge = new(StringComparer.Ordinal)
    {
        "nes", "snes", "n64", "gb", "gbc", "gba", "nds", "mastersystem", "megadrive", "32x", "gamegear", "pcengine", "atari2600",
    };

    private static bool IsCartridge(EmulatedSystem system) => Cartridge.Contains(system.Key);
}
