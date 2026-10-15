using System.Globalization;
using System.Text.Json.Nodes;

namespace ConsoleMode.Runtime.Tv;

/// <summary>
/// TV LG (webOS) pela API local: liga (Wake-on-LAN), troca para <c>HDMI_n</c> e, se pedido, desliga no fim. Na TV:
/// "LG Connect Apps" ligado (em modelos novos, "Ligar via Wi-Fi"/"Ligar com o celular" para o Wake-on-LAN). Na
/// primeira vez a TV mostra um aviso para aceitar o Tako; a client-key fica cifrada em <see cref="TvPairingStore"/>.
/// </summary>
public sealed class WebOsTvControl(TvPairingStore pairing, ITvTransport transport, Action<TvEvent>? events = null) : ITvControl
{
    /// <summary>Quanto esperar cada resposta a um pedido.</summary>
    public static readonly TimeSpan RequestTimeout = TimeSpan.FromSeconds(10);

    /// <summary>Intervalo entre perguntas "já ligou?" depois do Wake-on-LAN (os testes reduzem).</summary>
    public TimeSpan PollInterval { get; init; } = TimeSpan.FromSeconds(2);

    /// <summary>Tempo para a TV subir depois do Wake-on-LAN (os testes reduzem).</summary>
    public TimeSpan WakeWait { get; init; } = AndroidTvControl.WakeWait;

    public async Task TurnOnAsync(TvControlConfig config, TimeSpan approvalTimeout, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(config);
        string host = HostOf(config);
        var session = await TvNetwork.ConnectAsync(config, wakeOnLan: true, WakeWait, () => events?.Invoke(TvEvent.WakeOnLanSent), token => OpenAsync(host, approvalTimeout, token), cancellationToken, Messages(host)).ConfigureAwait(false);
        try
        {
            var power = await session.PowerAsync(cancellationToken).ConfigureAwait(false);
            if (power == WebOsPower.Standby)
            {
                // Em espera com a rede ligada: a API responde, mas só o Wake-on-LAN liga a tela.
                if (!WakeOnLan.TryParseMac(config.MacAddress, out byte[] mac))
                {
                    throw new TvControlException("A TV LG está em espera. Preencha o MAC da TV para o Tako ligá-la (e ative \"Ligar via Wi-Fi\" na TV).");
                }

                await WakeOnLan.SendAsync(mac, cancellationToken).ConfigureAwait(false);
                events?.Invoke(TvEvent.WakeOnLanSent);
                var deadline = DateTime.UtcNow + WakeWait;
                while (power == WebOsPower.Standby && DateTime.UtcNow < deadline)
                {
                    await Task.Delay(PollInterval, cancellationToken).ConfigureAwait(false);
                    power = await session.PowerAsync(cancellationToken).ConfigureAwait(false);
                }
            }

            if (power == WebOsPower.ScreenOff)
            {
                await session.TryCallAsync(WebOsProtocol.TurnOnScreenUri, new JsonObject { ["standbyMode"] = "active" }, cancellationToken).ConfigureAwait(false);
            }

            var inputs = await session.TryCallAsync(WebOsProtocol.ExternalInputsUri, null, cancellationToken).ConfigureAwait(false);
            string inputId = WebOsProtocol.ChooseInput(inputs is { Ok: true } ? WebOsProtocol.ParseInputs(inputs.Payload) : [], config.HdmiInput);
            var switched = await session.CallAsync(WebOsProtocol.SwitchInputUri, new JsonObject { ["inputId"] = inputId }, cancellationToken).ConfigureAwait(false);
            if (!switched.Ok)
            {
                throw new TvControlException($"A TV LG não trocou para a entrada {inputId}: {switched.Error}");
            }
        }
        finally
        {
            await session.DisposeAsync().ConfigureAwait(false);
        }
    }

    public async Task StandbyAsync(TvControlConfig config, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(config);
        string host = HostOf(config);
        var session = await TvNetwork.ConnectAsync(config, wakeOnLan: false, TimeSpan.Zero, null, token => OpenAsync(host, TimeSpan.FromSeconds(5), token), cancellationToken, Messages(host)).ConfigureAwait(false);
        try
        {
            var result = await session.CallAsync(WebOsProtocol.TurnOffUri, null, cancellationToken).ConfigureAwait(false);
            if (!result.Ok)
            {
                throw new TvControlException($"A TV LG não entrou em espera: {result.Error}");
            }
        }
        finally
        {
            await session.DisposeAsync().ConfigureAwait(false);
        }
    }

    /// <summary>Textos para quando a TV não responde ou recusa a conexão.</summary>
    public static TvNetworkMessages Messages(string host) => new(
        $"A TV LG em {host} não respondeu. Confira o IP, se a TV está na mesma rede e se \"LG Connect Apps\" está ligado na TV.",
        $"A TV LG em {host} recusou a conexão. Nas Configurações da TV, ligue \"LG Connect Apps\" (em modelos novos o nome muda: procure \"Configurações de dispositivo móvel\" ou \"Ligar via Wi-Fi\").");

    private static string HostOf(TvControlConfig config)
    {
        string host = AdbProtocol.ParseEndpoint(config.Host).Host;
        return host.Length > 0 ? host : throw new TvControlException("Informe o endereço IP da TV.");
    }

    /// <summary>Conecta (cifrado primeiro) e registra; a client-key nova é guardada.</summary>
    private async Task<Session> OpenAsync(string host, TimeSpan approvalTimeout, CancellationToken cancellationToken)
    {
        ITvSocket? socket = null;
        TvConnectionException? last = null;
        bool allRefused = true;
        foreach (var uri in WebOsProtocol.Endpoints(host))
        {
            try
            {
                socket = await transport.ConnectAsync(uri, cancellationToken).ConfigureAwait(false);
                break;
            }
            catch (TvConnectionException ex)
            {
                last = ex;
                allRefused &= ex.Refused;
            }
        }

        if (socket is null)
        {
            throw new TvConnectionException("A TV LG não aceitou a conexão", last!) { Refused = allRefused };
        }

        var session = new Session(socket);
        try
        {
            string? saved = pairing.Get(TvKind.LgWebOs, host);
            string key = await RegisterAsync(socket, saved, approvalTimeout, cancellationToken).ConfigureAwait(false);
            if (key != saved)
            {
                pairing.Set(TvKind.LgWebOs, host, key);
                events?.Invoke(TvEvent.PairingSaved);
            }

            return session;
        }
        catch
        {
            await session.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }

    private async Task<string> RegisterAsync(ITvSocket socket, string? clientKey, TimeSpan approvalTimeout, CancellationToken cancellationToken)
    {
        await socket.SendAsync(WebOsProtocol.Register(clientKey), cancellationToken).ConfigureAwait(false);
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(approvalTimeout + RequestTimeout);
        try
        {
            while (true)
            {
                string message = await socket.ReceiveAsync(timeout.Token).ConfigureAwait(false);
                switch (WebOsProtocol.ParseRegisterReply(message))
                {
                    case { State: WebOsRegisterState.Registered, ClientKey: { } key }:
                        return key;
                    case { State: WebOsRegisterState.Prompted }:
                        events?.Invoke(TvEvent.WaitingForApproval);
                        break;
                    case { State: WebOsRegisterState.Denied } denied:
                        throw new TvControlException($"A TV LG recusou o Tako ({denied.Error}). Toque em Testar de novo e escolha \"Aceitar\" no aviso da TV.");
                }
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            throw new TvControlException("A TV LG não recebeu a permissão a tempo. Toque em Testar de novo e escolha \"Aceitar\" no aviso da TV.");
        }
    }

    /// <summary>Uma conexão registrada: pedidos numerados, cada um esperando a própria resposta.</summary>
    private sealed class Session(ITvSocket socket) : IAsyncDisposable
    {
        private int _next;

        public async Task<WebOsResponse> CallAsync(string uri, JsonObject? payload, CancellationToken cancellationToken)
        {
            string id = "tako_" + (++_next).ToString(CultureInfo.InvariantCulture);
            await socket.SendAsync(WebOsProtocol.Request(id, uri, payload), cancellationToken).ConfigureAwait(false);
            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            timeout.CancelAfter(RequestTimeout);
            try
            {
                while (true)
                {
                    var response = WebOsProtocol.ParseResponse(await socket.ReceiveAsync(timeout.Token).ConfigureAwait(false));
                    if (response.Id == id)
                    {
                        return response;
                    }
                }
            }
            catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
            {
                throw new TvControlException("A TV LG parou de responder.");
            }
        }

        /// <summary>Pedido opcional (TVs antigas não têm todos): falha vira nulo.</summary>
        public async Task<WebOsResponse?> TryCallAsync(string uri, JsonObject? payload, CancellationToken cancellationToken)
        {
            try
            {
                return await CallAsync(uri, payload, cancellationToken).ConfigureAwait(false);
            }
            catch (TvControlException)
            {
                return null;
            }
        }

        public async Task<WebOsPower> PowerAsync(CancellationToken cancellationToken) =>
            await TryCallAsync(WebOsProtocol.PowerStateUri, null, cancellationToken).ConfigureAwait(false) is { Ok: true } response
                ? WebOsProtocol.ParsePowerState(response.Payload)
                : WebOsPower.Unknown;

        public ValueTask DisposeAsync() => socket.DisposeAsync();
    }
}
