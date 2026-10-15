using Launcher.Core.Presentation;

namespace Launcher.Core.Library.Sources;

/// <summary>Como uma loja ficou na última leitura (Configurações › Lojas).</summary>
public sealed record LibrarySourceReport(GameStore Store, bool Enabled, int GameCount, bool ClientInstalled, string? Notice, bool Failed)
{
    /// <summary>Linha de ajuda nas Configurações, em pt-BR.</summary>
    public string Summary
    {
        get
        {
            string name = GameText.StoreOf(Store);
            if (!Enabled)
            {
                return $"Desligada: os jogos {name} não aparecem na biblioteca";
            }

            if (Failed)
            {
                return $"Não foi possível ler os jogos {name}; veja a pasta de logs";
            }

            string found = GameCount switch
            {
                0 => "Nenhum jogo instalado encontrado",
                1 => "1 jogo instalado",
                _ => $"{GameCount} jogos instalados",
            };
            // Sem o cliente, o aviso vem primeiro (é o que o usuário precisa resolver).
            return Notice is null ? found : ClientInstalled ? $"{found} · {Notice}" : $"{Notice} · {found}";
        }
    }
}

/// <summary>
/// Junta as fontes de biblioteca (ADR-0019): lê só as lojas ligadas nas Configurações, isola a falha de cada uma e
/// guarda o resultado para as Configurações (<see cref="Reports"/>).
/// </summary>
public sealed class LibrarySources(IReadOnlyList<ILibrarySource> sources, Func<GameStore, bool> isEnabled, Action<GameStore, Exception>? onFailure = null)
{
    private readonly Dictionary<GameStore, LibrarySourceReport> _reports = [];

    /// <summary>Lojas registradas, na ordem de leitura.</summary>
    public IReadOnlyList<GameStore> Stores => [.. sources.Select(s => s.Store)];

    /// <summary>Última leitura de cada loja; ausente = ainda não lida.</summary>
    public IReadOnlyDictionary<GameStore, LibrarySourceReport> Reports => _reports;

    /// <summary>Jogos de todas as lojas ligadas (ids distintos; o primeiro vence).</summary>
    public IReadOnlyList<Game> Discover()
    {
        var games = new List<Game>();
        var ids = new HashSet<string>(StringComparer.Ordinal);
        foreach (var source in sources)
        {
            if (!isEnabled(source.Store))
            {
                _reports[source.Store] = new LibrarySourceReport(source.Store, Enabled: false, 0, ClientInstalled: false, Notice: null, Failed: false);
                continue;
            }

            LibraryScan scan;
            try
            {
                scan = source.Scan();
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or System.Security.SecurityException or System.Text.Json.JsonException)
            {
                onFailure?.Invoke(source.Store, ex);
                _reports[source.Store] = new LibrarySourceReport(source.Store, Enabled: true, 0, ClientInstalled: false, Notice: null, Failed: true);
                continue;
            }

            int added = 0;
            foreach (var game in scan.Games)
            {
                if (ids.Add(game.Id))
                {
                    games.Add(game);
                    added++;
                }
            }

            _reports[source.Store] = new LibrarySourceReport(source.Store, Enabled: true, added, scan.ClientInstalled, scan.Notice, Failed: false);
        }

        return games;
    }
}
