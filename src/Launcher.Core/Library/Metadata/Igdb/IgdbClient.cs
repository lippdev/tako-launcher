using System.Globalization;
using System.Net;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace Launcher.Core.Library.Metadata.Igdb;

/// <summary>
/// Limite de vazão: no máximo <c>permits</c> pedidos por janela (a IGDB aceita 4 por segundo). <see cref="Reserve"/>
/// devolve quanto esperar antes de mandar o próximo; quem chama espera e manda. Puro: o relógio vem de fora.
/// </summary>
public sealed class RateGate(int permits, TimeSpan window, TimeProvider time)
{
    private readonly Queue<DateTimeOffset> _slots = new();
    private readonly object _lock = new();

    public TimeSpan Reserve()
    {
        lock (_lock)
        {
            var now = time.GetUtcNow();
            while (_slots.Count > 0 && _slots.Peek() <= now - window)
            {
                _slots.Dequeue();
            }

            var at = _slots.Count < permits ? now : _slots.ElementAt(_slots.Count - permits) + window;
            _slots.Enqueue(at);
            return at - now;
        }
    }
}

/// <summary>Cache das respostas da IGDB (mesma consulta → mesma resposta), para não repetir consultas.</summary>
public interface IIgdbResponseCache
{
    string? TryRead(string key);

    void Write(string key, string json);
}

/// <summary>
/// Cache em arquivos (<c>%LOCALAPPDATA%\Tako\cache\igdb\&lt;hash&gt;.json</c>), um por consulta, válido por 30 dias.
/// Só guarda respostas com sucesso.
/// </summary>
public sealed class IgdbFileCache(string directory, TimeProvider time, TimeSpan? maxAge = null) : IIgdbResponseCache
{
    private readonly TimeSpan _maxAge = maxAge ?? TimeSpan.FromDays(30);

    public string? TryRead(string key)
    {
        string path = PathFor(key);
        try
        {
            return File.Exists(path) && time.GetUtcNow() - File.GetLastWriteTimeUtc(path) < _maxAge ? File.ReadAllText(path) : null;
        }
        catch (IOException)
        {
            return null;
        }
    }

    public void Write(string key, string json)
    {
        try
        {
            Directory.CreateDirectory(directory);
            File.WriteAllText(PathFor(key), json);
        }
        catch (IOException)
        {
            // Sem cache, a consulta só se repete na próxima vez.
        }
    }

    private string PathFor(string key) => Path.Combine(directory, Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(key)))[..32] + ".json");
}

/// <summary>
/// Cliente da IGDB v4 (<c>POST https://api.igdb.com/v4/{endpoint}</c> com o corpo em Apicalypse). Uma consulta por vez
/// (fila), no máximo 4 por segundo, espera e tenta de novo no 429 (backoff exponencial ou <c>Retry-After</c>), troca o
/// token no 401 e guarda as respostas em cache.
/// </summary>
public sealed class IgdbClient : IDisposable
{
    public static readonly Uri BaseUri = new("https://api.igdb.com/v4/");

    private const int MaxAttempts = 4;
    private readonly HttpClient _http;
    private readonly IIgdbAuthenticator _auth;
    private readonly IIgdbResponseCache? _cache;
    private readonly RateGate _gate;
    private readonly Func<TimeSpan, CancellationToken, Task> _delay;
    private readonly SemaphoreSlim _queue = new(1, 1);

    public IgdbClient(HttpClient http, IIgdbAuthenticator auth, TimeProvider time, IIgdbResponseCache? cache = null, Func<TimeSpan, CancellationToken, Task>? delay = null)
    {
        _http = http;
        _auth = auth;
        _cache = cache;
        _gate = new RateGate(4, TimeSpan.FromSeconds(1), time);
        _delay = delay ?? ((wait, token) => Task.Delay(wait, time, token));
    }

    public bool IsConfigured => _auth.IsConfigured;

    public void Dispose() => _queue.Dispose();

    /// <summary>Consultas que chegaram à rede (sem contar o cache); para testes e diagnóstico.</summary>
    public int NetworkRequests { get; private set; }

    /// <summary>Resposta da consulta (um array JSON).</summary>
    /// <exception cref="IgdbException">Sem chave, chave recusada, limite excedido após as tentativas ou fora do ar.</exception>
    public async Task<JsonElement> QueryAsync(string endpoint, string body, CancellationToken cancellationToken, bool useCache = true)
    {
        ArgumentException.ThrowIfNullOrEmpty(endpoint);
        ArgumentException.ThrowIfNullOrEmpty(body);
        string key = endpoint + "\n" + body;
        if (useCache && _cache?.TryRead(key) is { } cached && TryParse(cached) is { } hit)
        {
            return hit;
        }

        await _queue.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            string json = await SendAsync(endpoint, body, cancellationToken).ConfigureAwait(false);
            var result = TryParse(json) ?? throw new IgdbException(IgdbFailure.Unavailable, "Resposta inesperada da IGDB");
            if (useCache)
            {
                _cache?.Write(key, json);
            }

            return result;
        }
        finally
        {
            _queue.Release();
        }
    }

    private async Task<string> SendAsync(string endpoint, string body, CancellationToken cancellationToken)
    {
        bool refreshed = false;
        for (int attempt = 1; ; attempt++)
        {
            var access = await _auth.GetAccessAsync(cancellationToken).ConfigureAwait(false);
            var wait = _gate.Reserve();
            if (wait > TimeSpan.Zero)
            {
                await _delay(wait, cancellationToken).ConfigureAwait(false);
            }

            using var request = new HttpRequestMessage(HttpMethod.Post, new Uri(BaseUri, endpoint))
            {
                Content = new StringContent(body, Encoding.UTF8, "text/plain"),
            };
            request.Headers.Add("Client-ID", access.ClientId);
            request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", access.AccessToken);
            request.Headers.Accept.ParseAdd("application/json");

            HttpResponseMessage response;
            try
            {
                NetworkRequests++;
                response = await _http.SendAsync(request, cancellationToken).ConfigureAwait(false);
            }
            catch (HttpRequestException ex)
            {
                throw new IgdbException(IgdbFailure.Network, "Sem conexão com a IGDB", ex);
            }
            catch (TaskCanceledException ex) when (!cancellationToken.IsCancellationRequested)
            {
                throw new IgdbException(IgdbFailure.Network, "A IGDB demorou a responder", ex);
            }

            using (response)
            {
                if (response.IsSuccessStatusCode)
                {
                    return await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
                }

                switch (response.StatusCode)
                {
                    case HttpStatusCode.Unauthorized when !refreshed:
                        _auth.Invalidate(); // token vencido ou revogado: pede outro uma vez
                        refreshed = true;
                        continue;
                    case HttpStatusCode.Unauthorized or HttpStatusCode.Forbidden:
                        throw new IgdbException(IgdbFailure.Unauthorized, $"IGDB recusou o acesso ({(int)response.StatusCode})");
                    case HttpStatusCode.TooManyRequests when attempt < MaxAttempts:
                        await _delay(Backoff(attempt, response.Headers.RetryAfter?.Delta), cancellationToken).ConfigureAwait(false);
                        continue;
                    case HttpStatusCode.TooManyRequests:
                        throw new IgdbException(IgdbFailure.RateLimited, "IGDB: limite de consultas");
                    default:
                        throw new IgdbException(IgdbFailure.Unavailable, $"IGDB respondeu {(int)response.StatusCode}");
                }
            }
        }
    }

    /// <summary>
    /// Botão "Testar": obtém um token com a chave informada (ainda não salva) e faz uma consulta mínima, sem cache.
    /// </summary>
    /// <exception cref="IgdbException">Com o motivo (Client ID, segredo, rede...), para virar mensagem clara.</exception>
    public static async Task TestAsync(HttpClient http, IgdbSettings settings, TimeProvider time, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var key = settings with { Enabled = true };
        using var auth = new TwitchAppAuthenticator(http, () => key, time);
        using var client = new IgdbClient(http, auth, time);
        _ = await client.QueryAsync("games", IgdbQueries.Ping, cancellationToken, useCache: false).ConfigureAwait(false);
    }

    /// <summary>1 s, 2 s, 4 s... ou o que o servidor pedir (até 30 s).</summary>
    public static TimeSpan Backoff(int attempt, TimeSpan? retryAfter = null) =>
        retryAfter is { } asked && asked > TimeSpan.Zero
            ? TimeSpan.FromSeconds(Math.Min(asked.TotalSeconds, 30))
            : TimeSpan.FromSeconds(Math.Pow(2, Math.Clamp(attempt - 1, 0, 4)));

    private static JsonElement? TryParse(string json)
    {
        try
        {
            using var document = JsonDocument.Parse(json);
            return document.RootElement.ValueKind == JsonValueKind.Array ? document.RootElement.Clone() : null;
        }
        catch (JsonException)
        {
            return null;
        }
    }

    internal static string Number(long value) => value.ToString(CultureInfo.InvariantCulture);
}
