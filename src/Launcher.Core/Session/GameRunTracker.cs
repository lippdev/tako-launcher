namespace Launcher.Core.Session;

/// <summary>Processo visto numa leitura da lista de processos. Caminho e início são nulos quando o acesso é negado.</summary>
public readonly record struct ProcessEntry(int Id, int ParentId, string? ExecutablePath = null, DateTimeOffset? StartedAt = null);

public enum GameRunStatus
{
    /// <summary>Aguardando o jogo aparecer (ex.: a Steam baixando uma atualização).</summary>
    Waiting,

    Running,

    /// <summary>O jogo terminou.</summary>
    Exited,

    /// <summary>O jogo não apareceu dentro do prazo.</summary>
    TimedOut,
}

/// <summary>
/// Decisão pura do acompanhamento (MVP-026): a cada leitura da lista de processos diz se o jogo está rodando.
/// A enumeração real fica na UI; aqui só há regras, testáveis sem Windows.
/// </summary>
public sealed class GameRunTracker
{
    /// <summary>Prazo para o jogo aparecer depois de aberto (a Steam pode atualizar antes).</summary>
    public static readonly TimeSpan DefaultStartTimeout = TimeSpan.FromMinutes(2);

    /// <summary>Tempo sem nenhum processo do jogo antes de considerar encerrado (troca de launcher para jogo).</summary>
    public static readonly TimeSpan DefaultExitGrace = TimeSpan.FromSeconds(4);

    // Tolerância de relógio ao comparar o início de um filho com a hora em que o jogo foi aberto.
    private static readonly TimeSpan ClockSkew = TimeSpan.FromSeconds(5);

    private readonly HashSet<int> _tree = [];
    private readonly HashSet<int> _gone = [];
    private readonly DateTimeOffset _launchedAt;
    private readonly TimeSpan _startTimeout;
    private readonly TimeSpan _exitGrace;
    private List<int> _current = [];

    /// <param name="plan">Estratégia e pasta (para <see cref="GameTrackingStrategy.InstallFolder"/>).</param>
    /// <param name="launchedAt">Quando o launcher abriu o jogo.</param>
    /// <param name="rootProcessId">Processo iniciado (para <see cref="GameTrackingStrategy.Process"/>).</param>
    public GameRunTracker(TrackingPlan plan, DateTimeOffset launchedAt, int? rootProcessId = null, TimeSpan? startTimeout = null, TimeSpan? exitGrace = null)
    {
        ArgumentNullException.ThrowIfNull(plan);
        if (plan.Strategy == GameTrackingStrategy.None)
        {
            throw new ArgumentException("Sem estratégia não há o que acompanhar.", nameof(plan));
        }

        if (plan.Strategy == GameTrackingStrategy.InstallFolder && string.IsNullOrWhiteSpace(plan.Folder))
        {
            throw new ArgumentException("A estratégia por pasta precisa da pasta do jogo.", nameof(plan));
        }

        if (plan.Strategy == GameTrackingStrategy.Process && rootProcessId is null)
        {
            throw new ArgumentException("A estratégia por processo precisa do processo iniciado.", nameof(rootProcessId));
        }

        Plan = plan;
        _launchedAt = launchedAt;
        _startTimeout = startTimeout ?? DefaultStartTimeout;
        _exitGrace = exitGrace ?? DefaultExitGrace;
        if (rootProcessId is int root)
        {
            // O processo foi iniciado agora: o jogo já conta como rodando.
            _tree.Add(root);
            _current = [root];
            FirstSeenAt = launchedAt;
            LastSeenAt = launchedAt;
            Status = GameRunStatus.Running;
        }
    }

    public TrackingPlan Plan { get; }

    public GameRunStatus Status { get; private set; } = GameRunStatus.Waiting;

    public DateTimeOffset? FirstSeenAt { get; private set; }

    public DateTimeOffset? LastSeenAt { get; private set; }

    /// <summary>Processos do jogo na última leitura (para trazer a janela do jogo para frente).</summary>
    public IReadOnlyList<int> GameProcessIds => _current;

    /// <summary>Tempo jogado: da primeira à última vez em que o jogo foi visto.</summary>
    public TimeSpan PlayTime => FirstSeenAt is { } first && LastSeenAt is { } last ? last - first : TimeSpan.Zero;

    public bool IsFinished => Status is GameRunStatus.Exited or GameRunStatus.TimedOut;

    /// <summary>Aplica uma leitura da lista de processos feita em <paramref name="now"/> e devolve o novo status.</summary>
    public GameRunStatus Update(IReadOnlyCollection<ProcessEntry> snapshot, DateTimeOffset now)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        if (IsFinished)
        {
            return Status;
        }

        _current = Plan.Strategy == GameTrackingStrategy.Process ? MatchTree(snapshot) : MatchFolder(snapshot);
        if (_current.Count > 0)
        {
            FirstSeenAt ??= now;
            LastSeenAt = now;
            Status = GameRunStatus.Running;
        }
        else if (Status == GameRunStatus.Waiting && now - _launchedAt >= _startTimeout)
        {
            Status = GameRunStatus.TimedOut;
        }
        else if (Status == GameRunStatus.Running && LastSeenAt is { } last && now - last >= _exitGrace)
        {
            Status = GameRunStatus.Exited;
        }

        return Status;
    }

    private List<int> MatchFolder(IReadOnlyCollection<ProcessEntry> snapshot) =>
        [.. snapshot.Where(p => GameTracking.IsInside(p.ExecutablePath, Plan.Folder!)).Select(p => p.Id)];

    /// <summary>
    /// Processo iniciado e descendentes. Filhos de um processo que já fechou continuam contando (o Windows não troca o pai);
    /// um id que sumiu não volta a contar, para não confundir com outro processo que reaproveitou o número.
    /// </summary>
    private List<int> MatchTree(IReadOnlyCollection<ProcessEntry> snapshot)
    {
        bool grew;
        do
        {
            grew = false;
            foreach (var p in snapshot)
            {
                if (!_tree.Contains(p.Id) && _tree.Contains(p.ParentId) && p.Id != p.ParentId && (p.StartedAt is not { } started || started >= _launchedAt - ClockSkew))
                {
                    _tree.Add(p.Id);
                    grew = true;
                }
            }
        }
        while (grew);

        var alive = snapshot.Where(p => _tree.Contains(p.Id) && !_gone.Contains(p.Id)).Select(p => p.Id).ToList();
        foreach (int id in _tree.Where(id => !alive.Contains(id)))
        {
            _gone.Add(id);
        }

        return alive;
    }
}
