using System.Net;
using System.Net.Http.Headers;
using System.Text.Json;
using Launcher.Core.Library.Metadata.Igdb;

namespace Launcher.Core.Library.Metadata.SteamGridDb;

public enum SteamGridDbFailure
{
    NotConfigured,
    Unauthorized,
    RateLimited,
    Network,
    Unavailable,
}

public sealed class SteamGridDbException : Exception
{
    public SteamGridDbException()
        : this(SteamGridDbFailure.Unavailable, "SteamGridDB indisponível")
    {
    }

    public SteamGridDbException(string message)
        : this(SteamGridDbFailure.Unavailable, message)
    {
    }

    public SteamGridDbException(string message, Exception innerException)
        : this(SteamGridDbFailure.Unavailable, message, innerException)
    {
    }

    public SteamGridDbException(SteamGridDbFailure failure, string message, Exception? innerException = null)
        : base(message, innerException)
    {
        Failure = failure;
    }

    public SteamGridDbFailure Failure { get; }
}

/// <summary>Mensagens em português para o "Testar" e o seletor.</summary>
public static class SteamGridDbErrors
{
    public static string Message(SteamGridDbFailure failure) => failure switch
    {
        SteamGridDbFailure.NotConfigured => "Cole a chave da API do SteamGridDB e ligue a fonte.",
        SteamGridDbFailure.Unauthorized => "O SteamGridDB recusou a chave. Gere uma nova em steamgriddb.com › Preferências › API e cole aqui.",
        SteamGridDbFailure.RateLimited => "Muitas consultas seguidas. Espere alguns segundos e tente de novo.",
        SteamGridDbFailure.Network => "Sem conexão com o SteamGridDB. Confira a internet e tente de novo.",
        _ => "O SteamGridDB não respondeu agora. Tente de novo em alguns minutos.",
    };
}

/// <summary>
/// Cliente da API v2 do SteamGridDB (<c>GET https://www.steamgriddb.com/api/v2/{caminho}</c>, <c>Authorization: Bearer</c>
/// com a chave do usuário, ADR-0023). Uma consulta por vez, no máximo 2 por segundo (a API não publica limite; o Tako
/// é educado), espera e tenta de novo no 429 e guarda as respostas em cache por 7 dias. 404 = não existe (nulo).
/// </summary>
public sealed class SteamGridDbClient : IDisposable
{
    public static readonly Uri BaseUri = new("https://www.steamgriddb.com/api/v2/");

    private const int MaxAttempts = 4;
    private readonly HttpClient _http;
    private readonly Func<SteamGridDbSettings> _settings;
    private readonly IIgdbResponseCache? _cache;
    private readonly RateGate _gate;
    private readonly Func<TimeSpan, CancellationToken, Task> _delay;
    private readonly SemaphoreSlim _queue = new(1, 1);

    public SteamGridDbClient(HttpClient http, Func<SteamGridDbSettings> settings, TimeProvider time, IIgdbResponseCache? cache = null, Func<TimeSpan, CancellationToken, Task>? delay = null)
    {
        _http = http;
        _settings = settings;
        _cache = cache;
        _gate = new RateGate(2, TimeSpan.FromSeconds(1), time);
        _delay = delay ?? ((wait, token) => Task.Delay(wait, time, token));
    }

    public SteamGridDbSettings Settings => _settings();

    public bool IsConfigured => _settings().IsActive;

    /// <summary>Consultas que chegaram à rede (sem contar o cache).</summary>
    public int NetworkRequests { get; private set; }

    public void Dispose() => _queue.Dispose();

    /// <summary>O <c>data</c> da resposta; nulo quando o SteamGridDB não conhece (404).</summary>
    /// <exception cref="SteamGridDbException">Sem chave, chave recusada, limite após as tentativas ou fora do ar.</exception>
    public async Task<JsonElement?> GetAsync(string path, CancellationToken cancellationToken, bool useCache = true, SteamGridDbSettings? settings = null)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);
        var key = settings ?? _settings();
        if (!key.HasKey)
        {
            throw new SteamGridDbException(SteamGridDbFailure.NotConfigured, "SteamGridDB sem chave");
        }

        if (useCache && _cache?.TryRead(path) is { } cached && Data(cached) is { } hit)
        {
            return hit;
        }

        await _queue.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            string? json = await SendAsync(path, key.ApiKey, cancellationToken).ConfigureAwait(false);
            if (json is null)
            {
                return null;
            }

            var data = Data(json) ?? throw new SteamGridDbException(SteamGridDbFailure.Unavailable, "Resposta inesperada do SteamGridDB");
            if (useCache)
            {
                _cache?.Write(path, json);
            }

            return data;
        }
        finally
        {
            _queue.Release();
        }
    }

    /// <summary>Botão "Testar": uma busca mínima com a chave informada (ainda não salva), sem cache.</summary>
    public static async Task TestAsync(HttpClient http, SteamGridDbSettings settings, TimeProvider time, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(settings);
        using var client = new SteamGridDbClient(http, () => settings, time);
        _ = await client.GetAsync(SteamGridDbQueries.Autocomplete("portal"), cancellationToken, useCache: false, settings).ConfigureAwait(false);
    }

    /// <summary><c>{"success":true,"data":…}</c> → <c>data</c>; nulo se não for JSON ou se falhou.</summary>
    public static JsonElement? Data(string json)
    {
        try
        {
            using var document = JsonDocument.Parse(json);
            var root = document.RootElement;
            return root.ValueKind == JsonValueKind.Object
                && root.TryGetProperty("success", out var success) && success.ValueKind == JsonValueKind.True
                && root.TryGetProperty("data", out var data)
                ? data.Clone()
                : null;
        }
        catch (JsonException)
        {
            return null;
        }
    }

    private async Task<string?> SendAsync(string path, string apiKey, CancellationToken cancellationToken)
    {
        for (int attempt = 1; ; attempt++)
        {
            var wait = _gate.Reserve();
            if (wait > TimeSpan.Zero)
            {
                await _delay(wait, cancellationToken).ConfigureAwait(false);
            }

            using var request = new HttpRequestMessage(HttpMethod.Get, new Uri(BaseUri, path));
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", apiKey);
            request.Headers.Accept.ParseAdd("application/json");
            HttpResponseMessage response;
            try
            {
                NetworkRequests++;
                response = await _http.SendAsync(request, cancellationToken).ConfigureAwait(false);
            }
            catch (HttpRequestException ex)
            {
                throw new SteamGridDbException(SteamGridDbFailure.Network, "Sem conexão com o SteamGridDB", ex);
            }
            catch (TaskCanceledException ex) when (!cancellationToken.IsCancellationRequested)
            {
                throw new SteamGridDbException(SteamGridDbFailure.Network, "O SteamGridDB demorou a responder", ex);
            }

            using (response)
            {
                if (response.IsSuccessStatusCode)
                {
                    return await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
                }

                switch (response.StatusCode)
                {
                    case HttpStatusCode.NotFound:
                        return null;
                    case HttpStatusCode.Unauthorized or HttpStatusCode.Forbidden:
                        throw new SteamGridDbException(SteamGridDbFailure.Unauthorized, $"SteamGridDB recusou a chave ({(int)response.StatusCode})");
                    case HttpStatusCode.TooManyRequests when attempt < MaxAttempts:
                        await _delay(IgdbClient.Backoff(attempt, response.Headers.RetryAfter?.Delta), cancellationToken).ConfigureAwait(false);
                        continue;
                    case HttpStatusCode.TooManyRequests:
                        throw new SteamGridDbException(SteamGridDbFailure.RateLimited, "SteamGridDB: limite de consultas");
                    default:
                        throw new SteamGridDbException(SteamGridDbFailure.Unavailable, $"SteamGridDB respondeu {(int)response.StatusCode}");
                }
            }
        }
    }
}
