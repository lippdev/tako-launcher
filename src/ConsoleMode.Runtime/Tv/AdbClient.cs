// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/Tv/AdbClient.cs
using System.Net.Sockets;
using System.Security.Cryptography;

namespace ConsoleMode.Runtime.Tv;

/// <summary>
/// Cliente ADB mínimo por TCP ("Depuração USB" / "Depuração pela rede" na TV, porta 5555): conecta, autentica com a
/// chave deste PC e roda comandos de shell. Na primeira vez a TV pergunta "Permitir depuração deste computador?"; com
/// "Sempre permitir" marcado, ela lembra a chave e as próximas conexões passam direto.
/// </summary>
public sealed class AdbClient : IAsyncDisposable
{
    public const string NotAllowedMessage = "A TV não autorizou este PC. Toque em Testar de novo e escolha \"Sempre permitir\" e \"Permitir\" na TV.";

    private static readonly TimeSpan ConnectTimeout = TimeSpan.FromSeconds(3);
    private static readonly TimeSpan HandshakeTimeout = TimeSpan.FromSeconds(5);

    private readonly TcpClient _tcp;
    private readonly NetworkStream _stream;
    private uint _nextLocalId = 1;

    private AdbClient(TcpClient tcp)
    {
        _tcp = tcp;
        _stream = tcp.GetStream();
    }

    /// <param name="key">Chave deste PC (a mesma sempre, para a TV lembrar).</param>
    /// <param name="keyName">Nome mostrado na TV ao pedir permissão.</param>
    /// <param name="approvalTimeout">Quanto esperar o usuário aceitar na TV.</param>
    /// <param name="waitingForApproval">Chamado quando a TV ainda não conhece a chave e espera o "Permitir" na tela.</param>
    public static async Task<AdbClient> ConnectAsync(string host, int port, RSA key, string keyName, TimeSpan approvalTimeout, Action? waitingForApproval, CancellationToken cancellationToken)
    {
        var tcp = new TcpClient { NoDelay = true };
        try
        {
            using (var connect = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken))
            {
                connect.CancelAfter(ConnectTimeout);
                try
                {
                    await tcp.ConnectAsync(host, port, connect.Token).ConfigureAwait(false);
                }
                catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
                {
                    throw new SocketException((int)SocketError.TimedOut);
                }
            }

            var client = new AdbClient(tcp);
            await client.HandshakeAsync(key, keyName, approvalTimeout, waitingForApproval, cancellationToken).ConfigureAwait(false);
            return client;
        }
        catch
        {
            tcp.Dispose();
            throw;
        }
    }

    public async Task<string> ShellAsync(string command, CancellationToken cancellationToken)
    {
        uint localId = _nextLocalId++;
        uint remoteId = 0;
        var output = new AdbShellOutput();
        await SendAsync(AdbProtocol.Open, localId, 0, AdbProtocol.ShellService(command), cancellationToken).ConfigureAwait(false);
        while (true)
        {
            var message = await ReadAsync(cancellationToken).ConfigureAwait(false);
            if (message.Arg1 != localId)
            {
                continue;
            }

            switch (message.Command)
            {
                case AdbProtocol.Okay:
                    remoteId = message.Arg0;
                    break;
                case AdbProtocol.Write:
                    output.Append(message.Data);
                    await SendAsync(AdbProtocol.Okay, localId, message.Arg0, null, cancellationToken).ConfigureAwait(false);
                    break;
                case AdbProtocol.Close:
                    if (remoteId != 0)
                    {
                        await SendAsync(AdbProtocol.Close, localId, remoteId, null, cancellationToken).ConfigureAwait(false);
                    }

                    return output.ToString();
            }
        }
    }

    public ValueTask DisposeAsync()
    {
        _tcp.Dispose();
        return ValueTask.CompletedTask;
    }

    private async Task HandshakeAsync(RSA key, string keyName, TimeSpan approvalTimeout, Action? waitingForApproval, CancellationToken cancellationToken)
    {
        await SendAsync(AdbProtocol.Connect, AdbProtocol.Version, AdbProtocol.MaxPayload, AdbProtocol.ConnectBanner(), cancellationToken).ConfigureAwait(false);
        bool sentSignature = false;
        bool sentPublicKey = false;
        while (true)
        {
            // Depois de mandar a chave pública, a TV espera alguém tocar em "Permitir".
            using var wait = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            wait.CancelAfter(sentPublicKey ? approvalTimeout : HandshakeTimeout);
            AdbProtocol.Message message;
            try
            {
                message = await ReadAsync(wait.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
            {
                throw sentPublicKey ? new TvControlException(NotAllowedMessage) : new IOException("ADB: a TV não respondeu ao handshake");
            }

            switch (message.Command)
            {
                case AdbProtocol.Connect:
                    return;
                case AdbProtocol.StartTls:
                    // A "Depuração sem fio" (pareamento por código, Android 11+) usa TLS; a clássica na porta 5555, não.
                    throw new TvControlException("A TV só aceita \"Depuração sem fio\" (com código). Ative \"Depuração USB\" ou \"Depuração pela rede\" nas Opções do desenvolvedor.");
                case AdbProtocol.Auth when message.Arg0 == AdbProtocol.AuthToken:
                    if (!sentSignature)
                    {
                        await SendAsync(AdbProtocol.Auth, AdbProtocol.AuthSignature, 0, AdbProtocol.SignToken(key, message.Data), cancellationToken).ConfigureAwait(false);
                        sentSignature = true;
                    }
                    else if (!sentPublicKey)
                    {
                        waitingForApproval?.Invoke();
                        byte[] publicKey = AdbProtocol.EncodePublicKey(key.ExportParameters(false), keyName);
                        await SendAsync(AdbProtocol.Auth, AdbProtocol.AuthRsaPublicKey, 0, publicKey, cancellationToken).ConfigureAwait(false);
                        sentPublicKey = true;
                    }
                    else
                    {
                        throw new TvControlException(NotAllowedMessage);
                    }

                    break;
            }
        }
    }

    private async Task SendAsync(uint command, uint arg0, uint arg1, byte[]? data, CancellationToken cancellationToken) =>
        await _stream.WriteAsync(AdbProtocol.Encode(command, arg0, arg1, data), cancellationToken).ConfigureAwait(false);

    private async Task<AdbProtocol.Message> ReadAsync(CancellationToken cancellationToken)
    {
        var header = new byte[AdbProtocol.HeaderSize];
        await _stream.ReadExactlyAsync(header, cancellationToken).ConfigureAwait(false);
        var (command, arg0, arg1, length, checksum) = AdbProtocol.DecodeHeader(header);
        var data = new byte[length];
        if (length > 0)
        {
            await _stream.ReadExactlyAsync(data, cancellationToken).ConfigureAwait(false);
        }

        AdbProtocol.ValidatePayload(checksum, data);
        return new AdbProtocol.Message(command, arg0, arg1, data);
    }
}
