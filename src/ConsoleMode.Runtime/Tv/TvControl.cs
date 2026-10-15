// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/Tv/TvControlService.cs, src/ConsoleMode/Services/Tv/AndroidTvController.cs e src/ConsoleMode/Services/Tv/TvNetwork.cs
using System.Net.Sockets;

namespace ConsoleMode.Runtime.Tv;

/// <summary>Algo que aconteceu no caminho até a TV (para o log e para o aviso na tela de Testar).</summary>
public enum TvEvent
{
    /// <summary>A TV não respondeu e o pacote do Wake-on-LAN foi enviado.</summary>
    WakeOnLanSent,

    /// <summary>A TV ainda não conhece este PC e está mostrando "Permitir depuração".</summary>
    WaitingForApproval,

    /// <summary>Chave ADB nova criada (primeira vez, ou a anterior era de outro usuário/PC).</summary>
    KeyCreated,

    /// <summary>LG ou Samsung aceitou este PC e a chave/token novo foi guardado (cifrado).</summary>
    PairingSaved,
}

/// <summary>Um jeito de falar com a TV (ADB no Android, WebSocket na LG e na Samsung).</summary>
public interface ITvControl
{
    /// <summary>Liga a TV e troca para a entrada do PC.</summary>
    Task TurnOnAsync(TvControlConfig config, TimeSpan approvalTimeout, CancellationToken cancellationToken);

    /// <summary>Coloca a TV em espera.</summary>
    Task StandbyAsync(TvControlConfig config, CancellationToken cancellationToken);
}

/// <summary>
/// Google TV / Android TV (TCL, Sony, Hisense, Philips…) pela rede, com ADB: acorda e manda a tecla "HDMI n" (ou um
/// comando próprio). Precisa de "Depuração USB" / "Depuração pela rede" nas Opções do desenvolvedor da TV; nada a
/// instalar no PC.
/// </summary>
public sealed class AndroidTvControl(AdbKeyStore keys, string keyName, Action<TvEvent>? events = null) : ITvControl
{
    /// <summary>Uma TV acordada pelo Wake-on-LAN demora para subir a rede e os serviços.</summary>
    public static readonly TimeSpan WakeWait = TimeSpan.FromSeconds(20);

    /// <summary>Logo depois de acordar, algumas TVs perdem a tecla da entrada enquanto a tela inicial carrega.</summary>
    private static readonly TimeSpan AfterWake = TimeSpan.FromSeconds(1.5);

    public async Task TurnOnAsync(TvControlConfig config, TimeSpan approvalTimeout, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(config);
        var adb = await ConnectAsync(config, approvalTimeout, wakeOnLan: true, cancellationToken).ConfigureAwait(false);
        await using (adb.ConfigureAwait(false))
        {
            await adb.ShellAsync($"input keyevent {AdbProtocol.KeyWakeUp}", cancellationToken).ConfigureAwait(false);
            await Task.Delay(AfterWake, cancellationToken).ConfigureAwait(false);
            await adb.ShellAsync(config.InputSwitchCommand, cancellationToken).ConfigureAwait(false);
        }
    }

    public async Task StandbyAsync(TvControlConfig config, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(config);
        var adb = await ConnectAsync(config, TimeSpan.FromSeconds(5), wakeOnLan: false, cancellationToken).ConfigureAwait(false);
        await using (adb.ConfigureAwait(false))
        {
            await adb.ShellAsync($"input keyevent {AdbProtocol.KeySleep}", cancellationToken).ConfigureAwait(false);
        }
    }

    private Task<AdbClient> ConnectAsync(TvControlConfig config, TimeSpan approvalTimeout, bool wakeOnLan, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(config.Host))
        {
            throw new TvControlException("Informe o endereço IP da TV.");
        }

        var (host, port) = AdbProtocol.ParseEndpoint(config.Host);
        return TvNetwork.ConnectAsync(
            config,
            wakeOnLan,
            WakeWait,
            () => events?.Invoke(TvEvent.WakeOnLanSent),
            async token =>
            {
                var (key, origin) = keys.LoadOrCreate();
                using (key)
                {
                    if (origin != AdbKeyOrigin.Loaded)
                    {
                        events?.Invoke(TvEvent.KeyCreated);
                    }

                    return await AdbClient.ConnectAsync(host, port, key, keyName, approvalTimeout, () => events?.Invoke(TvEvent.WaitingForApproval), token).ConfigureAwait(false);
                }
            },
            cancellationToken);
    }
}

/// <summary>
/// Textos de "não respondeu" de cada tipo de TV. <see cref="Refused"/>: a TV está na rede, mas recusou a conexão na
/// porta do controle (o recurso de rede está desligado na TV).
/// </summary>
public sealed record TvNetworkMessages(string Unreachable, string Refused);

/// <summary>Conexão com uma TV na rede, acordando-a com Wake-on-LAN quando está em espera profunda.</summary>
public static class TvNetwork
{
    /// <param name="connect">Uma tentativa de conexão; lança erro de rede quando ninguém responde.</param>
    /// <param name="wakeOnLan">Enviar o pacote mágico (se houver MAC) quando a primeira tentativa falhar.</param>
    /// <param name="wakeWait">Quanto continuar tentando depois do Wake-on-LAN.</param>
    /// <param name="messages">Textos do tipo de TV; nulo = os do Android (ADB).</param>
    public static async Task<T> ConnectAsync<T>(
        TvControlConfig config,
        bool wakeOnLan,
        TimeSpan wakeWait,
        Action? wakeOnLanSent,
        Func<CancellationToken, Task<T>> connect,
        CancellationToken cancellationToken,
        TvNetworkMessages? messages = null)
    {
        ArgumentNullException.ThrowIfNull(config);
        ArgumentNullException.ThrowIfNull(connect);
        messages ??= AndroidMessages(config.Host);
        try
        {
            return await connect(cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (IsUnreachable(ex) && wakeOnLan && WakeOnLan.TryParseMac(config.MacAddress, out byte[] mac))
        {
            // Espera profunda: sem rede até o pacote mágico acordar a TV.
            await WakeOnLan.SendAsync(mac, cancellationToken).ConfigureAwait(false);
            wakeOnLanSent?.Invoke();
        }
        catch (Exception ex) when (IsUnreachable(ex))
        {
            throw Unreachable(messages, ex);
        }

        var deadline = DateTime.UtcNow + wakeWait;
        while (true)
        {
            await Task.Delay(TimeSpan.FromSeconds(2), cancellationToken).ConfigureAwait(false);
            try
            {
                return await connect(cancellationToken).ConfigureAwait(false);
            }
            catch (Exception ex) when (IsUnreachable(ex) && DateTime.UtcNow < deadline)
            {
                // Ainda ligando: tenta de novo.
            }
            catch (Exception ex) when (IsUnreachable(ex))
            {
                throw Unreachable(messages, ex);
            }
        }
    }

    /// <summary>Ninguém escutando, sem rota, tempo de conexão esgotado ou TV que desligou no meio do handshake.</summary>
    public static bool IsUnreachable(Exception ex) => ex is SocketException or IOException;

    /// <summary>A TV respondeu "porta fechada": está ligada na rede, mas o recurso de controle está desligado.</summary>
    public static bool IsRefused(Exception? ex)
    {
        for (var current = ex; current is not null; current = current.InnerException)
        {
            if (current is TvConnectionException { Refused: true } or SocketException { SocketErrorCode: SocketError.ConnectionRefused })
            {
                return true;
            }
        }

        return false;
    }

    private static TvNetworkMessages AndroidMessages(string host) => new(
        $"A TV em {host} não respondeu. Confira o IP, se a TV está na mesma rede e se a depuração está ativada.",
        $"A TV em {host} não respondeu. Confira o IP, se a TV está na mesma rede e se a depuração está ativada.");

    private static TvControlException Unreachable(TvNetworkMessages messages, Exception inner) =>
        new(IsRefused(inner) ? messages.Refused : messages.Unreachable, inner);
}
