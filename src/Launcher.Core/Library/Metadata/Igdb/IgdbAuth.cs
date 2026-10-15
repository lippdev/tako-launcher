using System.Net;
using System.Text.Json;

namespace Launcher.Core.Library.Metadata.Igdb;

/// <summary>Por que a IGDB não respondeu; a tela de configuração transforma em texto (<see cref="IgdbErrors"/>).</summary>
public enum IgdbFailure
{
    NotConfigured,
    InvalidClientId,
    InvalidSecret,
    Unauthorized,
    RateLimited,
    Network,
    Unavailable,
}

public sealed class IgdbException : Exception
{
    public IgdbException()
        : this(IgdbFailure.Unavailable, "IGDB indisponível")
    {
    }

    public IgdbException(string message)
        : this(IgdbFailure.Unavailable, message)
    {
    }

    public IgdbException(string message, Exception innerException)
        : base(message, innerException)
    {
        Failure = IgdbFailure.Unavailable;
    }

    public IgdbException(IgdbFailure failure, string message, Exception? innerException = null)
        : base(message, innerException)
    {
        Failure = failure;
    }

    public IgdbFailure Failure { get; }
}

/// <summary>Cabeçalhos de uma consulta à IGDB: <c>Client-ID</c> e <c>Authorization: Bearer</c>.</summary>
public sealed record IgdbAccess(string ClientId, string AccessToken);

/// <summary>
/// Obtém o acesso à IGDB. Hoje há um modo só, a chave da Twitch do usuário (<see cref="TwitchAppAuthenticator"/>); a
/// interface existe para os testes e para outro modo no futuro sem mexer no cliente (ADR-0021).
/// </summary>
public interface IIgdbAuthenticator
{
    /// <summary>Falso quando não há o que usar: a IGDB fica desligada, sem erro.</summary>
    bool IsConfigured { get; }

    /// <exception cref="IgdbException">Chave recusada, Twitch fora do ar ou sem rede.</exception>
    Task<IgdbAccess> GetAccessAsync(CancellationToken cancellationToken);

    /// <summary>A IGDB recusou o token (401): o próximo pedido obtém outro.</summary>
    void Invalidate();
}

/// <summary>
/// Token de app da Twitch (OAuth <c>client_credentials</c> em <c>https://id.twitch.tv/oauth2/token</c>) com a chave do
/// usuário. O token vale ~60 dias; é renovado um pouco antes de vencer, quando a chave muda ou quando a IGDB o recusa.
/// Fica só na memória.
/// </summary>
public sealed class TwitchAppAuthenticator(HttpClient http, Func<IgdbSettings> settings, TimeProvider time) : IIgdbAuthenticator, IDisposable
{
    public static readonly Uri TokenEndpoint = new("https://id.twitch.tv/oauth2/token");

    private static readonly TimeSpan Margin = TimeSpan.FromMinutes(5);
    private readonly SemaphoreSlim _gate = new(1, 1);
    private (string ClientId, string Secret, string Token, DateTimeOffset Expires)? _token;

    public bool IsConfigured => settings().IsActive;

    public async Task<IgdbAccess> GetAccessAsync(CancellationToken cancellationToken)
    {
        var current = settings();
        if (!current.IsActive)
        {
            throw new IgdbException(IgdbFailure.NotConfigured, "IGDB desligada ou sem chave");
        }

        await _gate.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (_token is { } t && t.ClientId == current.ClientId && t.Secret == current.ClientSecret && time.GetUtcNow() < t.Expires - Margin)
            {
                return new IgdbAccess(t.ClientId, t.Token);
            }

            var (token, lifetime) = await RequestTokenAsync(http, current.ClientId, current.ClientSecret, cancellationToken).ConfigureAwait(false);
            _token = (current.ClientId, current.ClientSecret, token, time.GetUtcNow() + lifetime);
            return new IgdbAccess(current.ClientId, token);
        }
        finally
        {
            _gate.Release();
        }
    }

    public void Invalidate() => _token = null;

    public void Dispose() => _gate.Dispose();

    /// <summary>POST do <c>client_credentials</c>; erros viram <see cref="IgdbException"/> com o motivo.</summary>
    public static async Task<(string Token, TimeSpan Lifetime)> RequestTokenAsync(HttpClient http, string clientId, string clientSecret, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(http);
        using var body = new FormUrlEncodedContent(new Dictionary<string, string>
        {
            ["client_id"] = clientId,
            ["client_secret"] = clientSecret,
            ["grant_type"] = "client_credentials",
        });
        try
        {
            using var response = await http.PostAsync(TokenEndpoint, body, cancellationToken).ConfigureAwait(false);
            string text = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
            if (!response.IsSuccessStatusCode)
            {
                throw TokenFailure(response.StatusCode, text);
            }

            using var json = JsonDocument.Parse(text);
            var root = json.RootElement;
            string token = root.TryGetProperty("access_token", out var value) ? value.GetString() ?? string.Empty : string.Empty;
            if (token.Length == 0)
            {
                throw new IgdbException(IgdbFailure.Unavailable, "A Twitch não devolveu um token");
            }

            long seconds = root.TryGetProperty("expires_in", out var expires) && expires.TryGetInt64(out long s) ? s : 3600;
            return (token, TimeSpan.FromSeconds(seconds));
        }
        catch (HttpRequestException ex)
        {
            throw new IgdbException(IgdbFailure.Network, "Sem conexão com a Twitch", ex);
        }
        catch (JsonException ex)
        {
            throw new IgdbException(IgdbFailure.Unavailable, "Resposta inesperada da Twitch", ex);
        }
        catch (TaskCanceledException ex) when (!cancellationToken.IsCancellationRequested)
        {
            throw new IgdbException(IgdbFailure.Network, "A Twitch demorou a responder", ex);
        }
    }

    /// <summary>A Twitch responde 400 "invalid client" (Client ID) e 403 "invalid client secret" (segredo).</summary>
    public static IgdbException TokenFailure(HttpStatusCode status, string body)
    {
        ArgumentNullException.ThrowIfNull(body);
        bool secret = body.Contains("secret", StringComparison.OrdinalIgnoreCase);
        return status switch
        {
            HttpStatusCode.Forbidden or HttpStatusCode.BadRequest when secret => new IgdbException(IgdbFailure.InvalidSecret, $"Twitch recusou o Client Secret ({(int)status})"),
            HttpStatusCode.BadRequest or HttpStatusCode.NotFound => new IgdbException(IgdbFailure.InvalidClientId, $"Twitch não reconheceu o Client ID ({(int)status})"),
            HttpStatusCode.Forbidden or HttpStatusCode.Unauthorized => new IgdbException(IgdbFailure.InvalidSecret, $"Twitch recusou a chave ({(int)status})"),
            HttpStatusCode.TooManyRequests => new IgdbException(IgdbFailure.RateLimited, "Twitch pediu para esperar (429)"),
            _ => new IgdbException(IgdbFailure.Unavailable, $"Twitch indisponível ({(int)status})"),
        };
    }
}

/// <summary>Mensagens claras, em português, para a tela de configuração.</summary>
public static class IgdbErrors
{
    public static string Message(IgdbFailure failure) => failure switch
    {
        IgdbFailure.NotConfigured => "Preencha o Client ID e o Client Secret do seu app da Twitch.",
        IgdbFailure.InvalidClientId => "A Twitch não reconheceu o Client ID. Copie de novo em dev.twitch.tv/console › Aplicativos › Gerenciar (sem espaços).",
        IgdbFailure.InvalidSecret => "A Twitch recusou o Client Secret. Gere um novo em Gerenciar › \"Novo segredo\" e cole aqui (o antigo deixa de valer).",
        IgdbFailure.Unauthorized => "A IGDB recusou o acesso. Confira se o app da Twitch está ativo e se a conta tem a verificação em duas etapas ligada.",
        IgdbFailure.RateLimited => "Muitas consultas seguidas. Espere alguns segundos e tente de novo.",
        IgdbFailure.Network => "Sem conexão com a Twitch/IGDB. Confira a internet e tente de novo.",
        _ => "A IGDB não respondeu agora. Tente de novo em alguns minutos.",
    };
}
