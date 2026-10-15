namespace Launcher.Core.Library.Metadata.Igdb;

/// <summary>
/// Plataformas do Tako (<see cref="Game.Platform"/>, nula = PC) → ids de plataforma da IGDB (endpoint <c>platforms</c>),
/// para filtrar a busca. Jogos de emulador (EMU-007) informam a plataforma pela chave curta.
/// </summary>
public static class IgdbPlatforms
{
    public const int PcWindows = 6;

    private static readonly Dictionary<string, (string Name, int[] Ids)> Known = new(StringComparer.OrdinalIgnoreCase)
    {
        ["pc"] = ("PC", [PcWindows]),
        ["nes"] = ("NES", [18, 99]), // NES e Famicom
        ["snes"] = ("SNES", [19, 58]), // SNES e Super Famicom
        ["n64"] = ("Nintendo 64", [4]),
        ["gamecube"] = ("GameCube", [21]),
        ["wii"] = ("Wii", [5]),
        ["wiiu"] = ("Wii U", [41]),
        ["switch"] = ("Nintendo Switch", [130]),
        ["gb"] = ("Game Boy", [33]),
        ["gbc"] = ("Game Boy Color", [22]),
        ["gba"] = ("Game Boy Advance", [24]),
        ["nds"] = ("Nintendo DS", [20]),
        ["3ds"] = ("Nintendo 3DS", [37]),
        ["ps1"] = ("PlayStation", [7]),
        ["ps2"] = ("PlayStation 2", [8]),
        ["ps3"] = ("PlayStation 3", [9]),
        ["psp"] = ("PSP", [38]),
        ["psvita"] = ("PS Vita", [46]),
        ["xbox"] = ("Xbox", [11]),
        ["xbox360"] = ("Xbox 360", [12]),
        ["mastersystem"] = ("Master System", [64]),
        ["megadrive"] = ("Mega Drive", [29]),
        ["segacd"] = ("Sega CD", [78]),
        ["32x"] = ("Sega 32X", [30]),
        ["saturn"] = ("Saturn", [32]),
        ["dreamcast"] = ("Dreamcast", [23]),
        ["gamegear"] = ("Game Gear", [35]),
        ["atari2600"] = ("Atari 2600", [59]),
        ["pcengine"] = ("PC Engine", [86]),
        ["neogeo"] = ("Neo Geo", [80]),
        ["arcade"] = ("Arcade", [52]),
    };

    /// <summary>Chaves conhecidas, para o cadastro de emuladores.</summary>
    public static IReadOnlyCollection<string> Keys => Known.Keys;

    /// <summary>Ids da IGDB para a plataforma; nula/vazia = PC. Desconhecida = sem filtro (busca em todas).</summary>
    public static IReadOnlyList<int> IdsFor(string? platform) =>
        string.IsNullOrWhiteSpace(platform) ? [PcWindows] : Known.TryGetValue(platform.Trim(), out var entry) ? entry.Ids : [];

    /// <summary>Nome curto para mostrar ("SNES"), a partir de um id da IGDB.</summary>
    public static string? NameOf(int igdbId) => Known.Values.FirstOrDefault(v => v.Ids.Contains(igdbId)).Name;
}
