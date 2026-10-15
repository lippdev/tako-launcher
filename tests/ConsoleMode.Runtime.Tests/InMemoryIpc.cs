using System.Threading.Channels;
using ConsoleMode.Runtime.Ipc;

namespace ConsoleMode.Runtime.Tests;

/// <summary>Par de streams ligados em memória: o que um escreve, o outro lê. Fechar um lado é fim de stream no outro.</summary>
internal sealed class InMemoryStream : Stream
{
    private readonly Channel<byte[]> _inbound;
    private readonly Channel<byte[]> _outbound;
    private ReadOnlyMemory<byte> _leftover;

    private InMemoryStream(Channel<byte[]> inbound, Channel<byte[]> outbound)
    {
        _inbound = inbound;
        _outbound = outbound;
    }

    public override bool CanRead => true;

    public override bool CanSeek => false;

    public override bool CanWrite => true;

    public override long Length => throw new NotSupportedException();

    public override long Position { get => throw new NotSupportedException(); set => throw new NotSupportedException(); }

    public static (Stream Client, Stream Server) CreatePair()
    {
        var toServer = Channel.CreateUnbounded<byte[]>();
        var toClient = Channel.CreateUnbounded<byte[]>();
        return (new InMemoryStream(toClient, toServer), new InMemoryStream(toServer, toClient));
    }

    public override async ValueTask<int> ReadAsync(Memory<byte> buffer, CancellationToken cancellationToken = default)
    {
        if (_leftover.IsEmpty)
        {
            if (!await _inbound.Reader.WaitToReadAsync(cancellationToken).ConfigureAwait(false) || !_inbound.Reader.TryRead(out byte[]? chunk))
            {
                return 0;
            }

            _leftover = chunk;
        }

        int count = Math.Min(buffer.Length, _leftover.Length);
        _leftover[..count].CopyTo(buffer);
        _leftover = _leftover[count..];
        return count;
    }

    public override Task<int> ReadAsync(byte[] buffer, int offset, int count, CancellationToken cancellationToken) =>
        ReadAsync(buffer.AsMemory(offset, count), cancellationToken).AsTask();

    public override ValueTask WriteAsync(ReadOnlyMemory<byte> buffer, CancellationToken cancellationToken = default)
    {
        if (!_outbound.Writer.TryWrite(buffer.ToArray()))
        {
            throw new IOException("Stream em memória fechado.");
        }

        return ValueTask.CompletedTask;
    }

    public override Task WriteAsync(byte[] buffer, int offset, int count, CancellationToken cancellationToken) =>
        WriteAsync(buffer.AsMemory(offset, count), cancellationToken).AsTask();

    public override void Flush()
    {
    }

    public override Task FlushAsync(CancellationToken cancellationToken) => Task.CompletedTask;

    public override int Read(byte[] buffer, int offset, int count) => throw new NotSupportedException();

    public override long Seek(long offset, SeekOrigin origin) => throw new NotSupportedException();

    public override void SetLength(long value) => throw new NotSupportedException();

    public override void Write(byte[] buffer, int offset, int count) => throw new NotSupportedException();

    protected override void Dispose(bool disposing)
    {
        _outbound.Writer.TryComplete();
        _inbound.Writer.TryComplete();
        base.Dispose(disposing);
    }
}

/// <summary>
/// "Endereço" em memória: o cliente conecta enquanto houver um servidor escutando (<see cref="Listen"/>); sem servidor,
/// a conexão falha como um pipe ausente. Permite simular reinício do servidor sem pipe real.
/// </summary>
internal sealed class InMemoryEndpoint : IIpcClientTransport
{
    private readonly object _gate = new();
    private Channel<Stream>? _listening;

    public int Attempts { get; private set; }

    public IIpcServerTransport Listen()
    {
        lock (_gate)
        {
            var listener = new Listener(this);
            _listening = listener.Pending;
            return listener;
        }
    }

    public async Task<Stream> ConnectAsync(CancellationToken cancellationToken)
    {
        Channel<Stream>? listening;
        lock (_gate)
        {
            Attempts++;
            listening = _listening;
        }

        if (listening is null)
        {
            await Task.Delay(20, cancellationToken).ConfigureAwait(false);
            throw new TimeoutException("Ninguém escutando.");
        }

        var (client, server) = InMemoryStream.CreatePair();
        if (!listening.Writer.TryWrite(server))
        {
            throw new TimeoutException("Servidor encerrado.");
        }

        return client;
    }

    private sealed class Listener(InMemoryEndpoint endpoint) : IIpcServerTransport
    {
        public Channel<Stream> Pending { get; } = Channel.CreateUnbounded<Stream>();

        public async Task<Stream> AcceptAsync(CancellationToken cancellationToken) =>
            await Pending.Reader.ReadAsync(cancellationToken).ConfigureAwait(false);

        public ValueTask DisposeAsync()
        {
            lock (endpoint._gate)
            {
                if (endpoint._listening == Pending)
                {
                    endpoint._listening = null;
                }
            }

            Pending.Writer.TryComplete();
            return ValueTask.CompletedTask;
        }
    }
}
