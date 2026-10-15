using System.Globalization;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Steam;

namespace Launcher.Core.Library.Sources;

/// <summary>
/// Jogo instalado em mais de uma loja (STORE-010, ADR-0019): cada cópia continua um item próprio, com a loja, o jeito de
/// abrir e o tempo de jogo dela; as cópias só ficam ligadas. Cada uma sabe em que outras lojas o jogo está, e a cópia
/// sem correspondência na Steam herda o appid e as artes da cópia da Steam (capa certa sem esperar a busca por nome).
/// </summary>
public static class StoreDuplicates
{
    /// <summary>Chave de comparação: o título sem edição, ™/®, acentos e pontuação (<see cref="TitleMatcher.Tokens"/>).</summary>
    public static string Key(string title) => string.Join(' ', TitleMatcher.Tokens(title));

    public static void Link(IReadOnlyList<Game> games)
    {
        ArgumentNullException.ThrowIfNull(games);
        foreach (var group in games.GroupBy(g => Key(g.Title), StringComparer.Ordinal))
        {
            var copies = group.ToList();
            bool linked = group.Key.Length > 0 && copies.Select(g => g.Store).Distinct().Count() > 1;
            int? steamAppId = linked ? copies.FirstOrDefault(g => g.Store == GameStore.Steam)?.SteamAppId : null;
            foreach (var game in copies)
            {
                game.AlsoInstalledIn = linked ? [.. copies.Select(g => g.Store).Where(s => s != game.Store).Distinct().Order()] : [];
                if (steamAppId is int appId && game.SteamAppId is null)
                {
                    game.SteamAppId = appId;
                    string id = appId.ToString(CultureInfo.InvariantCulture);
                    game.CoverUri ??= new Uri($"{SteamGames.ArtScheme}://{id}/cover");
                    game.HeroUri ??= new Uri($"{SteamGames.ArtScheme}://{id}/hero");
                }
            }
        }
    }
}
