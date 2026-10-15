using System.Buffers.Binary;
using System.Text.Json;

namespace ConsoleMode.Runtime.Ipc;

/// <summary>Quadros <c>[tamanho Int32 little-endian][JSON UTF-8]</c> sobre qualquer <see cref="Stream"/>.</summary>
public static class IpcFraming
{
    private const int HeaderBytes = 4;

    /// <summary>Lê um quadro. Nulo = o outro lado fechou entre quadros (fim limpo).</summary>
    public static async Task<IpcEnvelope?> ReadAsync(Stream stream, int maxFrameBytes, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(stream);
        byte[] header = new byte[HeaderBytes];
        int read = await stream.ReadAtLeastAsync(header, HeaderBytes, throwOnEndOfStream: false, cancellationToken).ConfigureAwait(false);
        if (read == 0)
        {
            return null;
        }

        if (read < HeaderBytes)
        {
            throw new IpcDisconnectedException("Conexão IPC encerrada no meio de um quadro.");
        }

        int length = BinaryPrimitives.ReadInt32LittleEndian(header);
        if (length <= 0 || length > maxFrameBytes)
        {
            throw new IpcProtocolException($"Quadro IPC com tamanho inválido: {length} bytes (limite {maxFrameBytes}).");
        }

        byte[] body = new byte[length];
        try
        {
            await stream.ReadExactlyAsync(body, cancellationToken).ConfigureAwait(false);
        }
        catch (EndOfStreamException ex)
        {
            throw new IpcDisconnectedException("Conexão IPC encerrada no meio de um quadro.", ex);
        }

        return Parse(body);
    }

    /// <summary>Converte o JSON de um quadro; JSON inválido ou sem os campos obrigatórios vira erro de protocolo.</summary>
    public static IpcEnvelope Parse(ReadOnlySpan<byte> json)
    {
        try
        {
            return JsonSerializer.Deserialize<IpcEnvelope>(json, IpcProtocol.SerializerOptions)
                ?? throw new IpcProtocolException("Quadro IPC vazio.");
        }
        catch (JsonException ex)
        {
            throw new IpcProtocolException("Quadro IPC com JSON inválido.", ex);
        }
    }

    /// <summary>Quadro pronto para enviar (cabeçalho + JSON).</summary>
    public static byte[] Encode(IpcEnvelope envelope)
    {
        byte[] json = JsonSerializer.SerializeToUtf8Bytes(envelope, IpcProtocol.SerializerOptions);
        byte[] frame = new byte[HeaderBytes + json.Length];
        BinaryPrimitives.WriteInt32LittleEndian(frame, json.Length);
        json.CopyTo(frame.AsSpan(HeaderBytes));
        return frame;
    }

    public static async Task WriteAsync(Stream stream, IpcEnvelope envelope, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(stream);
        byte[] frame = Encode(envelope);
        await stream.WriteAsync(frame, cancellationToken).ConfigureAwait(false);
        await stream.FlushAsync(cancellationToken).ConfigureAwait(false);
    }
}
