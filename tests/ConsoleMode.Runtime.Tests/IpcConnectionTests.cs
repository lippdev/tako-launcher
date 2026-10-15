using System.Text.Json;
using ConsoleMode.Runtime.Ipc;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

/// <summary>Comandos, respostas e eventos numa conexão negociada (MVP-003), sobre streams em memória.</summary>
public sealed class IpcConnectionTests
{
    private sealed record Echo(string Text, int Number);

    [Fact]
    public async Task Request_gets_its_response_and_events_arrive_in_order()
    {
        var ct = TestContext.Current.CancellationToken;
        var events = new List<int>();
        var allEvents = new TaskCompletionSource();
        var server = new IpcHandlerMap()
            .OnRequest("echo", payload => IpcConnection.FromElement<Echo>(payload)! with { Number = 42 })
            .OnEvent("tick", payload =>
            {
                lock (events)
                {
                    events.Add(payload!.Value.GetInt32());
                    if (events.Count == 50)
                    {
                        allEvents.TrySetResult();
                    }
                }
            });
        var (client, serverSide) = await ConnectAsync(new IpcConnectionOptions(), new IpcConnectionOptions { Handler = server }, ct);
        await using var _ = client;
        await using var __ = serverSide;

        var reply = await client.RequestAsync<Echo>("echo", new Echo("olá", 1), cancellationToken: ct);
        for (int i = 0; i < 50; i++)
        {
            await client.SendEventAsync("tick", i, ct);
        }

        await allEvents.Task.WaitAsync(TimeSpan.FromSeconds(5), ct);
        Assert.Equal(new Echo("olá", 42), reply);
        Assert.Equal(Enumerable.Range(0, 50), events);
    }

    [Fact]
    public async Task Both_sides_can_send_commands()
    {
        var ct = TestContext.Current.CancellationToken;
        var (client, server) = await ConnectAsync(
            new IpcConnectionOptions { Handler = new IpcHandlerMap().OnRequest("who", _ => "cliente") },
            new IpcConnectionOptions { Handler = new IpcHandlerMap().OnRequest("who", _ => "servidor") },
            ct);
        await using var _ = client;
        await using var __ = server;

        Assert.Equal("servidor", await client.RequestAsync<string>("who", cancellationToken: ct));
        Assert.Equal("cliente", await server.RequestAsync<string>("who", cancellationToken: ct));
    }

    [Fact]
    public async Task Slow_and_fast_commands_complete_out_of_order_with_the_right_results()
    {
        var ct = TestContext.Current.CancellationToken;
        var gate = new TaskCompletionSource();
        var handlers = new IpcHandlerMap()
            .OnRequest("slow", async (_, _, token) =>
            {
                await gate.Task.WaitAsync(token);
                return "lento";
            })
            .OnRequest("fast", _ => "rápido");
        var (client, server) = await ConnectAsync(new IpcConnectionOptions(), new IpcConnectionOptions { Handler = handlers }, ct);
        await using var _ = client;
        await using var __ = server;

        var slow = client.RequestAsync<string>("slow", cancellationToken: ct);
        var fast = await client.RequestAsync<string>("fast", cancellationToken: ct);
        Assert.False(slow.IsCompleted);
        gate.SetResult();

        Assert.Equal("rápido", fast);
        Assert.Equal("lento", await slow);
    }

    [Fact]
    public async Task Responses_sent_in_reverse_order_are_matched_by_id()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, raw) = await RawServerAsync(ct);
        await using var client = await clientStream;

        var first = client.RequestAsync<int>("n", 1, cancellationToken: ct);
        var second = client.RequestAsync<int>("n", 2, cancellationToken: ct);
        var third = client.RequestAsync<int>("n", 3, cancellationToken: ct);
        var requests = new List<IpcEnvelope>();
        for (int i = 0; i < 3; i++)
        {
            requests.Add((await IpcFraming.ReadAsync(raw, IpcProtocol.MaxFrameBytes, ct))!);
        }

        for (int i = requests.Count - 1; i >= 0; i--)
        {
            var request = requests[i];
            int value = request.Payload!.Value.GetInt32() * 10;
            await IpcFraming.WriteAsync(raw, Response(request.Id!.Value, value), ct);
        }

        int[] results = await Task.WhenAll(first, second, third);
        Assert.Equal([10, 20, 30], results);
    }

    [Fact]
    public async Task Unanswered_command_times_out_and_a_late_response_is_dropped()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, raw) = await RawServerAsync(ct);
        await using var client = await clientStream;

        var error = await Assert.ThrowsAsync<TimeoutException>(() => client.RequestAsync("lost", timeout: TimeSpan.FromMilliseconds(100), cancellationToken: ct));
        var lost = await IpcFraming.ReadAsync(raw, IpcProtocol.MaxFrameBytes, ct);
        await IpcFraming.WriteAsync(raw, Response(lost!.Id!.Value, 1), ct);

        // A conexão continua utilizável depois do prazo vencido.
        var next = client.RequestAsync<int>("next", cancellationToken: ct);
        var request = await IpcFraming.ReadAsync(raw, IpcProtocol.MaxFrameBytes, ct);
        await IpcFraming.WriteAsync(raw, Response(request!.Id!.Value, 2), ct);

        Assert.Contains("'lost'", error.Message, StringComparison.Ordinal);
        Assert.Equal(2, await next);
        Assert.Equal(1, client.LateResponses);
        Assert.True(client.IsConnected);
    }

    [Fact]
    public async Task Handler_errors_come_back_with_stable_codes()
    {
        var ct = TestContext.Current.CancellationToken;
        var failures = new List<Exception>();
        var handlers = new IpcHandlerMap()
            .OnRequest("boom", _ => throw new InvalidOperationException("falhou"))
            .OnRequest("denied", _ => throw new IpcRemoteException("denied", "sem permissão"));
        var (client, server) = await ConnectAsync(new IpcConnectionOptions(), new IpcConnectionOptions { Handler = handlers, HandlerError = failures.Add }, ct);
        await using var _ = client;
        await using var __ = server;

        var unknown = await Assert.ThrowsAsync<IpcRemoteException>(() => client.RequestAsync("nope", cancellationToken: ct));
        var boom = await Assert.ThrowsAsync<IpcRemoteException>(() => client.RequestAsync("boom", cancellationToken: ct));
        var denied = await Assert.ThrowsAsync<IpcRemoteException>(() => client.RequestAsync("denied", cancellationToken: ct));

        Assert.Equal(IpcErrorCodes.MethodNotFound, unknown.Code);
        Assert.Equal(IpcErrorCodes.HandlerFailed, boom.Code);
        Assert.Equal("falhou", boom.Message);
        Assert.Equal("denied", denied.Code);
        Assert.IsType<InvalidOperationException>(Assert.Single(failures));
        Assert.True(client.IsConnected);
    }

    [Fact]
    public async Task Pending_command_fails_when_the_connection_drops()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, raw) = await RawServerAsync(ct);
        await using var client = await clientStream;

        var pending = client.RequestAsync("never", timeout: TimeSpan.FromSeconds(30), cancellationToken: ct);
        await IpcFraming.ReadAsync(raw, IpcProtocol.MaxFrameBytes, ct);
        await raw.DisposeAsync();

        await Assert.ThrowsAsync<IpcDisconnectedException>(() => pending);
        await client.Completion.WaitAsync(TimeSpan.FromSeconds(5), ct);
        Assert.False(client.IsConnected);
        await Assert.ThrowsAsync<IpcDisconnectedException>(() => client.SendEventAsync("late", cancellationToken: ct));
    }

    [Fact]
    public async Task Message_with_another_version_after_handshake_closes_the_connection()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, raw) = await RawServerAsync(ct);
        await using var client = await clientStream;

        await IpcFraming.WriteAsync(raw, new IpcEnvelope { Version = 99, Kind = IpcMessageKind.Event, Method = "x" }, ct);

        await client.Completion.WaitAsync(TimeSpan.FromSeconds(5), ct);
        Assert.IsType<IpcProtocolException>(client.CloseReason);
    }

    [Fact]
    public async Task Response_to_an_unknown_id_is_ignored()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, raw) = await RawServerAsync(ct);
        await using var client = await clientStream;

        await IpcFraming.WriteAsync(raw, Response(12345, 0), ct);
        var request = client.RequestAsync<int>("n", cancellationToken: ct);
        var frame = await IpcFraming.ReadAsync(raw, IpcProtocol.MaxFrameBytes, ct);
        await IpcFraming.WriteAsync(raw, Response(frame!.Id!.Value, 5), ct);

        Assert.Equal(5, await request);
        Assert.Equal(1, client.LateResponses);
    }

    private static IpcEnvelope Response(long id, int value) => new()
    {
        Version = IpcProtocol.CurrentVersion,
        Kind = IpcMessageKind.Response,
        Id = id,
        Payload = JsonSerializer.SerializeToElement(value),
    };

    private static async Task<(IpcConnection Client, IpcConnection Server)> ConnectAsync(IpcConnectionOptions client, IpcConnectionOptions server, CancellationToken ct)
    {
        var (clientStream, serverStream) = InMemoryStream.CreatePair();
        var serverTask = IpcConnection.AcceptAsync(serverStream, server, ct);
        var connected = await IpcConnection.ConnectAsync(clientStream, client, ct);
        return (connected, await serverTask);
    }

    /// <summary>Servidor "à mão": faz o handshake e devolve o stream cru, para controlar ordem e prazo das respostas.</summary>
    private static async Task<(Task<IpcConnection> Client, Stream Raw)> RawServerAsync(CancellationToken ct)
    {
        var (clientStream, serverStream) = InMemoryStream.CreatePair();
        var client = IpcConnection.ConnectAsync(clientStream, new IpcConnectionOptions(), ct);
        var hello = await IpcFraming.ReadAsync(serverStream, IpcProtocol.MaxFrameBytes, ct);
        Assert.Equal(IpcMessageKind.Hello, hello!.Kind);
        await IpcFraming.WriteAsync(
            serverStream,
            new IpcEnvelope
            {
                Version = IpcProtocol.CurrentVersion,
                Kind = IpcMessageKind.Welcome,
                Payload = JsonSerializer.SerializeToElement(new IpcWelcome(IpcProtocol.CurrentVersion, "raw", 1), IpcProtocol.SerializerOptions),
            },
            ct);
        await client;
        return (client, serverStream);
    }
}
