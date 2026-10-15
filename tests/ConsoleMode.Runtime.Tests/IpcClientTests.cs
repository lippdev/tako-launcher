using ConsoleMode.Runtime.Ipc;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

/// <summary>Cliente com reconexão e servidor (MVP-003): em memória e com Named Pipe real (Windows, macOS e Linux).</summary>
public sealed class IpcClientTests
{
    private static readonly TimeSpan Wait = TimeSpan.FromSeconds(10);

    private static IpcClientOptions FastRetry(string peer = "cliente") => new()
    {
        Connection = new IpcConnectionOptions { PeerName = peer },
        ReconnectDelay = TimeSpan.FromMilliseconds(20),
        MaxReconnectDelay = TimeSpan.FromMilliseconds(100),
    };

    private static IpcServer StartServer(IIpcServerTransport transport, string answer)
    {
        var server = new IpcServer(transport, new IpcConnectionOptions
        {
            PeerName = "servidor",
            Handler = new IpcHandlerMap().OnRequest("who", _ => answer),
        });
        server.Start();
        return server;
    }

    [Fact]
    public async Task Client_waits_for_a_server_that_starts_later()
    {
        var ct = TestContext.Current.CancellationToken;
        var endpoint = new InMemoryEndpoint();
        await using var client = new IpcClient(endpoint, FastRetry());
        client.Start();
        await Task.Delay(100, ct);
        Assert.Equal(IpcClientState.Connecting, client.State);

        await using var server = StartServer(endpoint.Listen(), "primeiro");

        Assert.Equal("primeiro", await client.RequestAsync<string>("who", timeout: Wait, cancellationToken: ct));
        Assert.True(endpoint.Attempts > 1);
    }

    [Fact]
    public async Task Client_reconnects_after_the_server_restarts()
    {
        var ct = TestContext.Current.CancellationToken;
        var endpoint = new InMemoryEndpoint();
        var connections = 0;
        var disconnections = 0;
        await using var client = new IpcClient(endpoint, FastRetry());
        client.Connected += (_, _) => Interlocked.Increment(ref connections);
        client.Disconnected += (_, _) => Interlocked.Increment(ref disconnections);

        var first = StartServer(endpoint.Listen(), "primeiro");
        var dropped = new TaskCompletionSource();
        client.Disconnected += (_, _) => dropped.TrySetResult();
        client.Start();
        Assert.Equal("primeiro", await client.RequestAsync<string>("who", timeout: Wait, cancellationToken: ct));

        await first.DisposeAsync();
        await dropped.Task.WaitAsync(Wait, ct);
        await using var second = StartServer(endpoint.Listen(), "segundo");

        Assert.Equal("segundo", await client.RequestAsync<string>("who", timeout: Wait, cancellationToken: ct));
        Assert.Equal(2, client.ConnectionCount);
        Assert.Equal(2, connections);
        Assert.Equal(1, disconnections);
    }

    [Fact]
    public async Task Events_are_dropped_while_disconnected_instead_of_queued()
    {
        var ct = TestContext.Current.CancellationToken;
        await using var client = new IpcClient(new InMemoryEndpoint(), FastRetry());
        client.Start();

        Assert.False(await client.TrySendEventAsync("ui.heartbeat", cancellationToken: ct));
        await Assert.ThrowsAsync<TimeoutException>(() => client.RequestAsync("who", timeout: TimeSpan.FromMilliseconds(100), cancellationToken: ct));
    }

    [Fact]
    public async Task Version_mismatch_stops_retrying()
    {
        var ct = TestContext.Current.CancellationToken;
        var endpoint = new InMemoryEndpoint();
        await using var server = StartServer(endpoint.Listen(), "x");
        var options = FastRetry() with { Connection = new IpcConnectionOptions { MinVersion = 50, MaxVersion = 60 } };
        await using var client = new IpcClient(endpoint, options);
        client.Start();

        var error = await Assert.ThrowsAsync<IpcVersionMismatchException>(() => client.WaitForConnectionAsync(Wait, ct));
        int attempts = endpoint.Attempts;
        await Task.Delay(200, ct);

        Assert.Equal(IpcClientState.Faulted, client.State);
        Assert.Same(error, client.Fault);
        Assert.Equal(attempts, endpoint.Attempts);
    }

    [Fact]
    public async Task Server_reports_failed_handshakes_and_keeps_serving()
    {
        var ct = TestContext.Current.CancellationToken;
        var endpoint = new InMemoryEndpoint();
        await using var server = StartServer(endpoint.Listen(), "ok");
        var failed = new TaskCompletionSource<Exception>();
        server.HandshakeFailed += (_, e) => failed.TrySetResult(e.Error);

        await using (var old = new IpcClient(endpoint, FastRetry() with { Connection = new IpcConnectionOptions { MinVersion = 9, MaxVersion = 9 } }))
        {
            old.Start();
            Assert.IsType<IpcVersionMismatchException>(await failed.Task.WaitAsync(Wait, ct));
        }

        await using var client = new IpcClient(endpoint, FastRetry());
        client.Start();
        Assert.Equal("ok", await client.RequestAsync<string>("who", timeout: Wait, cancellationToken: ct));
        Assert.Single(server.Connections);
    }

    [Fact]
    public async Task Named_pipe_carries_commands_events_and_survives_a_server_restart()
    {
        var ct = TestContext.Current.CancellationToken;
        string pipe = NamedPipeIpc.NewPipeName("tk-test");
        var heartbeat = new TaskCompletionSource<int>();
        IpcServer NewServer(string answer)
        {
            var server = new IpcServer(new NamedPipeServerTransport(pipe), new IpcConnectionOptions
            {
                PeerName = "servidor",
                Handler = new IpcHandlerMap()
                    .OnRequest("who", _ => answer)
                    .OnEvent("beat", payload => heartbeat.TrySetResult(payload!.Value.GetInt32())),
            });
            server.Start();
            return server;
        }

        var first = NewServer("primeiro");
        await using var client = new IpcClient(new NamedPipeClientTransport(pipe, TimeSpan.FromMilliseconds(200)), FastRetry("tako-ui"));
        var dropped = new TaskCompletionSource();
        client.Disconnected += (_, _) => dropped.TrySetResult();
        client.Start();

        Assert.Equal("primeiro", await client.RequestAsync<string>("who", timeout: Wait, cancellationToken: ct));
        Assert.True(await client.TrySendEventAsync("beat", 7, ct));
        Assert.Equal(7, await heartbeat.Task.WaitAsync(Wait, ct));
        Assert.Equal(Environment.ProcessId, client.Current!.RemoteProcessId);

        await first.DisposeAsync();
        await dropped.Task.WaitAsync(Wait, ct);
        await using var second = NewServer("segundo");

        Assert.Equal("segundo", await client.RequestAsync<string>("who", timeout: Wait, cancellationToken: ct));
        Assert.Equal(2, client.ConnectionCount);
    }
}
