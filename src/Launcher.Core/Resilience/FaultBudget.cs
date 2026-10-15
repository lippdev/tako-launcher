namespace Launcher.Core.Resilience;

/// <summary>
/// Limite de erros não tratados numa janela de tempo (MVP-011). Um erro isolado mostra a tela de recuperação; muitos
/// seguidos (ex.: um erro que se repete a cada quadro) significam que a tela não tem conserto, e o Tako fecha a tela cheia
/// para devolver o desktop em vez de ficar preso numa tela vazia.
/// </summary>
public sealed class FaultBudget(int maxFaults, TimeSpan window)
{
    public static readonly TimeSpan DefaultWindow = TimeSpan.FromSeconds(10);
    public const int DefaultMaxFaults = 3;

    private readonly Queue<TimeSpan> _faults = new();

    public FaultBudget()
        : this(DefaultMaxFaults, DefaultWindow)
    {
    }

    /// <summary>Registra um erro em <paramref name="now"/>; falso quando o limite da janela estourou.</summary>
    public bool TryRecord(TimeSpan now)
    {
        while (_faults.Count > 0 && now - _faults.Peek() >= window)
        {
            _faults.Dequeue();
        }

        _faults.Enqueue(now);
        return _faults.Count <= maxFaults;
    }
}
