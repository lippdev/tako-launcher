using System.Globalization;
using System.Net;
using System.Text;
using System.Text.Json;
using Launcher.Core.Library.Metadata.Igdb;

namespace Launcher.Core.Library.Metadata.ScreenScraper;

/// <summary>Por que o ScreenScraper não respondeu; a tela de configuração transforma em texto (<see cref="ScreenScraperErrors"/>).</summary>
public enum ScreenScraperFailure
{
    /// <summary>Esta versão do Tako não tem as credenciais do software (devid/devpassword).</summary>
    NoSoftwareAccess,
    NotConfigured,
    BadUser,
    BadSoftware,
    Blacklisted,
    ThreadLimit,
    QuotaExceeded,
    Closed,
    Network,
    Unavailable,
}

public sealed class ScreenScraperException : Exception
{
    public ScreenScraperException()
        : this(ScreenScraperFailure.Unavailable, "ScreenScraper indisponível")
    {
    }

    public ScreenScraperException(string message)
        : this(ScreenScraperFailure.Unavailable, message)
    {
    }

    public ScreenScraperException(string message, Exception innerException)
        : base(message, innerException)
    {
        Failure = ScreenScraperFailure.Unavailable;
    }

    public ScreenScraperException(ScreenScraperFailure failure, string message, Exception? innerException = null)
        : base(message, innerException)
    {
        Failure = failure;
    }

    public ScreenScraperFailure Failure { get; }
}

/// <summary>Conta do usuário como o ScreenScraper informa (<c>ssuser</c>): nível, consultas simultâneas e cota do dia.</summary>
public sealed record ScreenScraperAccount(string User, int Level, int MaxThreads, int RequestsToday, int MaxRequestsPerDay)
{
    public int KoToday { get; init; }

    public int MaxKoPerDay { get; init; }

    public int RemainingToday => Math.Max(0, MaxRequestsPerDay - RequestsToday);

    /// <summary>Cota do dia esgotada (consultas ou ROMs não reconhecidas).</summary>
    public bool IsExhausted => (MaxRequestsPerDay > 0 && RequestsToday >= MaxRequestsPerDay) || (MaxKoPerDay > 0 && KoToday >= MaxKoPerDay);
}

/// <summary>Mensagens claras, em português, para cada motivo de falha (tela de configuração e avisos).</summary>
public static class ScreenScraperErrors
{
    public const string NoSoftwareAccess = "Esta versão do Tako ainda não tem acesso ao ScreenScraper. Ele chega numa próxima versão; a sua conta fica salva até lá.";

    public static string Message(ScreenScraperFailure failure) => failure switch
    {
        ScreenScraperFailure.NoSoftwareAccess => NoSoftwareAccess,
        ScreenScraperFailure.NotConfigured => "Informe o seu usuário e a sua senha do ScreenScraper.",
        ScreenScraperFailure.BadUser => "Usuário ou senha do ScreenScraper incorretos. Confira em screenscraper.fr (a conta precisa estar validada pelo e-mail).",
        ScreenScraperFailure.BadSoftware => "O ScreenScraper recusou esta versão do Tako. Atualize o Tako.",
        ScreenScraperFailure.Blacklisted => "O ScreenScraper bloqueou esta versão do Tako. Atualize o Tako.",
        ScreenScraperFailure.ThreadLimit => "O ScreenScraper está atendendo outra consulta da sua conta. Espere um pouco e tente de novo.",
        ScreenScraperFailure.QuotaExceeded => "A cota diária da sua conta no ScreenScraper acabou. Ela volta à meia-noite (horário de Paris).",
        ScreenScraperFailure.Closed => "O ScreenScraper está fechado ou sobrecarregado agora. Tente mais tarde.",
        ScreenScraperFailure.Network => "Sem conexão com o ScreenScraper. Confira a internet e tente de novo.",
        _ => "O ScreenScraper não respondeu como esperado. Tente mais tarde.",
    };
}

/// <summary>
/// Cliente da API v2 do ScreenScraper (<c>GET https://api.screenscraper.fr/api2/{endpoint}.php</c>, JSON), ADR-0022.
/// Respeita a conta do usuário: no máximo <c>maxthreads</c> consultas ao mesmo tempo (1 até a primeira resposta), uma
/// por 1,2 s por consulta simultânea, espera e tenta de novo quando o limite de consultas simultâneas estoura (429),
/// e pausa quando a cota do dia acaba (430/431) ou o servidor fecha (401/423) — pausado, o provedor fica desligado e as
/// ROMs não ficam marcadas como "sem correspondência". Respostas ficam em cache por 30 dias.
/// </summary>
public sealed class ScreenScraperClient : IDisposable
{
    public static readonly Uri BaseUri = new("https://api.screenscraper.fr/api2/");

    /// <summary>Esquema das mídias guardadas sem credenciais (<see cref="ScreenScraperMedia"/>).</summary>
    public const string MediaScheme = ScreenScraperMedia.Scheme;

    private const int MaxAttempts = 4;
    private static readonly TimeSpan Spacing = TimeSpan.FromSeconds(1.2);

    private readonly HttpClient _http;
    private readonly ScreenScraperSoftware _software;
    private readonly Func<ScreenScraperSettings> _settings;
    private readonly TimeProvider _time;
    private readonly IIgdbResponseCache? _cache;
    private readonly Func<TimeSpan, CancellationToken, Task> _delay;
    private readonly ThreadLimiter _threads = new(1);
    private readonly object _gate = new();
    private RateGate _rate;
    private int _maxThreads = 1;
    private (DateTimeOffset Until, ScreenScraperSettings For, ScreenScraperFailure Reason)? _pause;

    public ScreenScraperClient(HttpClient http, ScreenScraperSoftware software, Func<ScreenScraperSettings> settings, TimeProvider time, IIgdbResponseCache? cache = null, Func<TimeSpan, CancellationToken, Task>? delay = null)
    {
        _http = http;
        _software = software;
        _settings = settings;
        _time = time;
        _cache = cache;
        _rate = new RateGate(1, Spacing, time);
        _delay = delay ?? ((wait, token) => Task.Delay(wait, time, token));
    }

    /// <summary>Esta versão tem as credenciais do software (vêm só do build de release).</summary>
    public bool HasSoftwareAccess => _software.IsAvailable;

    /// <summary>Credenciais do software e conta do usuário ligada.</summary>
    public bool IsConfigured => _software.IsAvailable && _settings().IsActive;

    /// <summary>Pausado (cota do dia, servidor fechado, sem rede): volta sozinho no prazo ou quando a conta muda.</summary>
    public bool IsPaused => PauseReason is not null;

    public ScreenScraperFailure? PauseReason
    {
        get
        {
            lock (_gate)
            {
                return _pause is { } p && _time.GetUtcNow() < p.Until && p.For == _settings() ? p.Reason : null;
            }
        }
    }

    /// <summary>Última conta vista numa resposta (nível, cota); nula antes da primeira consulta.</summary>
    public ScreenScraperAccount? Account { get; private set; }

    /// <summary>Consultas que chegaram à rede (sem contar o cache); para testes e diagnóstico.</summary>
    public int NetworkRequests { get; private set; }

    public void Dispose() => _threads.Dispose();

    /// <summary>
    /// <c>response</c> da consulta, ou nulo quando o jogo não existe no ScreenScraper (404, "non trouvée").
    /// <paramref name="query"/> leva só os parâmetros da consulta: as credenciais são acrescentadas aqui e nunca vão para o cache.
    /// </summary>
    /// <exception cref="ScreenScraperException">Sem acesso, conta recusada, cota, servidor fechado ou fora do ar.</exception>
    public async Task<JsonElement?> GetAsync(string endpoint, IReadOnlyList<KeyValuePair<string, string>> query, CancellationToken cancellationToken, bool useCache = true)
    {
        ArgumentException.ThrowIfNullOrEmpty(endpoint);
        ArgumentNullException.ThrowIfNull(query);
        if (!_software.IsAvailable)
        {
            throw new ScreenScraperException(ScreenScraperFailure.NoSoftwareAccess, "Sem credenciais do software");
        }

        var settings = _settings();
        if (!settings.HasAccount)
        {
            throw new ScreenScraperException(ScreenScraperFailure.NotConfigured, "Sem conta do ScreenScraper");
        }

        if (PauseReason is { } paused)
        {
            throw new ScreenScraperException(paused, "ScreenScraper pausado");
        }

        string key = endpoint + "?" + string.Join('&', query.OrderBy(kv => kv.Key, StringComparer.Ordinal).Select(kv => kv.Key + "=" + kv.Value));
        if (useCache && _cache?.TryRead(key) is { } cached && ScreenScraperParser.Envelope(cached) is { } hit)
        {
            return hit;
        }

        await _threads.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            string? json = await SendAsync(BuildUri(endpoint, query, settings), settings, cancellationToken).ConfigureAwait(false);
            if (json is null)
            {
                return null;
            }

            var response = ScreenScraperParser.Envelope(json) ?? throw Pause(ScreenScraperFailure.Unavailable, TimeSpan.FromMinutes(5), settings, "Resposta inesperada do ScreenScraper");
            Observe(ScreenScraperParser.Account(response), settings);
            if (useCache)
            {
                _cache?.Write(key, json);
            }

            return response;
        }
        finally
        {
            _threads.Release();
        }
    }

    /// <summary>
    /// Baixa uma mídia (<c>ssmedia://…</c>) com as credenciais, dentro do mesmo limite de consultas simultâneas. Nulo
    /// quando não há acesso, a mídia não existe ou o ScreenScraper falhou (a interface usa a arte de reserva).
    /// </summary>
    public async Task<byte[]?> DownloadMediaAsync(Uri media, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(media);
        var settings = _settings();
        if (!_software.IsAvailable || ScreenScraperMedia.ToDownload(media, _software, settings) is not { } source || PauseReason is not null)
        {
            return null;
        }

        await _threads.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            await WaitRateAsync(cancellationToken).ConfigureAwait(false);
            NetworkRequests++;
            using var response = await _http.GetAsync(source, cancellationToken).ConfigureAwait(false);
            if (!response.IsSuccessStatusCode)
            {
                return null;
            }

            byte[] bytes = await response.Content.ReadAsByteArrayAsync(cancellationToken).ConfigureAwait(false);
            // O ScreenScraper responde texto ("NOMEDIA", erro) com 200 quando não tem a mídia.
            return bytes.Length < 256 ? null : bytes;
        }
        catch (HttpRequestException)
        {
            return null;
        }
        catch (TaskCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return null;
        }
        finally
        {
            _threads.Release();
        }
    }

    /// <summary>
    /// Botão "Testar": consulta <c>ssuserInfos.php</c> com a conta informada (ainda não salva), sem cache, e devolve o nível
    /// e a cota. Sem as credenciais do software, falha com <see cref="ScreenScraperFailure.NoSoftwareAccess"/>.
    /// </summary>
    public static async Task<ScreenScraperAccount> TestAsync(HttpClient http, ScreenScraperSoftware software, ScreenScraperSettings settings, TimeProvider time, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var account = settings with { Enabled = true };
        using var client = new ScreenScraperClient(http, software, () => account, time);
        var response = await client.GetAsync("ssuserInfos.php", [], cancellationToken, useCache: false).ConfigureAwait(false);
        return (response is { } r ? ScreenScraperParser.Account(r) : null)
            ?? throw new ScreenScraperException(ScreenScraperFailure.BadUser, "Conta não reconhecida");
    }

    /// <summary>Endereço completo: credenciais do software e do usuário, <c>output=json</c> e os parâmetros da consulta.</summary>
    public Uri BuildUri(string endpoint, IReadOnlyList<KeyValuePair<string, string>> query, ScreenScraperSettings settings)
    {
        ArgumentNullException.ThrowIfNull(query);
        ArgumentNullException.ThrowIfNull(settings);
        var all = new List<KeyValuePair<string, string>>
        {
            new("devid", _software.DevId),
            new("devpassword", _software.DevPassword),
            new("softname", _software.SoftName),
            new("output", "json"),
            new("ssid", settings.User),
            new("sspassword", settings.Password),
        };
        all.AddRange(query);
        return new Uri(BaseUri, endpoint + "?" + string.Join('&', all.Select(kv => Uri.EscapeDataString(kv.Key) + "=" + Uri.EscapeDataString(kv.Value))));
    }

    /// <summary>
    /// Classifica a resposta pelo código HTTP e, como o ScreenScraper às vezes responde com texto de erro em francês, pelo
    /// começo do corpo. Nulo = sucesso ou, com <paramref name="notFound"/>, jogo não encontrado.
    /// </summary>
    public static ScreenScraperFailure? Classify(HttpStatusCode status, string? body, out bool notFound)
    {
        string text = body ?? string.Empty;
        string head = text.Length > 512 ? text[..512] : text;
        int code = (int)status;
        notFound = false;
        if (code is >= 200 and < 300 && head.TrimStart().StartsWith('{'))
        {
            return null; // JSON: sucesso (o texto de erro nunca vem em JSON)
        }

        switch (code)
        {
            case 430 or 431:
                return ScreenScraperFailure.QuotaExceeded;
            case 426:
                return ScreenScraperFailure.Blacklisted;
            case 429:
                return ScreenScraperFailure.ThreadLimit;
            case 401 or 423:
                return ScreenScraperFailure.Closed;
        }

        if (status == HttpStatusCode.NotFound || head.StartsWith("Erreur : Rom", StringComparison.OrdinalIgnoreCase) || Has(head, "non trouvée !"))
        {
            notFound = true;
            return null;
        }

        return code switch
        {
            _ when Has(head, "quota") => ScreenScraperFailure.QuotaExceeded,
            _ when Has(head, "blacklist") => ScreenScraperFailure.Blacklisted,
            _ when Has(head, "threads") => ScreenScraperFailure.ThreadLimit,
            _ when Has(head, "API fermé") || Has(head, "API totalement") || Has(head, "API closed") => ScreenScraperFailure.Closed,
            _ when Has(head, "identifiants utilisateur") || (Has(head, "Erreur de login") && Has(head, "utilisateur")) => ScreenScraperFailure.BadUser,
            403 => ScreenScraperFailure.BadSoftware,
            _ when Has(head, "identifiants développeur") || Has(head, "Erreur de login") => ScreenScraperFailure.BadSoftware,
            _ => ScreenScraperFailure.Unavailable,
        };

        static bool Has(string text, string value) => text.Contains(value, StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>Próxima meia-noite de Paris, quando o ScreenScraper zera as cotas.</summary>
    public static DateTimeOffset NextQuotaReset(DateTimeOffset now)
    {
        TimeSpan offset;
        try
        {
            offset = TimeZoneInfo.FindSystemTimeZoneById("Europe/Paris").GetUtcOffset(now);
        }
        catch (Exception ex) when (ex is TimeZoneNotFoundException or InvalidTimeZoneException)
        {
            offset = TimeSpan.FromHours(1);
        }

        var local = now.ToOffset(offset);
        return new DateTimeOffset(local.Date.AddDays(1), offset).AddMinutes(5);
    }

    private async Task<string?> SendAsync(Uri uri, ScreenScraperSettings settings, CancellationToken cancellationToken)
    {
        for (int attempt = 1; ; attempt++)
        {
            await WaitRateAsync(cancellationToken).ConfigureAwait(false);
            HttpResponseMessage response;
            try
            {
                NetworkRequests++;
                response = await _http.GetAsync(uri, cancellationToken).ConfigureAwait(false);
            }
            catch (HttpRequestException ex)
            {
                throw Pause(ScreenScraperFailure.Network, TimeSpan.FromMinutes(2), settings, "Sem conexão com o ScreenScraper", ex);
            }
            catch (TaskCanceledException ex) when (!cancellationToken.IsCancellationRequested)
            {
                throw Pause(ScreenScraperFailure.Network, TimeSpan.FromMinutes(2), settings, "O ScreenScraper demorou a responder", ex);
            }

            using (response)
            {
                string body = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
                var failure = Classify(response.StatusCode, body, out bool notFound);
                if (notFound)
                {
                    return null;
                }

                switch (failure)
                {
                    case null:
                        return body;
                    case ScreenScraperFailure.ThreadLimit when attempt < MaxAttempts:
                        await _delay(IgdbClient.Backoff(attempt, response.Headers.RetryAfter?.Delta), cancellationToken).ConfigureAwait(false);
                        continue;
                    case ScreenScraperFailure.ThreadLimit:
                        throw Pause(failure.Value, TimeSpan.FromMinutes(1), settings, "ScreenScraper: limite de consultas simultâneas");
                    case ScreenScraperFailure.QuotaExceeded:
                        throw Pause(failure.Value, NextQuotaReset(_time.GetUtcNow()) - _time.GetUtcNow(), settings, "ScreenScraper: cota do dia esgotada");
                    case ScreenScraperFailure.Closed:
                        throw Pause(failure.Value, TimeSpan.FromMinutes(15), settings, $"ScreenScraper fechado ({(int)response.StatusCode})");
                    case ScreenScraperFailure.BadUser or ScreenScraperFailure.BadSoftware or ScreenScraperFailure.Blacklisted:
                        // Não adianta insistir até a conta (ou o Tako) mudar.
                        throw Pause(failure.Value, TimeSpan.FromHours(12), settings, $"ScreenScraper recusou o acesso ({(int)response.StatusCode})");
                    default:
                        throw Pause(failure.Value, TimeSpan.FromMinutes(5), settings, $"ScreenScraper respondeu {(int)response.StatusCode}");
                }
            }
        }
    }

    private async Task WaitRateAsync(CancellationToken cancellationToken)
    {
        TimeSpan wait;
        lock (_gate)
        {
            wait = _rate.Reserve();
        }

        if (wait > TimeSpan.Zero)
        {
            await _delay(wait, cancellationToken).ConfigureAwait(false);
        }
    }

    private ScreenScraperException Pause(ScreenScraperFailure failure, TimeSpan duration, ScreenScraperSettings settings, string message, Exception? inner = null)
    {
        lock (_gate)
        {
            _pause = (_time.GetUtcNow() + duration, settings, failure);
        }

        return new ScreenScraperException(failure, message, inner);
    }

    /// <summary>A conta vem em toda resposta: ajusta as consultas simultâneas e pausa quando a cota do dia acaba.</summary>
    private void Observe(ScreenScraperAccount? account, ScreenScraperSettings settings)
    {
        if (account is null)
        {
            return;
        }

        Account = account;
        int threads = Math.Clamp(account.MaxThreads, 1, 8);
        lock (_gate)
        {
            if (threads != _maxThreads)
            {
                _maxThreads = threads;
                _rate = new RateGate(threads, Spacing, _time);
                _threads.Resize(threads);
            }
        }

        if (account.IsExhausted)
        {
            _ = Pause(ScreenScraperFailure.QuotaExceeded, NextQuotaReset(_time.GetUtcNow()) - _time.GetUtcNow(), settings, "ScreenScraper: cota do dia esgotada");
        }
    }

    internal static string Number(long value) => value.ToString(CultureInfo.InvariantCulture);

    /// <summary>Semáforo cujo tamanho muda quando a conta informa <c>maxthreads</c>.</summary>
    private sealed class ThreadLimiter(int initial) : IDisposable
    {
        private readonly SemaphoreSlim _semaphore = new(initial, int.MaxValue);
        private readonly object _gate = new();
        private int _size = initial;
        private int _debt;

        public Task WaitAsync(CancellationToken cancellationToken) => _semaphore.WaitAsync(cancellationToken);

        public void Release()
        {
            lock (_gate)
            {
                if (_debt > 0)
                {
                    _debt--; // diminuiu: esta vaga some
                    return;
                }
            }

            _semaphore.Release();
        }

        public void Resize(int size)
        {
            lock (_gate)
            {
                int delta = size - _size;
                _size = size;
                if (delta > 0)
                {
                    int paid = Math.Min(delta, _debt);
                    _debt -= paid;
                    if (delta - paid > 0)
                    {
                        _semaphore.Release(delta - paid);
                    }
                }
                else
                {
                    _debt -= delta;
                }
            }
        }

        public void Dispose() => _semaphore.Dispose();
    }
}

/// <summary>
/// As URLs de mídia que o ScreenScraper devolve trazem as credenciais (do software e do usuário) na query. O Tako as
/// guarda sem credenciais, como <c>ssmedia://host/caminho?…</c>, e só as acrescenta na hora de baixar (cache de artes).
/// Só hosts <c>*.screenscraper.fr</c> são aceitos, para nunca mandar credenciais a outro lugar.
/// </summary>
public static class ScreenScraperMedia
{
    public const string Scheme = "ssmedia";

    private static readonly HashSet<string> Credentials = new(StringComparer.OrdinalIgnoreCase) { "devid", "devpassword", "softname", "ssid", "sspassword", "output" };

    /// <summary>URL da resposta → endereço guardado (sem credenciais); nulo se não for do ScreenScraper.</summary>
    public static Uri? FromResponse(string? url)
    {
        if (string.IsNullOrWhiteSpace(url) || !Uri.TryCreate(url.Replace(" ", "%20", StringComparison.Ordinal), UriKind.Absolute, out var uri)
            || uri.Scheme != Uri.UriSchemeHttps && uri.Scheme != Uri.UriSchemeHttp || !IsScreenScraperHost(uri.Host))
        {
            return null;
        }

        var kept = Query(uri).Where(kv => !Credentials.Contains(kv.Key)).ToList();
        string query = kept.Count == 0 ? string.Empty : "?" + string.Join('&', kept.Select(kv => kv.Key + "=" + kv.Value));
        return new Uri($"{Scheme}://{uri.Host}{uri.AbsolutePath}{query}");
    }

    /// <summary>Endereço guardado → URL https com as credenciais, para baixar; nulo se não for uma mídia do ScreenScraper.</summary>
    public static Uri? ToDownload(Uri media, ScreenScraperSoftware software, ScreenScraperSettings settings)
    {
        ArgumentNullException.ThrowIfNull(media);
        ArgumentNullException.ThrowIfNull(software);
        ArgumentNullException.ThrowIfNull(settings);
        if (media.Scheme != Scheme || !IsScreenScraperHost(media.Host))
        {
            return null;
        }

        var parts = new List<string>
        {
            "devid=" + Uri.EscapeDataString(software.DevId),
            "devpassword=" + Uri.EscapeDataString(software.DevPassword),
            "softname=" + Uri.EscapeDataString(software.SoftName),
            "ssid=" + Uri.EscapeDataString(settings.User),
            "sspassword=" + Uri.EscapeDataString(settings.Password),
        };
        parts.AddRange(Query(media).Where(kv => !Credentials.Contains(kv.Key)).Select(kv => kv.Key + "=" + kv.Value));
        return new Uri($"https://{media.Host}{media.AbsolutePath}?{string.Join('&', parts)}");
    }

    public static bool IsScreenScraperHost(string host) =>
        host.Equals("screenscraper.fr", StringComparison.OrdinalIgnoreCase) || host.EndsWith(".screenscraper.fr", StringComparison.OrdinalIgnoreCase);

    // Mantém os valores como vieram (já codificados), só separa os pares.
    private static IEnumerable<KeyValuePair<string, string>> Query(Uri uri) =>
        uri.Query.TrimStart('?').Split('&', StringSplitOptions.RemoveEmptyEntries)
            .Select(pair => pair.Split('=', 2))
            .Select(kv => new KeyValuePair<string, string>(kv[0], kv.Length > 1 ? kv[1] : string.Empty));
}
