namespace ConsoleMode.Runtime.Ipc;

/// <summary>Base dos erros do IPC.</summary>
public class IpcException : Exception
{
    public IpcException()
    {
    }

    public IpcException(string message)
        : base(message)
    {
    }

    public IpcException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}

/// <summary>Os dois lados não têm versão de protocolo em comum. Não adianta reconectar: é preciso atualizar um deles.</summary>
public sealed class IpcVersionMismatchException : IpcException
{
    public IpcVersionMismatchException()
    {
    }

    public IpcVersionMismatchException(string message)
        : base(message)
    {
    }

    public IpcVersionMismatchException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}

/// <summary>Mensagem malformada, grande demais ou fora de ordem. A conexão é encerrada.</summary>
public sealed class IpcProtocolException : IpcException
{
    public IpcProtocolException()
    {
    }

    public IpcProtocolException(string message)
        : base(message)
    {
    }

    public IpcProtocolException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}

/// <summary>A conexão caiu (ou não existe) antes da resposta.</summary>
public sealed class IpcDisconnectedException : IpcException
{
    public IpcDisconnectedException()
        : base("Conexão IPC encerrada.")
    {
    }

    public IpcDisconnectedException(string message)
        : base(message)
    {
    }

    public IpcDisconnectedException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}

/// <summary>
/// O outro lado respondeu com erro (<see cref="Code"/> de <see cref="IpcErrorCodes"/>). Um tratador de comando também
/// pode lançá-la para devolver um código específico.
/// </summary>
public sealed class IpcRemoteException : IpcException
{
    public IpcRemoteException()
        : this(IpcErrorCodes.HandlerFailed, "Erro no outro lado da conexão IPC.")
    {
    }

    public IpcRemoteException(string message)
        : this(IpcErrorCodes.HandlerFailed, message)
    {
    }

    public IpcRemoteException(string message, Exception innerException)
        : base(message, innerException)
    {
        Code = IpcErrorCodes.HandlerFailed;
    }

    public IpcRemoteException(string code, string message)
        : base(message)
    {
        Code = code;
    }

    public string Code { get; }
}
