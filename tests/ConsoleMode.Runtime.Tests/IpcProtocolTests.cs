using System.Buffers.Binary;
using System.Text;
using ConsoleMode.Runtime.Ipc;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

/// <summary>Contrato do protocolo IPC (MVP-003): versão, quadros, handshake e mensagens inválidas.</summary>
public sealed class IpcProtocolTests
{
    [Theory]
    [InlineData(1, 1, 1, 1, 1)]
    [InlineData(1, 3, 2, 5, 3)]
    [InlineData(2, 5, 1, 3, 3)]
    [InlineData(1, 1, 2, 3, null)]
    [InlineData(3, 4, 1, 2, null)]
    [InlineData(2, 1, 1, 2, null)]
    public void Negotiation_picks_the_highest_common_version(int localMin, int localMax, int remoteMin, int remoteMax, int? expected) =>
        Assert.Equal(expected, IpcProtocol.Negotiate(localMin, localMax, remoteMin, remoteMax));

    [Fact]
    public void Frame_round_trips_with_version_header()
    {
        var envelope = new IpcEnvelope { Version = 1, Kind = IpcMessageKind.Request, Id = 7, Method = "ping" };

        byte[] frame = IpcFraming.Encode(envelope);
        var parsed = IpcFraming.Parse(frame.AsSpan(4));

        Assert.Equal(frame.Length - 4, BinaryPrimitives.ReadInt32LittleEndian(frame));
        Assert.Equal(envelope, parsed);
        Assert.Contains("\"v\":1", Encoding.UTF8.GetString(frame, 4, frame.Length - 4), StringComparison.Ordinal);
        Assert.Contains("\"kind\":\"request\"", Encoding.UTF8.GetString(frame, 4, frame.Length - 4), StringComparison.Ordinal);
    }

    [Theory]
    [InlineData("{not json")]
    [InlineData("{\"v\":1}")] // sem "kind"
    [InlineData("{\"kind\":\"event\"}")] // sem cabeçalho de versão
    [InlineData("{\"v\":1,\"kind\":\"shout\"}")] // tipo desconhecido
    public void Malformed_frames_are_protocol_errors(string json) =>
        Assert.Throws<IpcProtocolException>(() => IpcFraming.Parse(Encoding.UTF8.GetBytes(json)));

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(IpcProtocol.MaxFrameBytes + 1)]
    public async Task Frame_length_outside_the_limit_is_rejected(int length)
    {
        byte[] header = new byte[4];
        BinaryPrimitives.WriteInt32LittleEndian(header, length);
        using var stream = new MemoryStream(header);

        await Assert.ThrowsAsync<IpcProtocolException>(() => IpcFraming.ReadAsync(stream, IpcProtocol.MaxFrameBytes, TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Clean_end_of_stream_reads_as_null()
    {
        using var stream = new MemoryStream();

        Assert.Null(await IpcFraming.ReadAsync(stream, IpcProtocol.MaxFrameBytes, TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Handshake_agrees_on_version_and_identifies_both_sides()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, serverStream) = InMemoryStream.CreatePair();
        var serverTask = IpcConnection.AcceptAsync(serverStream, new IpcConnectionOptions { PeerName = "servidor" }, ct);

        await using var client = await IpcConnection.ConnectAsync(clientStream, new IpcConnectionOptions { PeerName = "cliente" }, ct);
        await using var server = await serverTask;

        Assert.Equal(IpcProtocol.CurrentVersion, client.Version);
        Assert.Equal(IpcProtocol.CurrentVersion, server.Version);
        Assert.Equal("servidor", client.RemotePeer);
        Assert.Equal("cliente", server.RemotePeer);
        Assert.Equal(Environment.ProcessId, client.RemoteProcessId);
    }

    [Fact]
    public async Task Newer_client_falls_back_to_the_servers_version()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, serverStream) = InMemoryStream.CreatePair();
        var serverTask = IpcConnection.AcceptAsync(serverStream, new IpcConnectionOptions { MinVersion = 1, MaxVersion = 2 }, ct);

        await using var client = await IpcConnection.ConnectAsync(clientStream, new IpcConnectionOptions { MinVersion = 2, MaxVersion = 3 }, ct);
        await using var server = await serverTask;

        Assert.Equal(2, client.Version);
        Assert.Equal(2, server.Version);
    }

    [Fact]
    public async Task Incompatible_version_is_refused_with_a_clear_error_on_both_sides()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, serverStream) = InMemoryStream.CreatePair();
        var serverTask = IpcConnection.AcceptAsync(serverStream, new IpcConnectionOptions { PeerName = "tako-supervisor", MinVersion = 1, MaxVersion = 1 }, ct);

        var clientError = await Assert.ThrowsAsync<IpcVersionMismatchException>(() =>
            IpcConnection.ConnectAsync(clientStream, new IpcConnectionOptions { PeerName = "tako-ui", MinVersion = 2, MaxVersion = 3 }, ct));
        var serverError = await Assert.ThrowsAsync<IpcVersionMismatchException>(() => serverTask);

        Assert.Contains("incompatível", clientError.Message, StringComparison.Ordinal);
        Assert.Contains("'tako-supervisor' aceita 1–1", clientError.Message, StringComparison.Ordinal);
        Assert.Contains("'tako-ui' aceita 2–3", clientError.Message, StringComparison.Ordinal);
        Assert.Equal(clientError.Message, serverError.Message);
    }

    [Fact]
    public async Task Server_refuses_a_client_that_skips_hello()
    {
        var ct = TestContext.Current.CancellationToken;
        var (clientStream, serverStream) = InMemoryStream.CreatePair();
        var serverTask = IpcConnection.AcceptAsync(serverStream, new IpcConnectionOptions(), ct);

        await IpcFraming.WriteAsync(clientStream, new IpcEnvelope { Version = 1, Kind = IpcMessageKind.Request, Id = 1, Method = "x" }, ct);
        var reply = await IpcFraming.ReadAsync(clientStream, IpcProtocol.MaxFrameBytes, ct);

        await Assert.ThrowsAsync<IpcProtocolException>(() => serverTask);
        Assert.Equal(IpcMessageKind.Reject, reply!.Kind);
        Assert.Equal(IpcErrorCodes.InvalidMessage, reply.Error!.Code);
    }

    [Fact]
    public async Task Silent_client_times_out_the_handshake()
    {
        var ct = TestContext.Current.CancellationToken;
        var (_, serverStream) = InMemoryStream.CreatePair();

        await Assert.ThrowsAsync<TimeoutException>(() =>
            IpcConnection.AcceptAsync(serverStream, new IpcConnectionOptions { HandshakeTimeout = TimeSpan.FromMilliseconds(100) }, ct));
    }
}
