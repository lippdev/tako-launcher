namespace Launcher.Core.Supervision;

/// <summary>Como a UI é aberta.</summary>
public enum UiLaunchMode
{
    Normal,

    /// <summary><c>--safe-mode</c>: tema padrão embutido, sem atualizações automáticas (MVP-036).</summary>
    SafeMode,
}

/// <summary>Como a UI terminou, do ponto de vista do supervisor.</summary>
public enum UiExitKind
{
    /// <summary>Fechou de propósito (código 0 ou saída anunciada): o supervisor sai junto.</summary>
    Clean,

    /// <summary>Caiu (código diferente de 0, inclusive "voltar ao desktop" do CrashGuard).</summary>
    Crashed,

    /// <summary>Parou de mandar heartbeat e foi encerrada pelo supervisor.</summary>
    Hung,
}

/// <summary>O que o supervisor faz depois que a UI termina.</summary>
public enum SupervisorAction
{
    /// <summary>Sai junto com a UI (fechamento normal, atualização).</summary>
    Exit,

    /// <summary>Abre a UI de novo, no <see cref="SupervisorDecision.Mode"/>.</summary>
    Restart,

    /// <summary>Desiste: sai e deixa o usuário no desktop.</summary>
    GiveUp,
}

public sealed record SupervisorDecision(SupervisorAction Action, UiLaunchMode Mode, int FailuresInWindow, string Reason);

/// <summary>Limites do supervisor (padrões de produção; testes encurtam os prazos).</summary>
public sealed record SupervisorOptions
{
    /// <summary>Reinícios no modo normal dentro de <see cref="RestartWindow"/> antes de tentar o modo seguro.</summary>
    public int MaxRestarts { get; init; } = 3;

    /// <summary>Falhas mais antigas que isto não contam (uma queda por dia não leva ao modo seguro).</summary>
    public TimeSpan RestartWindow { get; init; } = TimeSpan.FromMinutes(5);

    /// <summary>Intervalo dos heartbeats da UI.</summary>
    public TimeSpan HeartbeatInterval { get; init; } = TimeSpan.FromSeconds(2);

    /// <summary>Sem heartbeat por este tempo depois de conectar: a UI travou.</summary>
    public TimeSpan HeartbeatTimeout { get; init; } = TimeSpan.FromSeconds(20);

    /// <summary>Prazo para a UI abrir e mandar o primeiro heartbeat (a primeira abertura do .exe único extrai arquivos).</summary>
    public TimeSpan StartupTimeout { get; init; } = TimeSpan.FromSeconds(60);

    /// <summary>De quanto em quanto tempo o supervisor confere a UI.</summary>
    public TimeSpan CheckInterval { get; init; } = TimeSpan.FromSeconds(1);

    /// <summary>Atualização anunciada que não termina neste prazo: o supervisor encerra a UI e sai.</summary>
    public TimeSpan ClosingGrace { get; init; } = TimeSpan.FromSeconds(30);

    /// <summary>Pausa antes de reabrir a UI.</summary>
    public TimeSpan RestartDelay { get; init; } = TimeSpan.FromMilliseconds(500);
}

/// <summary>
/// Política de reinício do supervisor (MVP-006), pura e sem E/S. Escada: a UI que cai ou trava é reaberta até
/// <see cref="SupervisorOptions.MaxRestarts"/> vezes na janela; depois, uma vez em modo seguro; se ainda falhar, o
/// supervisor desiste e o usuário fica no desktop. Saída limpa da UI encerra o supervisor.
/// </summary>
public sealed class SupervisorPolicy(SupervisorOptions options)
{
    private readonly Queue<TimeSpan> _failures = new();

    public UiLaunchMode Mode { get; private set; }

    /// <summary>Reinícios feitos nesta execução do supervisor.</summary>
    public int Restarts { get; private set; }

    /// <summary>Modo da primeira abertura: <c>Tako.exe --safe-mode</c> já começa no último degrau.</summary>
    public UiLaunchMode Begin(bool safeModeRequested)
    {
        Mode = safeModeRequested ? UiLaunchMode.SafeMode : UiLaunchMode.Normal;
        return Mode;
    }

    /// <summary>
    /// Classifica a saída: anunciada (<see cref="UiCloseReason"/>) é limpa mesmo com código diferente de 0 (o instalador
    /// pode forçar o fechamento); travada é <see cref="UiExitKind.Hung"/>; senão vale o código de saída.
    /// </summary>
    public static UiExitKind Classify(int exitCode, bool hung, UiCloseReason? announced) => (announced, hung, exitCode) switch
    {
        ({ }, _, _) => UiExitKind.Clean,
        (_, true, _) => UiExitKind.Hung,
        (_, _, 0) => UiExitKind.Clean,
        _ => UiExitKind.Crashed,
    };

    /// <summary>Decide o que fazer com a UI que terminou em <paramref name="now"/> (relógio monotônico).</summary>
    public SupervisorDecision OnUiExited(UiExitKind kind, TimeSpan now)
    {
        if (kind == UiExitKind.Clean)
        {
            return new SupervisorDecision(SupervisorAction.Exit, Mode, _failures.Count, "a UI fechou normalmente");
        }

        while (_failures.Count > 0 && now - _failures.Peek() >= options.RestartWindow)
        {
            _failures.Dequeue();
        }

        _failures.Enqueue(now);
        string what = kind == UiExitKind.Hung ? "travou" : "caiu";
        if (Mode == UiLaunchMode.SafeMode)
        {
            return new SupervisorDecision(SupervisorAction.GiveUp, Mode, _failures.Count, $"a UI {what} também no modo seguro: voltando ao desktop");
        }

        Restarts++;
        if (_failures.Count <= options.MaxRestarts)
        {
            return new SupervisorDecision(SupervisorAction.Restart, Mode, _failures.Count, $"a UI {what} ({_failures.Count}/{options.MaxRestarts} na janela)");
        }

        Mode = UiLaunchMode.SafeMode;
        return new SupervisorDecision(SupervisorAction.Restart, Mode, _failures.Count, $"a UI {what} {_failures.Count} vezes seguidas: reabrindo em modo seguro");
    }
}

/// <summary>
/// Detecção de UI travada pelos heartbeats, pura. Antes do primeiro heartbeat vale o prazo de abertura; depois, o de
/// heartbeat. Se o próprio supervisor ficou parado (PC suspenso, hibernação), o intervalo entre conferências denuncia e o
/// prazo recomeça, para não matar uma UI saudável ao acordar.
/// </summary>
public sealed class HeartbeatMonitor(SupervisorOptions options)
{
    private TimeSpan _lastBeat;
    private TimeSpan _lastCheck;
    private bool _connected;

    /// <summary>Heartbeats recebidos desde <see cref="Reset"/>.</summary>
    public long Beats { get; private set; }

    /// <summary>Nova UI iniciada em <paramref name="now"/>.</summary>
    public void Reset(TimeSpan now)
    {
        _lastBeat = now;
        _lastCheck = now;
        _connected = false;
        Beats = 0;
    }

    public void Beat(TimeSpan now)
    {
        _lastBeat = now;
        _connected = true;
        Beats++;
    }

    /// <summary>Conferência periódica: verdadeiro quando a UI passou do prazo sem heartbeat.</summary>
    public bool IsHung(TimeSpan now)
    {
        // Conferências deveriam acontecer a cada CheckInterval; um buraco muito maior é o supervisor que não rodou.
        var suspendGap = options.CheckInterval * 5 + TimeSpan.FromSeconds(5);
        if (now - _lastCheck > suspendGap)
        {
            _lastBeat = now;
        }

        _lastCheck = now;
        var limit = _connected ? options.HeartbeatTimeout : options.StartupTimeout;
        return now - _lastBeat > limit;
    }
}
