using System.IO.Pipes;

namespace ConsoleMode.Runtime.Ipc;

/// <summary>Como o cliente abre um <see cref="Stream"/> até o servidor (Named Pipe na produção; memória nos testes).</summary>
public interface IIpcClientTransport
{
    /// <exception cref="TimeoutException">Servidor ausente no prazo do transporte.</exception>
    Task<Stream> ConnectAsync(CancellationToken cancellationToken);
}

/// <summary>Como o servidor recebe conexões.</summary>
public interface IIpcServerTransport : IAsyncDisposable
{
    /// <summary>Espera o próximo cliente e devolve o <see cref="Stream"/> dele.</summary>
    Task<Stream> AcceptAsync(CancellationToken cancellationToken);
}

/// <summary>
/// Named Pipe restrito ao usuário atual (<see cref="PipeOptions.CurrentUserOnly"/>, PRD §11): no Windows, o pipe só
/// aceita o mesmo usuário e o cliente confere o dono do servidor; no macOS/Linux o .NET usa um socket de domínio
/// Unix com as mesmas garantias.
/// </summary>
public static class NamedPipeIpc
{
    internal const PipeOptions Options = PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly;

    /// <summary>
    /// Nome curto e difícil de adivinhar. Sockets de domínio Unix limitam o caminho a ~104 caracteres, então o nome fica
    /// abaixo de 32.
    /// </summary>
    public static string NewPipeName(string prefix)
    {
        ArgumentException.ThrowIfNullOrEmpty(prefix);
        return $"{prefix}-{Environment.ProcessId}-{Guid.NewGuid().ToString("N")[..8]}";
    }
}

/// <summary>Cliente de Named Pipe.</summary>
public sealed class NamedPipeClientTransport(string pipeName, TimeSpan connectTimeout) : IIpcClientTransport
{
    public NamedPipeClientTransport(string pipeName)
        : this(pipeName, TimeSpan.FromSeconds(1))
    {
    }

    public string PipeName { get; } = pipeName;

    public async Task<Stream> ConnectAsync(CancellationToken cancellationToken)
    {
        var pipe = new NamedPipeClientStream(".", PipeName, PipeDirection.InOut, NamedPipeIpc.Options);
        try
        {
            await pipe.ConnectAsync((int)connectTimeout.TotalMilliseconds, cancellationToken).ConfigureAwait(false);
            return pipe;
        }
        catch
        {
            await pipe.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }
}

/// <summary>Servidor de Named Pipe: uma instância nova do pipe por cliente.</summary>
public sealed class NamedPipeServerTransport(string pipeName) : IIpcServerTransport
{
    public string PipeName { get; } = pipeName;

    public async Task<Stream> AcceptAsync(CancellationToken cancellationToken)
    {
        var pipe = new NamedPipeServerStream(PipeName, PipeDirection.InOut, NamedPipeServerStream.MaxAllowedServerInstances, PipeTransmissionMode.Byte, NamedPipeIpc.Options);
        try
        {
            await pipe.WaitForConnectionAsync(cancellationToken).ConfigureAwait(false);
            return pipe;
        }
        catch
        {
            await pipe.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }

    public ValueTask DisposeAsync() => ValueTask.CompletedTask;
}
