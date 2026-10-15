using System.Net;
using System.Net.Http;
using System.Net.Security;
using System.Net.Sockets;
using System.Net.WebSockets;
using System.Security.Authentication;
using System.Text;

namespace ConsoleMode.Runtime.Tv;

/// <summary>Uma conexão WebSocket com a TV, mensagem de texto por mensagem de texto.</summary>
public interface ITvSocket : IAsyncDisposable
{
    Task SendAsync(string message, CancellationToken cancellationToken);

    /// <summary>A próxima mensagem inteira; <see cref="IOException"/> quando a TV fecha a conexão.</summary>
    Task<string> ReceiveAsync(CancellationToken cancellationToken);
}

/// <summary>
/// Rede até a TV (LG e Samsung): WebSocket, HTTP e "porta aberta?". Atrás de interface para os testes simularem a TV.
/// Falhas de conexão viram <see cref="TvConnectionException"/> (um <see cref="IOException"/>), que o
/// <see cref="TvNetwork"/> trata como "TV não respondeu" (e tenta o Wake-on-LAN).
/// </summary>
public interface ITvTransport
{
    /// <summary>
    /// Abre um WebSocket. Em <c>wss://</c>, o certificado autoassinado da TV é aceito só quando o endereço é um IP da
    /// rede local (<see cref="TvCertificatePolicy"/>).
    /// </summary>
    Task<ITvSocket> ConnectAsync(Uri uri, CancellationToken cancellationToken);

    /// <summary>Corpo de um GET (resposta de sucesso), ou <see cref="TvConnectionException"/>.</summary>
    Task<string> GetStringAsync(Uri uri, CancellationToken cancellationToken);

    Task<bool> IsPortOpenAsync(string host, int port, TimeSpan timeout, CancellationToken cancellationToken);
}

/// <summary>Não deu para conectar na TV. <see cref="Refused"/>: a TV respondeu, mas a porta está fechada.</summary>
public sealed class TvConnectionException : IOException
{
    public TvConnectionException()
    {
    }

    public TvConnectionException(string message)
        : base(message)
    {
    }

    public TvConnectionException(string message, Exception innerException)
        : base(message, innerException)
    {
    }

    public bool Refused { get; init; }
}

/// <summary>
/// Quando aceitar o certificado TLS da TV. LG (porta 3001) e Samsung (porta 8002) usam certificados autoassinados,
/// que nenhuma autoridade confirma. O Tako aceita esse certificado só na conexão com o IP configurado e só se esse IP é
/// da rede local (10.x, 172.16–31.x, 192.168.x, 169.254.x, IPv6 local); nunca para um nome (que poderia ser trocado
/// no DNS) nem para um endereço da internet. A cifra protege a chave/token de quem só escuta a rede; não protege de
/// alguém que se passe pela TV dentro da rede local (mesmo nível do app de celular da própria fabricante).
/// </summary>
public static class TvCertificatePolicy
{
    public static bool Accept(Uri uri, SslPolicyErrors errors)
    {
        ArgumentNullException.ThrowIfNull(uri);
        return errors == SslPolicyErrors.None || (IPAddress.TryParse(uri.IdnHost, out var address) && IsLocal(address));
    }

    public static bool IsLocal(IPAddress address)
    {
        ArgumentNullException.ThrowIfNull(address);
        if (address.IsIPv4MappedToIPv6)
        {
            address = address.MapToIPv4();
        }

        if (address.AddressFamily == AddressFamily.InterNetworkV6)
        {
            byte first = address.GetAddressBytes()[0];
            return address.IsIPv6LinkLocal || (first & 0xFE) == 0xFC; // fe80::/10 e fc00::/7
        }

        byte[] b = address.GetAddressBytes();
        return b[0] == 10
            || (b[0] == 172 && b[1] is >= 16 and <= 31)
            || (b[0] == 192 && b[1] == 168)
            || (b[0] == 169 && b[1] == 254);
    }
}

/// <summary>Transporte real (<see cref="ClientWebSocket"/>, <see cref="HttpClient"/>, <see cref="TcpClient"/>), sem pacotes.</summary>
public sealed class NetworkTvTransport : ITvTransport
{
    /// <summary>Uma TV acordada responde rápido; sem resposta nesse tempo, ela não está lá (ou ainda está ligando).</summary>
    public static readonly TimeSpan ConnectTimeout = TimeSpan.FromSeconds(5);

    /// <summary>Respostas da TV são pequenas (lista de entradas, eventos); passou disso, algo está errado.</summary>
    public const int MaxMessageBytes = 1024 * 1024;

    public async Task<ITvSocket> ConnectAsync(Uri uri, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(uri);
        var socket = new ClientWebSocket();
        socket.Options.KeepAliveInterval = TimeSpan.FromSeconds(15);
        if (uri.Scheme == "wss")
        {
            socket.Options.RemoteCertificateValidationCallback = (_, _, _, errors) => TvCertificatePolicy.Accept(uri, errors);
        }

        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(ConnectTimeout);
        try
        {
            await socket.ConnectAsync(uri, timeout.Token).ConfigureAwait(false);
            return new WebSocketConnection(socket);
        }
        catch (OperationCanceledException ex) when (!cancellationToken.IsCancellationRequested)
        {
            socket.Dispose();
            throw new TvConnectionException($"Sem resposta de {uri.Authority}", ex);
        }
        catch (Exception ex) when (ex is WebSocketException or HttpRequestException or IOException or SocketException or AuthenticationException)
        {
            socket.Dispose();
            throw new TvConnectionException($"Sem conexão com {uri.Authority}", ex) { Refused = TvNetwork.IsRefused(ex) };
        }
    }

    public async Task<string> GetStringAsync(Uri uri, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(uri);
        using var http = new HttpClient { Timeout = Timeout.InfiniteTimeSpan };
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(ConnectTimeout);
        try
        {
            using var response = await http.GetAsync(uri, timeout.Token).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            return await response.Content.ReadAsStringAsync(timeout.Token).ConfigureAwait(false);
        }
        catch (OperationCanceledException ex) when (!cancellationToken.IsCancellationRequested)
        {
            throw new TvConnectionException($"Sem resposta de {uri.Authority}", ex);
        }
        catch (Exception ex) when (ex is HttpRequestException or IOException or SocketException)
        {
            throw new TvConnectionException($"Sem conexão com {uri.Authority}", ex) { Refused = TvNetwork.IsRefused(ex) };
        }
    }

    public async Task<bool> IsPortOpenAsync(string host, int port, TimeSpan timeout, CancellationToken cancellationToken)
    {
        using var tcp = new TcpClient();
        using var limit = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        limit.CancelAfter(timeout);
        try
        {
            await tcp.ConnectAsync(host, port, limit.Token).ConfigureAwait(false);
            return true;
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return false;
        }
        catch (SocketException)
        {
            return false;
        }
    }

    private sealed class WebSocketConnection(ClientWebSocket socket) : ITvSocket
    {
        public Task SendAsync(string message, CancellationToken cancellationToken) =>
            Wrap(() => socket.SendAsync(Encoding.UTF8.GetBytes(message), WebSocketMessageType.Text, endOfMessage: true, cancellationToken));

        public async Task<string> ReceiveAsync(CancellationToken cancellationToken)
        {
            var buffer = new byte[8192];
            using var message = new MemoryStream();
            while (true)
            {
                ValueWebSocketReceiveResult result = default;
                await Wrap(async () => result = await socket.ReceiveAsync(buffer.AsMemory(), cancellationToken).ConfigureAwait(false)).ConfigureAwait(false);
                if (result.MessageType == WebSocketMessageType.Close)
                {
                    throw new TvConnectionException("A TV fechou a conexão");
                }

                if (message.Length + result.Count > MaxMessageBytes)
                {
                    throw new InvalidDataException("Resposta da TV grande demais");
                }

                message.Write(buffer, 0, result.Count);
                if (result.EndOfMessage)
                {
                    return Encoding.UTF8.GetString(message.GetBuffer(), 0, (int)message.Length);
                }
            }
        }

        public async ValueTask DisposeAsync()
        {
            if (socket.State == WebSocketState.Open)
            {
                using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(2));
                try
                {
                    await socket.CloseOutputAsync(WebSocketCloseStatus.NormalClosure, null, timeout.Token).ConfigureAwait(false);
                }
                catch (Exception ex) when (ex is WebSocketException or OperationCanceledException or IOException)
                {
                    // A TV já foi embora: nada a fechar.
                }
            }

            socket.Dispose();
        }

        private static async Task Wrap(Func<Task> action)
        {
            try
            {
                await action().ConfigureAwait(false);
            }
            catch (WebSocketException ex)
            {
                throw new TvConnectionException("A conexão com a TV caiu", ex);
            }
        }
    }
}
