namespace Launcher.Core.Library.BattleNet;

/// <summary>Um jogo do Battle.net: o id interno (uid no disco), o código de abrir (<c>--exec="launch &lt;código&gt;"</c>) e o nome.</summary>
public sealed record BattleNetProduct(string Uid, string LaunchCode, string Name);

/// <summary>
/// Tabela dos jogos conhecidos do Battle.net. O disco só guarda o uid ("prometheus"); o código de abrir ("Pro") e o nome
/// vêm daqui. Jogo fora da tabela continua na biblioteca (nome da pasta) e abre o Battle.net para o usuário escolher.
/// </summary>
// Adaptado de JosefNemec/PlayniteExtensions: source/Libraries/BattleNetLibrary/Models/BattleNetGames.cs (MIT).
public static class BattleNetProducts
{
    public static readonly IReadOnlyList<BattleNetProduct> All =
    [
        new("wow", "WoW", "World of Warcraft"),
        new("diablo3", "D3", "Diablo III"),
        new("s2", "S2", "StarCraft II"),
        new("s1", "S1", "StarCraft"),
        new("hs_beta", "WTCG", "Hearthstone"),
        new("heroes", "Hero", "Heroes of the Storm"),
        new("prometheus", "Pro", "Overwatch 2"),
        new("viper", "VIPR", "Call of Duty: Black Ops 4"),
        new("odin", "ODIN", "Call of Duty: Modern Warfare"),
        new("w3", "W3", "Warcraft III: Reforged"),
        new("lazarus", "LAZR", "Call of Duty: Modern Warfare 2 Campaign Remastered"),
        new("zeus", "ZEUS", "Call of Duty: Black Ops Cold War"),
        new("wlby", "WLBY", "Crash Bandicoot 4: It's About Time"),
        new("osi", "OSI", "Diablo II: Resurrected"),
        new("rtro", "RTRO", "Blizzard Arcade Collection"),
        new("fore", "FORE", "Call of Duty: Vanguard"),
        new("anbs", "ANBS", "Diablo Immortal"),
        new("auks", "AUKS", "Call of Duty"),
        new("fenris", "Fen", "Diablo IV"),
        new("d1", "D1", "Diablo"),
        new("w1r", "W1R", "Warcraft: Remastered"),
        new("w2r", "W2R", "Warcraft II: Remastered"),
        new("w1", "W1", "Warcraft: Orcs & Humans"),
        new("w2", "W2", "Warcraft II: Battle.net Edition"),
        new("gryphon", "GRY", "Warcraft Rumble"),
        new("aris", "ARIS", "DOOM: The Dark Ages"),
        new("scorpio", "SCOR", "Sea of Thieves"),
        new("arkansas", "ARK", "The Outer Worlds 2"),
        new("libra", "LBRA", "Tony Hawk's Pro Skater 3 + 4"),
        new("pinta", "PNTA", "Call of Duty: Modern Warfare III"),
        new("aqua", "AQUA", "Avowed"),
    ];

    /// <summary>Programas do Battle.net que não são jogos (o próprio cliente e o agente).</summary>
    private static readonly HashSet<string> NotGames = new(StringComparer.OrdinalIgnoreCase) { "battle.net", "agent", "bna", "bts" };

    /// <summary>Idiomas do Battle.net que aparecem no fim do uid ("diablo3_enus").</summary>
    private static readonly HashSet<string> Locales = new(StringComparer.OrdinalIgnoreCase)
    {
        "enus", "engb", "dede", "eses", "esmx", "frfr", "itit", "kokr", "plpl", "ptbr", "ptpt", "ruru", "zhcn", "zhtw", "jajp", "thth",
    };

    public static bool IsClient(string uid) => NotGames.Contains(uid);

    /// <summary>
    /// O jogo do uid: igual ao da tabela ou o da tabela seguido de idioma ("diablo3_enus"). Outras variações
    /// ("wow_classic", "wow_beta") não herdam o código, para não abrir o jogo errado.
    /// </summary>
    public static BattleNetProduct? Find(string uid)
    {
        ArgumentNullException.ThrowIfNull(uid);
        foreach (var product in All)
        {
            if (string.Equals(uid, product.Uid, StringComparison.OrdinalIgnoreCase))
            {
                return product;
            }

            if (uid.Length == product.Uid.Length + 5
                && uid.StartsWith(product.Uid + "_", StringComparison.OrdinalIgnoreCase)
                && Locales.Contains(uid[^4..]))
            {
                return product;
            }
        }

        return null;
    }
}
