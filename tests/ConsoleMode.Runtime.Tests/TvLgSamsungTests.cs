using System.Net;
using System.Net.Security;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using ConsoleMode.Runtime.Tv;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

/// <summary>TVs LG (webOS) e Samsung (Tizen): mensagens, respostas de exemplo, detecção e o fluxo com uma TV simulada.</summary>
public sealed class TvLgSamsungTests : IDisposable
{
    // Respostas de exemplo no formato que as TVs mandam (valores inventados).
    private const string LgPrompt = """{"type":"response","id":"register_0","payload":{"pairingType":"PROMPT","returnValue":true}}""";
    private const string LgRegistered = """{"type":"registered","id":"register_0","payload":{"client-key":"chave-de-exemplo"}}""";
    private const string LgDenied = """{"type":"error","id":"register_0","error":"403 User denied access","payload":{}}""";
    private const string LgInputs = """{"type":"response","id":"tako_2","payload":{"devices":[{"id":"HDMI_1","label":"HDMI 1","connected":true},{"id":"HDMI_2","label":"PC","connected":true},{"id":"AV_1","label":"AV"}],"returnValue":true}}""";
    private const string SamsungInfo2019 = """{"device":{"FrameTVSupport":"false","OS":"Tizen","PowerState":"on","TokenAuthSupport":"true","modelName":"QN00Q00","name":"[TV] Sala","type":"Samsung SmartTV"},"name":"[TV] Sala","type":"Samsung SmartTV","version":"2.0.25"}""";
    private const string SamsungInfo2016 = """{"device":{"modelName":"UN00K0000","name":"[TV] Quarto","type":"Samsung SmartTV"},"type":"Samsung SmartTV"}""";

    private readonly string _folder = Path.Combine(Path.GetTempPath(), "tako-tv-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_folder))
        {
            Directory.Delete(_folder, recursive: true);
        }
    }

    [Fact]
    public void Lg_register_carries_the_signed_manifest_and_the_saved_client_key()
    {
        var first = JsonNode.Parse(WebOsProtocol.Register(null))!;
        Assert.Equal("register", (string?)first["type"]);
        Assert.Equal(WebOsProtocol.RegisterId, (string?)first["id"]);
        Assert.Equal("PROMPT", (string?)first["payload"]!["pairingType"]);
        Assert.Null(first["payload"]!["client-key"]);
        Assert.Equal("com.lge.test", (string?)first["payload"]!["manifest"]!["signed"]!["appId"]);
        Assert.StartsWith("eyJhbGdvcml0aG0i", (string?)first["payload"]!["manifest"]!["signatures"]![0]!["signature"], StringComparison.Ordinal);
        var permissions = first["payload"]!["manifest"]!["permissions"]!.AsArray().Select(p => (string?)p).ToList();
        Assert.Contains("CONTROL_INPUT_TV", permissions);
        Assert.Contains("READ_INPUT_DEVICE_LIST", permissions);

        var again = JsonNode.Parse(WebOsProtocol.Register("chave-de-exemplo"))!;
        Assert.Equal("chave-de-exemplo", (string?)again["payload"]!["client-key"]);
    }

    [Fact]
    public void Lg_requests_and_endpoints()
    {
        var request = JsonNode.Parse(WebOsProtocol.Request("tako_1", WebOsProtocol.SwitchInputUri, new JsonObject { ["inputId"] = "HDMI_2" }))!;
        Assert.Equal("request", (string?)request["type"]);
        Assert.Equal("ssap://tv/switchInput", (string?)request["uri"]);
        Assert.Equal("HDMI_2", (string?)request["payload"]!["inputId"]);
        Assert.Null(JsonNode.Parse(WebOsProtocol.Request("tako_2", WebOsProtocol.TurnOffUri))!["payload"]);

        Assert.Equal(["wss://192.168.0.60:3001/", "ws://192.168.0.60:3000/"], WebOsProtocol.Endpoints("192.168.0.60").Select(u => u.ToString()));
        Assert.Equal("HDMI_1", WebOsProtocol.InputId(0));
        Assert.Equal("HDMI_4", WebOsProtocol.InputId(9));
    }

    [Fact]
    public void Lg_register_replies_are_recognized()
    {
        Assert.Equal(WebOsRegisterState.Prompted, WebOsProtocol.ParseRegisterReply(LgPrompt)!.State);
        Assert.Equal(new WebOsRegisterReply(WebOsRegisterState.Registered, "chave-de-exemplo", null), WebOsProtocol.ParseRegisterReply(LgRegistered));
        Assert.Equal(WebOsRegisterState.Denied, WebOsProtocol.ParseRegisterReply(LgDenied)!.State);
        Assert.Contains("403", WebOsProtocol.ParseRegisterReply(LgDenied)!.Error, StringComparison.Ordinal);
        Assert.Null(WebOsProtocol.ParseRegisterReply("""{"type":"response","id":"outro","payload":{}}"""));
        Assert.Throws<InvalidDataException>(() => WebOsProtocol.ParseRegisterReply("isso não é json"));
    }

    [Fact]
    public void Lg_responses_inputs_and_power_state_parse()
    {
        var inputs = WebOsProtocol.ParseResponse(LgInputs);
        Assert.True(inputs.Ok);
        Assert.Equal("tako_2", inputs.Id);
        Assert.Equal([new WebOsInput("HDMI_1", "HDMI 1"), new WebOsInput("HDMI_2", "PC"), new WebOsInput("AV_1", "AV")], WebOsProtocol.ParseInputs(inputs.Payload));

        var refused = WebOsProtocol.ParseResponse("""{"type":"response","id":"tako_3","payload":{"returnValue":false,"errorText":"Unknown input"}}""");
        Assert.False(refused.Ok);
        Assert.Equal("Unknown input", refused.Error);
        Assert.False(WebOsProtocol.ParseResponse("""{"type":"error","id":"tako_4","error":"401 insufficient permissions"}""").Ok);

        Assert.Equal(WebOsPower.Active, Power("Active"));
        Assert.Equal(WebOsPower.ScreenOff, Power("Screen Off"));
        Assert.Equal(WebOsPower.Standby, Power("Active Standby"));
        Assert.Equal(WebOsPower.Unknown, Power("Algo novo"));

        static WebOsPower Power(string state) =>
            WebOsProtocol.ParsePowerState(WebOsProtocol.ParseResponse($$$"""{"type":"response","id":"p","payload":{"state":"{{{state}}}","returnValue":true}}""").Payload);
    }

    [Fact]
    public void Lg_input_choice_uses_the_list_when_there_is_one()
    {
        var listed = WebOsProtocol.ParseInputs(WebOsProtocol.ParseResponse(LgInputs).Payload);
        Assert.Equal("HDMI_2", WebOsProtocol.ChooseInput(listed, 2));
        Assert.Equal("HDMI_3", WebOsProtocol.ChooseInput([], 3)); // TV que não lista: tenta o id mesmo assim
        var error = Assert.Throws<TvControlException>(() => WebOsProtocol.ChooseInput(listed, 4));
        Assert.Contains("HDMI 4", error.Message, StringComparison.Ordinal);
        Assert.Contains("PC", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Samsung_remote_uri_keys_and_hdmi()
    {
        var uri = SamsungTvProtocol.RemoteUri("192.168.0.70", "Tako", "12345678", secure: true);
        Assert.Equal("wss", uri.Scheme);
        Assert.Equal(8002, uri.Port);
        Assert.Equal("/api/v2/channels/samsung.remote.control", uri.AbsolutePath);
        Assert.Contains("name=" + Uri.EscapeDataString(Convert.ToBase64String(Encoding.UTF8.GetBytes("Tako"))), uri.Query, StringComparison.Ordinal);
        Assert.Contains("token=12345678", uri.Query, StringComparison.Ordinal);

        var plain = SamsungTvProtocol.RemoteUri("192.168.0.70", "Tako", null, secure: false);
        Assert.Equal("ws://192.168.0.70:8001/api/v2/channels/samsung.remote.control?name=VGFrbw%3D%3D", plain.ToString());
        Assert.Equal("http://192.168.0.70:8001/api/v2/", SamsungTvProtocol.InfoUri("192.168.0.70").ToString());

        var key = JsonNode.Parse(SamsungTvProtocol.Key("KEY_HDMI2"))!;
        Assert.Equal("ms.remote.control", (string?)key["method"]);
        Assert.Equal("Click", (string?)key["params"]!["Cmd"]);
        Assert.Equal("KEY_HDMI2", (string?)key["params"]!["DataOfCmd"]);
        Assert.Equal("SendRemoteKey", (string?)key["params"]!["TypeOfRemote"]);
        Assert.Equal("KEY_HDMI1", SamsungTvProtocol.HdmiKey(0));
        Assert.Equal("KEY_HDMI4", SamsungTvProtocol.HdmiKey(7));
    }

    [Fact]
    public void Samsung_info_and_channel_events_parse()
    {
        var info = SamsungTvProtocol.ParseInfo(SamsungInfo2019)!;
        Assert.True(info.IsSamsung);
        Assert.True(info.TokenAuth);
        Assert.False(info.IsStandby);
        Assert.Equal("QN00Q00", info.Model);
        Assert.True(SamsungTvProtocol.ParseInfo(SamsungInfo2019.Replace("\"on\"", "\"standby\"", StringComparison.Ordinal))!.IsStandby);

        var old = SamsungTvProtocol.ParseInfo(SamsungInfo2016)!;
        Assert.True(old.IsSamsung);
        Assert.Null(old.PowerState);
        Assert.True(old.TokenAuth); // sem o campo: tenta o canal cifrado
        Assert.False(SamsungTvProtocol.ParseInfo(SamsungInfo2019.Replace("\"true\"", "\"false\"", StringComparison.Ordinal))!.TokenAuth);
        Assert.Null(SamsungTvProtocol.ParseInfo("""{"name":"outra coisa"}"""));
        Assert.Null(SamsungTvProtocol.ParseInfo("<html>"));

        Assert.Equal(new SamsungChannelEvent(SamsungEventKind.Connected, "12345678"), SamsungTvProtocol.ParseEvent("""{"event":"ms.channel.connect","data":{"clients":[],"id":"x","token":"12345678"}}"""));
        Assert.Equal(new SamsungChannelEvent(SamsungEventKind.Connected, "87654321"), SamsungTvProtocol.ParseEvent("""{"event":"ms.channel.connect","data":{"token":87654321}}"""));
        Assert.Equal(new SamsungChannelEvent(SamsungEventKind.Connected, null), SamsungTvProtocol.ParseEvent("""{"event":"ms.channel.connect","data":{"clients":[]}}"""));
        Assert.Equal(SamsungEventKind.Unauthorized, SamsungTvProtocol.ParseEvent("""{"event":"ms.channel.unauthorized"}""").Kind);
        Assert.Equal(SamsungEventKind.TimedOut, SamsungTvProtocol.ParseEvent("""{"event":"ms.channel.timeOut"}""").Kind);
        Assert.Equal(SamsungEventKind.Other, SamsungTvProtocol.ParseEvent("""{"event":"ms.channel.ready"}""").Kind);
        Assert.Throws<InvalidDataException>(() => SamsungTvProtocol.ParseEvent("{"));
    }

    [Theory]
    [InlineData(new[] { 8001, 8002 }, true, TvKind.Samsung)]
    [InlineData(new[] { 3000, 3001 }, false, TvKind.LgWebOs)]
    [InlineData(new[] { 3001 }, false, TvKind.LgWebOs)]
    [InlineData(new[] { 8002 }, false, TvKind.Samsung)]
    [InlineData(new[] { 5555 }, false, TvKind.Android)]
    [InlineData(new[] { 5555, 3000 }, false, TvKind.LgWebOs)]
    [InlineData(new int[0], false, null)]
    public void Detection_decides_by_what_answered(int[] open, bool samsungInfo, TvKind? expected)
    {
        var info = samsungInfo ? SamsungTvProtocol.ParseInfo(SamsungInfo2019) : null;
        Assert.Equal(expected, TvDetection.Decide(new TvProbe(open.ToHashSet(), info)));
    }

    [Fact]
    public async Task Detector_probes_the_known_ports_and_reads_the_samsung_info()
    {
        var transport = new FakeTvTransport { OpenPorts = { 8001, 8002 }, Http = _ => SamsungInfo2019 };
        Assert.Equal(TvKind.Samsung, await new TvDetector(transport).DetectAsync("192.168.0.70", TestContext.Current.CancellationToken));
        Assert.Equal(TvDetection.Ports.Count, transport.ProbedPorts.Count);

        Assert.Null(await new TvDetector(new FakeTvTransport()).DetectAsync("192.168.0.71", TestContext.Current.CancellationToken));
    }

    [Theory]
    [InlineData("wss://192.168.0.60:3001/", SslPolicyErrors.RemoteCertificateChainErrors, true)]
    [InlineData("wss://10.0.0.5:8002/", SslPolicyErrors.RemoteCertificateNameMismatch | SslPolicyErrors.RemoteCertificateChainErrors, true)]
    [InlineData("wss://172.20.1.1:8002/", SslPolicyErrors.RemoteCertificateChainErrors, true)]
    [InlineData("wss://[fe80::1]:3001/", SslPolicyErrors.RemoteCertificateChainErrors, true)]
    [InlineData("wss://8.8.8.8:3001/", SslPolicyErrors.RemoteCertificateChainErrors, false)]
    [InlineData("wss://172.32.0.1:3001/", SslPolicyErrors.RemoteCertificateChainErrors, false)]
    [InlineData("wss://tv.lan:3001/", SslPolicyErrors.RemoteCertificateChainErrors, false)]
    [InlineData("wss://tv.lan:3001/", SslPolicyErrors.None, true)]
    public void Self_signed_certificates_are_accepted_only_for_local_ips(string uri, SslPolicyErrors errors, bool accepted) =>
        Assert.Equal(accepted, TvCertificatePolicy.Accept(new Uri(uri), errors));

    [Fact]
    public void Pairing_keys_are_stored_protected_per_kind_and_ip()
    {
        var protector = new MarkProtector();
        var store = new TvPairingStore(Path.Combine(_folder, "tv-pairing.dat"), protector);
        Assert.Null(store.Get(TvKind.LgWebOs, "192.168.0.60"));

        store.Set(TvKind.LgWebOs, "192.168.0.60", "chave-lg");
        store.Set(TvKind.Samsung, "192.168.0.70", "token-samsung");
        Assert.Equal("chave-lg", store.Get(TvKind.LgWebOs, " 192.168.0.60 "));
        Assert.Null(store.Get(TvKind.Samsung, "192.168.0.60"));
        Assert.Equal("token-samsung", store.Get(TvKind.Samsung, "192.168.0.70"));

        string raw = File.ReadAllText(store.FilePath);
        Assert.StartsWith(MarkProtector.Prefix, raw, StringComparison.Ordinal);

        protector.Readable = false; // pasta copiada para outro usuário/PC
        Assert.Null(store.Get(TvKind.LgWebOs, "192.168.0.60"));
    }

    [Fact]
    public void Tv_kind_round_trips_and_old_settings_stay_android()
    {
        var lg = new TvControlConfig { Enabled = true, Kind = TvKind.LgWebOs, Host = "192.168.0.60", HdmiInput = 2 };
        string json = TvControlConfigJson.Serialize(lg);
        Assert.Contains("\"kind\":\"LgWebOs\"", json, StringComparison.Ordinal);
        Assert.Equal(lg, TvControlConfigJson.Parse(json));
        Assert.Equal(TvKind.Android, TvControlConfigJson.Parse("""{"enabled":true,"host":"192.168.0.50"}""").Kind);
        Assert.Equal(TvKind.Android, TvControlConfigJson.Parse("""{"enabled":true,"host":"tv","kind":7}""").Kind);

        Assert.Equal(TvKind.LgWebOs, TvKinds.Next(TvKind.Android));
        Assert.Equal(TvKind.Samsung, TvKinds.Next(TvKind.LgWebOs));
        Assert.Equal(TvKind.Android, TvKinds.Next(TvKind.Samsung));
    }

    [Fact]
    public async Task Lg_pairs_once_then_switches_to_the_hdmi_input()
    {
        var tv = new FakeLgTv();
        var transport = new FakeTvTransport { Socket = _ => tv.Connect() };
        var events = new List<TvEvent>();
        var store = new TvPairingStore(Path.Combine(_folder, "tv-pairing.dat"), new MarkProtector());
        var control = new WebOsTvControl(store, transport, events.Add);
        var config = new TvControlConfig { Enabled = true, Kind = TvKind.LgWebOs, Host = "192.168.0.60", HdmiInput = 2 };

        await control.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        Assert.Equal([TvEvent.WaitingForApproval, TvEvent.PairingSaved], events);
        Assert.Equal("chave-de-exemplo", store.Get(TvKind.LgWebOs, "192.168.0.60"));
        Assert.Equal("wss://192.168.0.60:3001/", transport.Connected.Single().ToString());
        Assert.Equal("HDMI_2", tv.SwitchedTo);

        // Segunda vez: registra com a chave salva, sem aviso na TV.
        events.Clear();
        await control.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
        Assert.Empty(events);
        Assert.Equal("chave-de-exemplo", tv.LastClientKey);

        await control.StandbyAsync(config, TestContext.Current.CancellationToken);
        Assert.True(tv.TurnedOff);
    }

    [Fact]
    public async Task Lg_refusal_and_closed_ports_become_messages_for_the_user()
    {
        var store = new TvPairingStore(Path.Combine(_folder, "tv-pairing.dat"), new MarkProtector());
        var config = new TvControlConfig { Enabled = true, Kind = TvKind.LgWebOs, Host = "192.168.0.60" };

        var denying = new WebOsTvControl(store, new FakeTvTransport { Socket = _ => new FakeLgTv { Deny = true }.Connect() });
        var denied = await Assert.ThrowsAsync<TvControlException>(() => denying.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Assert.Contains("recusou", denied.Message, StringComparison.Ordinal);

        var closed = new WebOsTvControl(store, new FakeTvTransport { Socket = _ => throw new TvConnectionException("porta fechada") { Refused = true } });
        var off = await Assert.ThrowsAsync<TvControlException>(() => closed.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Assert.Contains("LG Connect Apps", off.Message, StringComparison.Ordinal);
        Assert.Contains("recusou a conexão", off.Message, StringComparison.Ordinal);

        var away = new WebOsTvControl(store, new FakeTvTransport { Socket = _ => throw new TvConnectionException("sem resposta") });
        var unreachable = await Assert.ThrowsAsync<TvControlException>(() => away.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Assert.Contains("não respondeu", unreachable.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Lg_in_network_standby_without_mac_asks_for_the_mac()
    {
        var store = new TvPairingStore(Path.Combine(_folder, "tv-pairing.dat"), new MarkProtector());
        var control = new WebOsTvControl(store, new FakeTvTransport { Socket = _ => new FakeLgTv { PowerState = "Active Standby" }.Connect() });
        var error = await Assert.ThrowsAsync<TvControlException>(() => control.TurnOnAsync(new TvControlConfig { Kind = TvKind.LgWebOs, Host = "192.168.0.60" }, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Assert.Contains("MAC", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Samsung_pairs_with_a_token_then_sends_the_hdmi_key()
    {
        var tv = new FakeSamsungTv();
        var transport = new FakeTvTransport { Socket = tv.Connect, Http = _ => SamsungInfo2019 };
        var events = new List<TvEvent>();
        var store = new TvPairingStore(Path.Combine(_folder, "tv-pairing.dat"), new MarkProtector());
        var control = new SamsungTvControl(store, transport, "Tako", events.Add) { AfterKey = TimeSpan.Zero, AfterPower = TimeSpan.Zero };
        var config = new TvControlConfig { Enabled = true, Kind = TvKind.Samsung, Host = "192.168.0.70", HdmiInput = 3 };

        await control.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        Assert.Equal([TvEvent.WaitingForApproval, TvEvent.PairingSaved], events);
        Assert.Equal("12345678", store.Get(TvKind.Samsung, "192.168.0.70"));
        Assert.Equal(["KEY_HDMI3"], tv.Keys);
        Assert.DoesNotContain("token=", transport.Connected[0].Query, StringComparison.Ordinal);

        events.Clear();
        await control.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
        Assert.Empty(events);
        Assert.Contains("token=12345678", transport.Connected[1].Query, StringComparison.Ordinal);

        await control.StandbyAsync(config, TestContext.Current.CancellationToken);
        Assert.Equal(["KEY_HDMI3", "KEY_HDMI3", "KEY_POWER"], tv.Keys);
    }

    [Fact]
    public async Task Samsung_in_standby_gets_power_on_but_never_power_off()
    {
        string standby = SamsungInfo2019.Replace("\"on\"", "\"standby\"", StringComparison.Ordinal);
        var tv = new FakeSamsungTv();
        var transport = new FakeTvTransport { Socket = tv.Connect, Http = _ => standby };
        var store = new TvPairingStore(Path.Combine(_folder, "tv-pairing.dat"), new MarkProtector());
        var control = new SamsungTvControl(store, transport, "Tako") { AfterKey = TimeSpan.Zero, AfterPower = TimeSpan.Zero };
        var config = new TvControlConfig { Enabled = true, Kind = TvKind.Samsung, Host = "192.168.0.70", HdmiInput = 1 };

        await control.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
        Assert.Equal(["KEY_POWER", "KEY_HDMI1"], tv.Keys);

        tv.Keys.Clear();
        await control.StandbyAsync(config, TestContext.Current.CancellationToken);
        Assert.Empty(tv.Keys); // já em espera: a tecla Power ligaria a TV
    }

    [Fact]
    public async Task Samsung_refusal_and_disabled_remote_access_become_messages_for_the_user()
    {
        var store = new TvPairingStore(Path.Combine(_folder, "tv-pairing.dat"), new MarkProtector());
        var config = new TvControlConfig { Enabled = true, Kind = TvKind.Samsung, Host = "192.168.0.70" };

        var denying = new SamsungTvControl(store, new FakeTvTransport { Socket = new FakeSamsungTv { Deny = true }.Connect, Http = _ => SamsungInfo2019 }, "Tako");
        var denied = await Assert.ThrowsAsync<TvControlException>(() => denying.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Assert.Contains("recusou o Tako", denied.Message, StringComparison.Ordinal);

        var closed = new SamsungTvControl(store, new FakeTvTransport { Socket = _ => throw new TvConnectionException("porta fechada") { Refused = true }, Http = _ => SamsungInfo2019 }, "Tako");
        var off = await Assert.ThrowsAsync<TvControlException>(() => closed.TurnOnAsync(config, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Assert.Contains("Acesso de dispositivos IP remotos", off.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Router_picks_the_control_by_kind()
    {
        var android = new Simulation.SimulatedTvControl();
        var lg = new Simulation.SimulatedTvControl();
        var samsung = new Simulation.SimulatedTvControl();
        var router = new TvControlRouter(android, lg, samsung, new TvDetector(new FakeTvTransport { OpenPorts = { 3001 } }));

        await router.TurnOnAsync(new TvControlConfig { Host = "a", Kind = TvKind.Android }, TimeSpan.Zero, TestContext.Current.CancellationToken);
        await router.TurnOnAsync(new TvControlConfig { Host = "l", Kind = TvKind.LgWebOs }, TimeSpan.Zero, TestContext.Current.CancellationToken);
        await router.StandbyAsync(new TvControlConfig { Host = "s", Kind = TvKind.Samsung }, TestContext.Current.CancellationToken);

        Assert.Equal(["ligar:a"], android.Calls);
        Assert.Equal(["ligar:l"], lg.Calls);
        Assert.Equal(["espera:s"], samsung.Calls);
        Assert.Equal(TvKind.LgWebOs, await router.DetectAsync("192.168.0.60", TestContext.Current.CancellationToken));
    }

    /// <summary>Protetor de mentira: só marca o texto, e pode fingir que não consegue ler.</summary>
    private sealed class MarkProtector : ISecretProtector
    {
        public const string Prefix = "teste:";

        public bool Readable { get; set; } = true;

        public string Protect(string secret) => Prefix + secret;

        public bool TryUnprotect(string stored, out string secret)
        {
            secret = Readable && stored.StartsWith(Prefix, StringComparison.Ordinal) ? stored[Prefix.Length..] : string.Empty;
            return Readable;
        }
    }

    /// <summary>Rede simulada: WebSocket por função, HTTP por função, portas abertas por lista.</summary>
    private sealed class FakeTvTransport : ITvTransport
    {
        public Func<Uri, ITvSocket> Socket { get; init; } = _ => throw new TvConnectionException("sem TV");

        public Func<Uri, string> Http { get; init; } = _ => throw new TvConnectionException("sem TV");

        public HashSet<int> OpenPorts { get; } = [];

        public List<Uri> Connected { get; } = [];

        public List<int> ProbedPorts { get; } = [];

        public Task<ITvSocket> ConnectAsync(Uri uri, CancellationToken cancellationToken)
        {
            var socket = Socket(uri);
            Connected.Add(uri);
            return Task.FromResult(socket);
        }

        public Task<string> GetStringAsync(Uri uri, CancellationToken cancellationToken) => Task.FromResult(Http(uri));

        public Task<bool> IsPortOpenAsync(string host, int port, TimeSpan timeout, CancellationToken cancellationToken)
        {
            lock (ProbedPorts)
            {
                ProbedPorts.Add(port);
            }

            return Task.FromResult(OpenPorts.Contains(port));
        }
    }

    /// <summary>Socket que responde na hora a cada mensagem enviada (a TV simulada decide as respostas).</summary>
    private sealed class ScriptedSocket(Func<string, IEnumerable<string>> reply) : ITvSocket
    {
        private readonly Queue<string> _inbox = new();

        public static ScriptedSocket WithGreeting(string greeting, Func<string, IEnumerable<string>> reply)
        {
            var socket = new ScriptedSocket(reply);
            socket._inbox.Enqueue(greeting);
            return socket;
        }

        public Task SendAsync(string message, CancellationToken cancellationToken)
        {
            foreach (string answer in reply(message))
            {
                _inbox.Enqueue(answer);
            }

            return Task.CompletedTask;
        }

        public async Task<string> ReceiveAsync(CancellationToken cancellationToken)
        {
            if (_inbox.Count == 0)
            {
                await Task.Delay(Timeout.Infinite, cancellationToken);
            }

            return _inbox.Dequeue();
        }

        public ValueTask DisposeAsync() => ValueTask.CompletedTask;
    }

    /// <summary>LG simulada: pede aceite sem client-key, devolve a lista de entradas e guarda o que foi pedido.</summary>
    private sealed class FakeLgTv
    {
        public bool Deny { get; init; }

        public string PowerState { get; init; } = "Active";

        public string? SwitchedTo { get; private set; }

        public string? LastClientKey { get; private set; }

        public bool TurnedOff { get; private set; }

        public ScriptedSocket Connect() => new(Reply);

        private IEnumerable<string> Reply(string message)
        {
            var node = JsonNode.Parse(message)!;
            string id = (string)node["id"]!;
            if ((string?)node["type"] == "register")
            {
                LastClientKey = (string?)node["payload"]!["client-key"];
                if (Deny)
                {
                    return [LgPrompt, LgDenied];
                }

                return LastClientKey is null ? [LgPrompt, LgRegistered] : [LgRegistered];
            }

            string uri = (string)node["uri"]!;
            if (uri == WebOsProtocol.SwitchInputUri)
            {
                SwitchedTo = (string?)node["payload"]!["inputId"];
            }
            else if (uri == WebOsProtocol.TurnOffUri)
            {
                TurnedOff = true;
            }

            var payload = uri switch
            {
                WebOsProtocol.PowerStateUri => new JsonObject { ["state"] = PowerState, ["returnValue"] = true },
                WebOsProtocol.ExternalInputsUri => JsonNode.Parse(LgInputs)!["payload"]!.DeepClone(),
                _ => new JsonObject { ["returnValue"] = true },
            };
            // Uma mensagem de outra assinatura antes da resposta: o cliente deve ignorar.
            return ["""{"type":"response","id":"outro","payload":{}}""", new JsonObject { ["type"] = "response", ["id"] = id, ["payload"] = payload }.ToJsonString()];
        }
    }

    /// <summary>Samsung simulada: devolve token na primeira conexão (ou recusa) e guarda as teclas.</summary>
    private sealed class FakeSamsungTv
    {
        public bool Deny { get; init; }

        public List<string> Keys { get; } = [];

        public ScriptedSocket Connect(Uri uri)
        {
            string greeting = Deny
                ? """{"event":"ms.channel.unauthorized"}"""
                : """{"event":"ms.channel.connect","data":{"clients":[],"id":"x","token":"12345678"}}""";
            return ScriptedSocket.WithGreeting(greeting, message =>
            {
                Keys.Add(JsonDocument.Parse(message).RootElement.GetProperty("params").GetProperty("DataOfCmd").GetString()!);
                return [];
            });
        }
    }

    [Fact]
    public void Local_address_ranges()
    {
        Assert.True(TvCertificatePolicy.IsLocal(IPAddress.Parse("192.168.1.10")));
        Assert.True(TvCertificatePolicy.IsLocal(IPAddress.Parse("::ffff:10.1.2.3")));
        Assert.True(TvCertificatePolicy.IsLocal(IPAddress.Parse("fd00::1")));
        Assert.False(TvCertificatePolicy.IsLocal(IPAddress.Parse("2001:db8::1")));
        Assert.False(TvCertificatePolicy.IsLocal(IPAddress.Parse("1.1.1.1")));
    }
}
