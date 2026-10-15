using ConsoleMode.Runtime;

namespace Launcher.Core.Session;

/// <summary>Resultado de um pedido de transição de sessão.</summary>
public readonly record struct SessionTransition(SessionState From, SessionState To, bool Accepted)
{
    /// <summary>Pedido para o estado atual: aceito sem mudar nada (transições idempotentes).</summary>
    public bool IsNoOp => Accepted && From == To;
}

/// <summary>
/// Máquina de estados de sessão (MVP-004, PRD §4.4). Só as transições da tabela são aceitas; uma transição inválida
/// não muda o estado e é reportada em <see cref="Rejected"/>, para que nada seja alterado no Windows.
/// </summary>
public sealed class SessionStateMachine(SessionState initial = SessionState.Idle)
{
    /// <summary>
    /// Fluxo normal: Idle → EnteringFullscreen → Browsing → LaunchingGame → Playing → Returning → Restoring → Browsing.
    /// LaunchingGame → Browsing cobre cancelamento e falha ao abrir (nada foi alterado ainda). Qualquer etapa com efeito
    /// no Windows pode falhar para Error → Recovery → Browsing ou DesktopFallback.
    /// </summary>
    private static readonly Dictionary<SessionState, SessionState[]> Allowed = new()
    {
        [SessionState.Idle] = [SessionState.EnteringFullscreen, SessionState.Browsing],
        [SessionState.EnteringFullscreen] = [SessionState.Browsing, SessionState.Error],
        [SessionState.Browsing] = [SessionState.LaunchingGame, SessionState.Idle],
        [SessionState.LaunchingGame] = [SessionState.Playing, SessionState.Browsing, SessionState.Error],
        [SessionState.Playing] = [SessionState.Returning, SessionState.Error],
        [SessionState.Returning] = [SessionState.Restoring, SessionState.Error],
        [SessionState.Restoring] = [SessionState.Browsing, SessionState.Error],
        [SessionState.Error] = [SessionState.Recovery],
        [SessionState.Recovery] = [SessionState.Browsing, SessionState.DesktopFallback],
        [SessionState.DesktopFallback] = [SessionState.Idle],
    };

    public SessionState State { get; private set; } = initial;

    /// <summary>O estado mudou (não dispara em pedidos idempotentes).</summary>
    public event EventHandler<SessionTransition>? Changed;

    /// <summary>Transição recusada: o estado continua o mesmo.</summary>
    public event EventHandler<SessionTransition>? Rejected;

    /// <summary>Estados alcançáveis a partir de <paramref name="from"/> (sem contar o próprio).</summary>
    public static IReadOnlyList<SessionState> NextStates(SessionState from) => Allowed.TryGetValue(from, out var next) ? next : [];

    public static bool CanTransition(SessionState from, SessionState to) => from == to || NextStates(from).Contains(to);

    public SessionTransition TryTransition(SessionState to)
    {
        var from = State;
        if (!CanTransition(from, to))
        {
            var rejected = new SessionTransition(from, to, Accepted: false);
            Rejected?.Invoke(this, rejected);
            return rejected;
        }

        var transition = new SessionTransition(from, to, Accepted: true);
        if (from != to)
        {
            State = to;
            Changed?.Invoke(this, transition);
        }

        return transition;
    }
}
