using System.IO;
using System.Net.Http;
using System.Text.Json;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Igdb;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Agente de metadados (como no Plex): para cada jogo, junta os dados das fontes ligadas — Steam Store sempre, IGDB
/// com a chave do usuário (ADR-0021) — pela prioridade do tipo de jogo (<see cref="MetadataAgent"/>) e guarda em
/// <c>%LOCALAPPDATA%\Tako\cache\metadata</c>. O cache é aplicado na hora; a atualização roda em segundo plano, um jogo
/// por vez, respeitando o limite das APIs. "Corrigir correspondência" fixa a escolha do usuário.
/// </summary>
public sealed partial class MetadataService(IEnumerable<IMetadataProvider> providers, IgdbSettingsStore igdb, HttpClient http, TimeProvider time, ILogger<MetadataService> logger)
{
    /// <summary>Espaço entre jogos, pelo limite da loja Steam (~200 consultas a cada 5 minutos).</summary>
    private static readonly TimeSpan Spacing = TimeSpan.FromSeconds(1.5);
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    private readonly MetadataAgent _agent = new(providers, time);
    private readonly string _directory = AppPaths.MetadataCache;
    private bool _running;
    private bool _pending;
    private bool _pendingForce;

    /// <summary>Metadados novos para o jogo (id); disparado na thread de UI.</summary>
    public event EventHandler<string>? GameUpdated;

    /// <summary>Fontes ligadas agora ("Steam" ou "Steam e IGDB").</summary>
    public string ProviderName => _agent.EnabledNames;

    /// <summary>A IGDB está ligada e com chave.</summary>
    public bool IgdbEnabled => _agent.IsEnabled(MetadataPriority.Igdb);

    /// <summary>IGDB ligada ou não e a chave da Twitch (Configurações › Dados dos jogos).</summary>
    public IgdbSettings IgdbSettings => igdb.Current;

    /// <summary>Salva a configuração da IGDB; ligada, os jogos ganham os dados dela aos poucos, em segundo plano.</summary>
    public void SaveIgdb(IgdbSettings settings) => igdb.Save(settings);

    /// <summary>Botão "Testar": token da Twitch e uma consulta mínima com a chave informada (ainda não salva).</summary>
    /// <exception cref="IgdbException">Com o motivo, para a mensagem (<see cref="IgdbErrors"/>).</exception>
    public Task TestIgdbAsync(IgdbSettings settings, CancellationToken cancellationToken) => IgdbTester(settings, cancellationToken);

    /// <summary>O teste de fumaça troca por um simulado: o CI nunca fala com a Twitch.</summary>
    internal Func<IgdbSettings, CancellationToken, Task> IgdbTester { get; set; } = (settings, token) => IgdbClient.TestAsync(http, settings, time, token);

    /// <summary>Aplica o que já está em cache (chamado pela biblioteca ao montar cada jogo).</summary>
    public void ApplyCached(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        Read(game.Id)?.ApplyTo(game);
    }

    /// <summary>De onde vieram os dados do jogo, para "Opções" ("Steam (appid 570) + IGDB").</summary>
    public string Describe(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        var cached = MetadataAgent.Normalize(Read(game.Id));
        if (cached is null || cached.IsEmpty)
        {
            return game.SteamAppId is int own ? $"Steam (appid {own})" : "nenhuma correspondência ainda";
        }

        var names = cached.Parts.Select(p => p.Source == SteamStoreProvider.SourceName && p.SteamAppId is int id ? $"Steam (appid {id})" : p.Source);
        return string.Join(" + ", names) + (cached.IsPinned ? " · escolhida por você" : string.Empty);
    }

    /// <summary>Jogo sem descrição nem correspondência, que poderia ganhar dados com a IGDB ligada.</summary>
    public bool CouldUseIgdb(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return !igdb.Current.IsActive && game.Store != GameStore.Steam && string.IsNullOrWhiteSpace(game.Description);
    }

    /// <summary>Atualiza em segundo plano os jogos sem dados ou com dados antigos; <paramref name="force"/> busca todos.</summary>
    public async Task RefreshAsync(Func<IReadOnlyList<Game>> games, bool force = false)
    {
        if (_running)
        {
            _pending = true; // a rodada atual termina e roda de novo
            _pendingForce |= force;
            return;
        }

        _running = true;
        try
        {
            do
            {
                _pending = false;
                var due = games().Where(g => _agent.NeedsUpdate(g, Read(g.Id), force)).ToList();
                if (due.Count > 0)
                {
                    // IGDB: a biblioteca inteira em poucas consultas em lote, antes de ir jogo a jogo.
                    await _agent.PrefetchAsync(due, CancellationToken.None).ConfigureAwait(true);
                }

                foreach (var game in due)
                {
                    if (await UpdateAsync(game, force).ConfigureAwait(true))
                    {
                        await Task.Delay(Spacing).ConfigureAwait(true);
                    }
                }

                force = _pendingForce;
                _pendingForce = false;
            }
            while (_pending);
        }
        finally
        {
            _running = false;
        }
    }

    /// <summary>"Corrigir correspondência": resultados de todas as fontes ligadas.</summary>
    public Task<IReadOnlyList<MetadataMatch>> SearchAsync(Game game) => _agent.SearchAsync(game, CancellationToken.None);

    /// <summary>Correspondência escolhida pelo usuário: busca os dados dela (e das outras fontes, pelos ids) e fixa a escolha.</summary>
    public async Task<bool> PinAsync(Game game, MetadataMatch match)
    {
        ArgumentNullException.ThrowIfNull(game);
        var metadata = await _agent.PinAsync(game, match, CancellationToken.None).ConfigureAwait(true);
        if (metadata is null)
        {
            return false;
        }

        Write(game.Id, metadata);
        GameUpdated?.Invoke(this, game.Id);
        return true;
    }

    /// <summary>Consulta as fontes se precisar; devolve verdadeiro quando consultou (para espaçar a próxima).</summary>
    private async Task<bool> UpdateAsync(Game game, bool force)
    {
        var update = await _agent.UpdateAsync(game, Read(game.Id), force, CancellationToken.None).ConfigureAwait(true);
        if (update.Metadata is not { } metadata)
        {
            return update.Queried;
        }

        Write(game.Id, metadata);
        if (metadata.IsEmpty)
        {
            LogNoMatch(logger, game.Title, metadata.Source); // tenta de novo em 7 dias
        }
        else
        {
            GameUpdated?.Invoke(this, game.Id);
        }

        return update.Queried;
    }

    private string PathFor(string gameId) => Path.Combine(_directory, string.Concat(gameId.Select(c => Path.GetInvalidFileNameChars().Contains(c) ? '_' : c)) + ".json");

    private GameMetadata? Read(string gameId)
    {
        string path = PathFor(gameId);
        try
        {
            return File.Exists(path) ? JsonSerializer.Deserialize<GameMetadata>(File.ReadAllText(path), Options) : null;
        }
        catch (Exception ex) when (ex is JsonException or IOException)
        {
            return null;
        }
    }

    private void Write(string gameId, GameMetadata metadata)
    {
        Directory.CreateDirectory(_directory);
        File.WriteAllText(PathFor(gameId), JsonSerializer.Serialize(metadata, Options));
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Metadados: nenhuma correspondência para '{Title}' ({Sources})")]
    private static partial void LogNoMatch(ILogger logger, string title, string sources);
}
