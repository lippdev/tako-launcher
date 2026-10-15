namespace Launcher.Core.Supervision;

/// <summary>Mensagens entre a UI e o supervisor pelo IPC (ADR-0012).</summary>
public static class SupervisorMessages
{
    /// <summary>Evento UI → supervisor, enviado pela thread da UI: se ela trava, os heartbeats param.</summary>
    public const string Heartbeat = "ui.heartbeat";

    /// <summary>Evento UI → supervisor: a UI vai sair de propósito (atualização, fim de sessão do Windows); não reiniciar.</summary>
    public const string Closing = "ui.closing";

    /// <summary>Comando UI → supervisor: estado do supervisor (<see cref="SupervisorStatus"/>).</summary>
    public const string Status = "supervisor.status";

    public const string SupervisorPeer = "tako-supervisor";
    public const string UiPeer = "tako-ui";
}

/// <summary>Por que a UI anunciou a saída.</summary>
public enum UiCloseReason
{
    /// <summary>O atualizador vai trocar o executável: o supervisor também precisa sair.</summary>
    Update,

    /// <summary>O Windows está encerrando a sessão (ou o instalador pediu para fechar).</summary>
    SessionEnding,
}

public sealed record UiHeartbeat(int ProcessId, long Sequence);

public sealed record UiClosing(int ProcessId, UiCloseReason Reason);

/// <summary>Resposta de <see cref="SupervisorMessages.Status"/>.</summary>
public sealed record SupervisorStatus(int ProcessId, int Restarts, UiLaunchMode Mode);

/// <summary>Papel deste processo do <c>Tako.exe</c>.</summary>
public enum ProcessRole
{
    /// <summary>Processo leve, sem janela, que inicia e vigia a UI.</summary>
    Supervisor,

    /// <summary>A interface (WPF).</summary>
    Ui,
}

/// <summary>
/// Linha de comando do <c>Tako.exe</c> único (ADR-0012). Aberto normalmente, ele vira supervisor e se inicia de novo
/// com <c>--ui</c>. Fora das versões publicadas (<c>dotnet run</c>, testes), roda a UI direto.
/// </summary>
public static class SupervisorArguments
{
    /// <summary>Este processo é a UI iniciada pelo supervisor.</summary>
    public const string Ui = "--ui";

    /// <summary>Nome do pipe do supervisor (acompanha <see cref="Ui"/>).</summary>
    public const string Pipe = "--supervisor-pipe";

    /// <summary>Abre a UI direto, sem supervisor (diagnóstico).</summary>
    public const string NoSupervisor = "--no-supervisor";

    public const string SafeMode = "--safe-mode";

    /// <summary><c>0</c> desliga e <c>1</c> liga o supervisor, em qualquer build (ex.: testar o supervisor no <c>dotnet run</c>).</summary>
    public const string EnvironmentVariable = "TAKO_SUPERVISOR";

    public static ProcessRole ResolveRole(IReadOnlyList<string> args, string? environmentOverride, bool publishedBuild)
    {
        ArgumentNullException.ThrowIfNull(args);
        if (Has(args, Ui) || Has(args, NoSupervisor))
        {
            return ProcessRole.Ui;
        }

        return environmentOverride?.Trim() switch
        {
            "0" => ProcessRole.Ui,
            "1" => ProcessRole.Supervisor,
            _ => publishedBuild ? ProcessRole.Supervisor : ProcessRole.Ui,
        };
    }

    /// <summary>Pipe do supervisor passado à UI; nulo se a UI roda sozinha.</summary>
    public static string? PipeName(IReadOnlyList<string> args)
    {
        ArgumentNullException.ThrowIfNull(args);
        for (int i = 0; i < args.Count - 1; i++)
        {
            if (string.Equals(args[i], Pipe, StringComparison.OrdinalIgnoreCase) && args[i + 1].Length > 0)
            {
                return args[i + 1];
            }
        }

        return null;
    }

    public static bool Has(IReadOnlyList<string> args, string flag)
    {
        ArgumentNullException.ThrowIfNull(args);
        return args.Contains(flag, StringComparer.OrdinalIgnoreCase);
    }

    /// <summary>Argumentos do usuário repassados à UI (sem os que o supervisor controla).</summary>
    public static IReadOnlyList<string> Passthrough(IReadOnlyList<string> args)
    {
        ArgumentNullException.ThrowIfNull(args);
        var result = new List<string>();
        for (int i = 0; i < args.Count; i++)
        {
            if (string.Equals(args[i], Pipe, StringComparison.OrdinalIgnoreCase))
            {
                i++; // e o valor
            }
            else if (!string.Equals(args[i], Ui, StringComparison.OrdinalIgnoreCase)
                && !string.Equals(args[i], SafeMode, StringComparison.OrdinalIgnoreCase)
                && !string.Equals(args[i], NoSupervisor, StringComparison.OrdinalIgnoreCase))
            {
                result.Add(args[i]);
            }
        }

        return result;
    }

    /// <summary>Linha de comando da UI iniciada pelo supervisor.</summary>
    public static IReadOnlyList<string> ForUi(string pipeName, UiLaunchMode mode, IReadOnlyList<string> passthrough)
    {
        ArgumentException.ThrowIfNullOrEmpty(pipeName);
        ArgumentNullException.ThrowIfNull(passthrough);
        List<string> args = [Ui, Pipe, pipeName];
        if (mode == UiLaunchMode.SafeMode)
        {
            args.Add(SafeMode);
        }

        args.AddRange(passthrough);
        return args;
    }
}
