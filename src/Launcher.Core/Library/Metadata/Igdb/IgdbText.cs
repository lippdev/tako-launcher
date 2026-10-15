namespace Launcher.Core.Library.Metadata.Igdb;

/// <summary>
/// A IGDB só tem nomes em inglês para gêneros, temas e modos; a interface do Tako é em português. Nomes fora da lista
/// passam como vieram.
/// </summary>
public static class IgdbText
{
    private static readonly Dictionary<string, string> Terms = new(StringComparer.OrdinalIgnoreCase)
    {
        // Gêneros
        ["Adventure"] = "Aventura",
        ["Arcade"] = "Arcade",
        ["Card & Board Game"] = "Cartas e tabuleiro",
        ["Fighting"] = "Luta",
        ["Hack and slash/Beat 'em up"] = "Hack and slash",
        ["Indie"] = "Indie",
        ["MOBA"] = "MOBA",
        ["Music"] = "Música",
        ["Pinball"] = "Pinball",
        ["Platform"] = "Plataforma",
        ["Point-and-click"] = "Apontar e clicar",
        ["Puzzle"] = "Quebra-cabeça",
        ["Quiz/Trivia"] = "Perguntas",
        ["Racing"] = "Corrida",
        ["Real Time Strategy (RTS)"] = "Estratégia em tempo real",
        ["Role-playing (RPG)"] = "RPG",
        ["Shooter"] = "Tiro",
        ["Simulator"] = "Simulação",
        ["Sport"] = "Esporte",
        ["Strategy"] = "Estratégia",
        ["Tactical"] = "Tático",
        ["Turn-based strategy (TBS)"] = "Estratégia por turnos",
        ["Visual Novel"] = "Visual novel",

        // Temas
        ["Action"] = "Ação",
        ["Business"] = "Negócios",
        ["Comedy"] = "Comédia",
        ["Drama"] = "Drama",
        ["Educational"] = "Educativo",
        ["Erotic"] = "Erótico",
        ["Fantasy"] = "Fantasia",
        ["Historical"] = "Histórico",
        ["Horror"] = "Terror",
        ["Kids"] = "Infantil",
        ["Mystery"] = "Mistério",
        ["Non-fiction"] = "Não ficção",
        ["Open world"] = "Mundo aberto",
        ["Party"] = "Festa",
        ["Romance"] = "Romance",
        ["Sandbox"] = "Sandbox",
        ["Science fiction"] = "Ficção científica",
        ["Stealth"] = "Furtividade",
        ["Survival"] = "Sobrevivência",
        ["Thriller"] = "Suspense",
        ["Warfare"] = "Guerra",
        ["4X (explore, expand, exploit, and exterminate)"] = "4X",

        // Modos
        ["Single player"] = "Um jogador",
        ["Multiplayer"] = "Multijogador",
        ["Co-operative"] = "Cooperativo",
        ["Split screen"] = "Tela dividida",
        ["Massively Multiplayer Online (MMO)"] = "MMO",
        ["Battle Royale"] = "Battle royale",
    };

    public static string Translate(string term)
    {
        ArgumentNullException.ThrowIfNull(term);
        return Terms.TryGetValue(term.Trim(), out string? pt) ? pt : term.Trim();
    }
}
