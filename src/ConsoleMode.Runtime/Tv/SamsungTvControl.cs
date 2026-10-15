namespace ConsoleMode.Runtime.Tv;

/// <summary>
/// TV Samsung (Tizen, 2016 em diante) pela API de controle remoto: liga (Wake-on-LAN ou tecla Power quando está em
/// espera com a rede ligada), manda a tecla "HDMI n" e, se pedido, a tecla Power no fim (só se a TV está ligada, já que
/// a tecla alterna). Na TV: "Acesso de dispositivos IP remotos" ligado. Na primeira vez a TV pergunta se permite o
/// "Tako"; o token fica cifrado em <see cref="TvPairingStore"/>.
/// </summary>
public sealed class SamsungTvControl(TvPairingStore pairing, ITvTransport transport, string name, Action<TvEvent>? events = null) : ITvControl
{
    /// <summary>Intervalo entre perguntas "já ligou?" depois do Wake-on-LAN (os testes reduzem).</summary>
    public TimeSpan PollInterval { get; init; } = TimeSpan.FromSeconds(2);

    /// <summary>Tempo para a TV subir depois do Wake-on-LAN (os testes reduzem).</summary>
    public TimeSpan WakeWait { get; init; } = AndroidTvControl.WakeWait;

    /// <summary>Depois da tecla Power a TV demora para aceitar a tecla da entrada (os testes reduzem).</summary>
    public TimeSpan AfterPower { get; init; } = TimeSpan.FromSeconds(3);

    /// <summary>Dá tempo da tecla sair antes de fechar a conexão (os testes reduzem).</summary>
    public TimeSpan AfterKey { get; init; } = TimeSpan.FromMilliseconds(400);

    public async Task TurnOnAsync(TvControlConfig config, TimeSpan approvalTimeout, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(config);
        string host = HostOf(config);
        var info = await TvNetwork.ConnectAsync(config, wakeOnLan: true, WakeWait, () => events?.Invoke(TvEvent.WakeOnLanSent), token => ReadInfoAsync(host, token), cancellationToken, Messages(host)).ConfigureAwait(false);
        if (info.IsStandby && WakeOnLan.TryParseMac(config.MacAddress, out byte[] mac))
        {
            await WakeOnLan.SendAsync(mac, cancellationToken).ConfigureAwait(false);
            events?.Invoke(TvEvent.WakeOnLanSent);
            var deadline = DateTime.UtcNow + WakeWait;
            while (info.IsStandby && DateTime.UtcNow < deadline)
            {
                await Task.Delay(PollInterval, cancellationToken).ConfigureAwait(false);
                info = await ReadInfoAsync(host, cancellationToken).ConfigureAwait(false);
            }
        }

        var socket = await OpenRemoteAsync(host, info, approvalTimeout, cancellationToken).ConfigureAwait(false);
        await using (socket.ConfigureAwait(false))
        {
            if (info.IsStandby)
            {
                // Sem MAC (ou o Wake-on-LAN não acordou): a tecla Power liga a TV que está em espera com a rede ligada.
                await socket.SendAsync(SamsungTvProtocol.Key(SamsungTvProtocol.KeyPower), cancellationToken).ConfigureAwait(false);
                await Task.Delay(AfterPower, cancellationToken).ConfigureAwait(false);
            }

            await socket.SendAsync(SamsungTvProtocol.Key(SamsungTvProtocol.HdmiKey(config.HdmiInput)), cancellationToken).ConfigureAwait(false);
            await Task.Delay(AfterKey, cancellationToken).ConfigureAwait(false);
        }
    }

    public async Task StandbyAsync(TvControlConfig config, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(config);
        string host = HostOf(config);
        var info = await TvNetwork.ConnectAsync(config, wakeOnLan: false, TimeSpan.Zero, null, token => ReadInfoAsync(host, token), cancellationToken, Messages(host)).ConfigureAwait(false);
        if (info.IsStandby)
        {
            return; // a tecla Power alternaria e ligaria a TV
        }

        var socket = await OpenRemoteAsync(host, info, TimeSpan.FromSeconds(5), cancellationToken).ConfigureAwait(false);
        await using (socket.ConfigureAwait(false))
        {
            await socket.SendAsync(SamsungTvProtocol.Key(SamsungTvProtocol.KeyPower), cancellationToken).ConfigureAwait(false);
            await Task.Delay(AfterKey, cancellationToken).ConfigureAwait(false);
        }
    }

    public static TvNetworkMessages Messages(string host) => new(
        $"A TV Samsung em {host} não respondeu. Confira o IP e se a TV está na mesma rede.",
        $"A TV Samsung em {host} recusou a conexão. Na TV, ligue Configurações › Geral › Gerenciador de dispositivos externos › Gerenciador de conexão de dispositivo › \"Acesso de dispositivos IP remotos\".");

    private static string HostOf(TvControlConfig config)
    {
        string host = AdbProtocol.ParseEndpoint(config.Host).Host;
        return host.Length > 0 ? host : throw new TvControlException("Informe o endereço IP da TV.");
    }

    /// <summary>Lê a informação da TV; JSON estranho vira "TV ligada, modelo desconhecido" (a tecla ainda pode funcionar).</summary>
    private async Task<SamsungInfo> ReadInfoAsync(string host, CancellationToken cancellationToken)
    {
        string json = await transport.GetStringAsync(SamsungTvProtocol.InfoUri(host), cancellationToken).ConfigureAwait(false);
        return SamsungTvProtocol.ParseInfo(json) ?? new SamsungInfo(null, null, null, TokenAuth: true, IsSamsung: false);
    }

    private async Task<ITvSocket> OpenRemoteAsync(string host, SamsungInfo info, TimeSpan approvalTimeout, CancellationToken cancellationToken)
    {
        string? saved = pairing.Get(TvKind.Samsung, host);
        ITvSocket socket;
        try
        {
            socket = await transport.ConnectAsync(SamsungTvProtocol.RemoteUri(host, name, saved, info.TokenAuth), cancellationToken).ConfigureAwait(false);
        }
        catch (TvConnectionException ex)
        {
            throw new TvControlException(TvNetwork.IsRefused(ex) ? Messages(host).Refused : Messages(host).Unreachable, ex);
        }

        try
        {
            if (saved is null)
            {
                events?.Invoke(TvEvent.WaitingForApproval);
            }

            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            timeout.CancelAfter(approvalTimeout + WebOsTvControl.RequestTimeout);
            while (true)
            {
                var channelEvent = SamsungTvProtocol.ParseEvent(await socket.ReceiveAsync(timeout.Token).ConfigureAwait(false));
                switch (channelEvent.Kind)
                {
                    case SamsungEventKind.Connected:
                        if (channelEvent.Token is { } token && token != saved)
                        {
                            pairing.Set(TvKind.Samsung, host, token);
                            events?.Invoke(TvEvent.PairingSaved);
                        }

                        return socket;
                    case SamsungEventKind.Unauthorized:
                        throw new TvControlException("A TV Samsung recusou o Tako. Na TV, em Gerenciador de conexão de dispositivo › Lista de dispositivos, permita o \"Tako\" e toque em Testar de novo.");
                    case SamsungEventKind.TimedOut:
                        throw NotApproved();
                }
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            await socket.DisposeAsync().ConfigureAwait(false);
            throw NotApproved();
        }
        catch
        {
            await socket.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }

    private static TvControlException NotApproved() =>
        new("A TV Samsung não recebeu a permissão a tempo. Toque em Testar de novo e escolha \"Permitir\" no aviso da TV.");
}
