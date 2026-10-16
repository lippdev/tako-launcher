using System.IO;
using System.Net.Http;
using System.Text.Json;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Art;
using Launcher.Core.Library.Metadata.Igdb;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.Core.Library.Metadata.SteamGridDb;
using Launcher.Core.Persistence;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Agente de metadados (como no Plex): para cada jogo, junta os dados das fontes ligadas — Steam Store sempre, IGDB
/// com a chave do usuário (ADR-0021), ScreenScraper para ROMs com a conta do usuário (ADR-0022), SteamGridDB para capas
/// que faltam (ADR-0023) — pela prioridade do tipo de jogo (<see cref="MetadataAgent"/>) e guarda em
/// <c>%LOCALAPPDATA%\Tako\cache\metadata</c>. O cache é aplicado na hora; a atualização roda em segundo plano, um jogo
/// por vez, respeitando o limite das APIs. Os ids fixados em "Fontes e IDs" e as imagens fixadas no seletor ficam no
/// banco e nunca são trocados pelo agente.
/// </summary>
public sealed partial class MetadataService(
    IEnumerable<IMetadataProvider> providers,
    IgdbSettingsStore igdb,
    ScreenScraperSettingsStore screenScraper,
    ScreenScraperClient screenScraperClient,
    ScreenScraperSoftware screenScraperSoftware,
    SteamGridDbSettingsStore steamGridDb,
    SteamGridDbClient steamGridDbClient,
    GameChoicesStore choices,
    HttpClient http,
    TimeProvider time,
    ILogger<MetadataService> logger) : ISourceIdService
{
    /// <summary>Espaço entre jogos, pelo limite da loja Steam (~200 consultas a cada 5 minutos).</summary>
    private static readonly TimeSpan Spacing = TimeSpan.FromSeconds(1.5);
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    private readonly MetadataAgent _agent = new(providers, time);
    private readonly string _directory = AppPaths.MetadataCache;
    private readonly object _choicesGate = new();
    private Dictionary<string, IReadOnlyDictionary<ArtKind, Uri>>? _artPins;
    private Dictionary<string, IReadOnlyDictionary<string, string>>? _idPins;
    private bool _running;
    private bool _pending;
    private bool _pendingForce;

    /// <summary>Metadados novos para o jogo (id); disparado na thread de UI.</summary>
    public event EventHandler<string>? GameUpdated;

    /// <summary>Fontes ligadas agora ("Steam" ou "Steam e IGDB").</summary>
    public string ProviderName => _agent.EnabledNames;

    /// <summary>A IGDB está ligada e com chave.</summary>
    public bool IgdbEnabled => _agent.IsEnabled(MetadataPriority.Igdb);

    /// <summary>IGDB ligada ou não e a chave da Twitch (Configurações › Capas e metadados).</summary>
    public IgdbSettings IgdbSettings => igdb.Current;

    /// <summary>Salva a configuração da IGDB; ligada, os jogos ganham os dados dela aos poucos, em segundo plano.</summary>
    public void SaveIgdb(IgdbSettings settings) => igdb.Save(settings);

    /// <summary>Botão "Testar": token da Twitch e uma consulta mínima com a chave informada (ainda não salva).</summary>
    /// <exception cref="IgdbException">Com o motivo, para a mensagem (<see cref="IgdbErrors"/>).</exception>
    public Task TestIgdbAsync(IgdbSettings settings, CancellationToken cancellationToken) => IgdbTester(settings, cancellationToken);

    /// <summary>O teste de fumaça troca por um simulado: o CI nunca fala com a Twitch.</summary>
    internal Func<IgdbSettings, CancellationToken, Task> IgdbTester { get; set; } = (settings, token) => IgdbClient.TestAsync(http, settings, time, token);

    /// <summary>ScreenScraper ligado ou não e a conta do usuário (Configurações › Capas e metadados).</summary>
    public ScreenScraperSettings ScreenScraperSettings => screenScraper.Current;

    /// <summary>Esta versão do Tako tem as credenciais do software do ScreenScraper (só o build de release).</summary>
    public bool HasScreenScraperAccess => screenScraperSoftware.IsAvailable;

    /// <summary>Por que o ScreenScraper está pausado agora (cota do dia, servidor fechado); nulo = não está.</summary>
    public ScreenScraperFailure? ScreenScraperPause => screenScraperClient.PauseReason;

    /// <summary>Última conta vista numa resposta (nível e cota do dia).</summary>
    public ScreenScraperAccount? ScreenScraperAccount => screenScraperClient.Account;

    public void SaveScreenScraper(ScreenScraperSettings settings) => screenScraper.Save(settings);

    /// <summary>Botão "Testar": nível da conta e cota do dia, com a conta informada (ainda não salva).</summary>
    /// <exception cref="ScreenScraperException">Com o motivo, para a mensagem (<see cref="ScreenScraperErrors"/>).</exception>
    public Task<ScreenScraperAccount> TestScreenScraperAsync(ScreenScraperSettings settings, CancellationToken cancellationToken) => ScreenScraperTester(settings, cancellationToken);

    /// <summary>O teste de fumaça troca por um simulado: o CI nunca fala com o ScreenScraper.</summary>
    internal Func<ScreenScraperSettings, CancellationToken, Task<ScreenScraperAccount>> ScreenScraperTester { get; set; } =
        (settings, token) => ScreenScraperClient.TestAsync(http, screenScraperSoftware, settings, time, token);

    /// <summary>SteamGridDB ligado ou não, a chave e as opções de capas (Configurações › Capas e metadados).</summary>
    public SteamGridDbSettings SteamGridDbSettings => steamGridDb.Current;

    /// <summary>Última falha do SteamGridDB (chave recusada, sem rede); nula depois de um sucesso.</summary>
    public SteamGridDbFailure? SteamGridDbFailure => steamGridDbClient.LastFailure;

    /// <summary>Salva o SteamGridDB; com "usar quando faltar capa" ligado, os jogos sem capa ganham uma aos poucos.</summary>
    public void SaveSteamGridDb(SteamGridDbSettings settings) => steamGridDb.Save(settings);

    /// <summary>Botão "Testar": uma busca mínima com a chave informada (ainda não salva).</summary>
    /// <exception cref="SteamGridDbException">Com o motivo, para a mensagem (<see cref="SteamGridDbErrors"/>).</exception>
    public Task TestSteamGridDbAsync(SteamGridDbSettings settings, CancellationToken cancellationToken) => SteamGridDbTester(settings, cancellationToken);

    /// <summary>O teste de fumaça troca por um simulado: o CI nunca fala com o SteamGridDB.</summary>
    internal Func<SteamGridDbSettings, CancellationToken, Task> SteamGridDbTester { get; set; } =
        (settings, token) => SteamGridDbClient.TestAsync(http, settings, time, token);

    /// <summary>
    /// Aplica o que já está em cache (chamado pela biblioteca ao montar cada jogo) e as imagens que o usuário fixou
    /// (ADR-0023), que valem mais que as automáticas.
    /// </summary>
    public void ApplyCached(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        Read(game.Id)?.ApplyTo(game);
        game.PinnedArt = ArtPinsByGame().GetValueOrDefault(game.Id) ?? new Dictionary<ArtKind, Uri>();
    }

    /// <summary>Fixa a imagem do tipo para o jogo (nula = voltar à automática), no banco e no jogo em memória.</summary>
    public bool SetPinnedArt(Game game, ArtKind kind, Uri? url)
    {
        ArgumentNullException.ThrowIfNull(game);
        try
        {
            choices.SetArt(game.Id, kind, url);
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogChoiceFailed(logger, ex, game.Id);
            return false;
        }

        game.PinnedArt = ArtPins.With(game.PinnedArt, kind, url);
        lock (_choicesGate)
        {
            ArtPinsByGame()[game.Id] = game.PinnedArt;
        }

        return true;
    }

    /// <summary>Ids do jogo em cada fonte: os fixados pelo usuário, os do cache e os da loja ("nenhum" tira a fonte).</summary>
    public IReadOnlyDictionary<string, string> KnownIds(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        var ids = new Dictionary<string, string>(ExternalGameIds.For(game), StringComparer.Ordinal);
        if (game.SteamAppId is int appId)
        {
            ids.TryAdd(ExternalGameIds.Steam, appId.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        foreach (var (source, id) in MetadataAgent.Normalize(Read(game.Id))?.SourceIds ?? new Dictionary<string, string>())
        {
            ids[source] = id;
        }

        foreach (var (source, id) in PinnedIds(game.Id))
        {
            if (id.Length == 0)
            {
                ids.Remove(source);
            }
            else
            {
                ids[source] = id;
            }
        }

        return ids;
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
        return string.Join(" + ", names) + (cached.IsPinned || PinnedIds(game.Id).Count > 0 ? " · escolhida por você" : string.Empty);
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
                var due = games().Where(g => _agent.NeedsUpdate(g, Read(g.Id), force, PinnedIds(g.Id))).ToList();
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

    // ---------------------------------------------------------------- Fontes e IDs (ADR-0023)

    public IReadOnlyList<SourceState> SourceStates(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        var cached = MetadataAgent.Normalize(Read(game.Id));
        var pinned = PinnedIds(game.Id);
        var known = KnownIds(game);
        return [.. _agent.Sources(game).Select(provider =>
        {
            var part = cached?.Parts.FirstOrDefault(p => p.Source == provider.Name);
            bool none = pinned.TryGetValue(provider.Name, out string? chosen) && chosen.Length == 0;
            string? id = none ? null : known.GetValueOrDefault(provider.Name);
            return new SourceState(provider.Name, id, part?.Title, part?.ReleaseDate?.Year, pinned.ContainsKey(provider.Name), none);
        })];
    }

    public Task<IReadOnlyList<MetadataMatch>> SearchSourceAsync(Game game, string source, string term, CancellationToken cancellationToken) =>
        _agent.SearchSourceAsync(game, source, term, cancellationToken);

    public Task<(string Id, GameMetadata Metadata)?> ValidateSourceAsync(string source, string text, CancellationToken cancellationToken) =>
        _agent.ValidateAsync(source, SourceIdInput.Parse(source, text), cancellationToken);

    /// <summary>
    /// Fixa o id da fonte (vazio = "nenhum"; nulo = voltar ao automático) e busca de novo os dados e as imagens do jogo
    /// por ele; o agente nunca troca o que foi fixado.
    /// </summary>
    public async Task<bool> PinSourceAsync(Game game, string source, string? id)
    {
        ArgumentNullException.ThrowIfNull(game);
        try
        {
            choices.SetId(game.Id, source, id);
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogChoiceFailed(logger, ex, game.Id);
            return false;
        }

        lock (_choicesGate)
        {
            var ids = new Dictionary<string, string>(IdPinsByGame().GetValueOrDefault(game.Id) ?? new Dictionary<string, string>(), StringComparer.Ordinal);
            if (id is null)
            {
                ids.Remove(source);
            }
            else
            {
                ids[source] = id.Trim();
            }

            IdPinsByGame()[game.Id] = ids;
        }

        await UpdateAsync(game, force: true).ConfigureAwait(true);
        return true;
    }

    private IReadOnlyDictionary<string, string> PinnedIds(string gameId)
    {
        lock (_choicesGate)
        {
            if (IdPinsByGame().TryGetValue(gameId, out var ids))
            {
                return ids;
            }
        }

        // Correspondência fixada pelo "Corrigir correspondência" antigo: vale como ids fixados, sem gravar nada.
        return MetadataAgent.Normalize(Read(gameId)) is { IsPinned: true } legacy ? legacy.SourceIds : new Dictionary<string, string>();
    }

    private Dictionary<string, IReadOnlyDictionary<ArtKind, Uri>> ArtPinsByGame()
    {
        lock (_choicesGate)
        {
            return _artPins ??= LoadChoices(choices.LoadAllArt);
        }
    }

    private Dictionary<string, IReadOnlyDictionary<string, string>> IdPinsByGame()
    {
        lock (_choicesGate)
        {
            return _idPins ??= LoadChoices(choices.LoadAllIds);
        }
    }

    private Dictionary<string, T> LoadChoices<T>(Func<IReadOnlyDictionary<string, T>> read)
    {
        try
        {
            return new Dictionary<string, T>(read(), StringComparer.Ordinal);
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogChoiceFailed(logger, ex, "*");
            return new Dictionary<string, T>(StringComparer.Ordinal);
        }
    }

    /// <summary>Consulta as fontes se precisar; devolve verdadeiro quando consultou (para espaçar a próxima).</summary>
    private async Task<bool> UpdateAsync(Game game, bool force)
    {
        var update = await _agent.UpdateAsync(game, Read(game.Id), force, CancellationToken.None, PinnedIds(game.Id)).ConfigureAwait(true);
        if (update.Metadata is not { } metadata)
        {
            return update.Queried;
        }

        Write(game.Id, metadata);
        if (metadata.IsEmpty)
        {
            LogNoMatch(logger, game.Title, metadata.Source); // tenta de novo em 7 dias
        }

        GameUpdated?.Invoke(this, game.Id);
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

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível ler ou salvar as escolhas de imagens e ids do jogo {GameId}")]
    private static partial void LogChoiceFailed(ILogger logger, Exception exception, string gameId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Metadados: nenhuma correspondência para '{Title}' ({Sources})")]
    private static partial void LogNoMatch(ILogger logger, string title, string sources);
}

/// <summary>Uma fonte em "Fontes e IDs": o id em uso, o nome e o ano que ele representa e se foi fixado pelo usuário.</summary>
public sealed record SourceState(string Source, string? Id, string? Title, int? Year, bool IsPinned, bool IsNone);

/// <summary>"Fontes e IDs" (ADR-0023). O teste de fumaça troca por um simulado: o CI nunca vai à rede.</summary>
public interface ISourceIdService
{
    IReadOnlyList<SourceState> SourceStates(Game game);

    Task<IReadOnlyList<MetadataMatch>> SearchSourceAsync(Game game, string source, string term, CancellationToken cancellationToken);

    /// <summary>Id ou endereço colado → id e os dados que ele representa; nulo quando a fonte não conhece.</summary>
    Task<(string Id, GameMetadata Metadata)?> ValidateSourceAsync(string source, string text, CancellationToken cancellationToken);

    /// <summary>Fixa (id), marca "nenhum" (vazio) ou volta ao automático (nulo) e busca de novo os dados.</summary>
    Task<bool> PinSourceAsync(Game game, string source, string? id);
}
