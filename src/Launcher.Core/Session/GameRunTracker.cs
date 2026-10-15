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
    private List<string> _currentPaths = [];

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

        if (plan.Strategy == GameTrackingStrategy.SteamApp && plan.SteamAppId is null)
        {
            throw new ArgumentException("A estratégia pela Steam precisa do id do jogo na Steam.", nameof(plan));
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

    /// <summary>Executáveis dos processos do jogo na última leitura (sem repetidos), para o perfil do driver de vídeo.</summary>
    public IReadOnlyList<string> GameExecutablePaths => _currentPaths;

    /// <summary>Tempo jogado: da primeira à última vez em que o jogo foi visto.</summary>
    public TimeSpan PlayTime => FirstSeenAt is { } first && LastSeenAt is { } last ? last - first : TimeSpan.Zero;

    public bool IsFinished => Status is GameRunStatus.Exited or GameRunStatus.TimedOut;

    /// <summary>A Steam já confirmou este jogo rodando (<c>RunningAppID</c>): daí em diante ela decide o fim.</summary>
    public bool SteamConfirmed { get; private set; }

    /// <summary>
    /// Aplica uma leitura da lista de processos feita em <paramref name="now"/> e devolve o novo status.
    /// <paramref name="steamRunningAppId"/> é o <c>RunningAppID</c> lido junto (nulo: não lido ou ilegível).
    /// </summary>
    public GameRunStatus Update(IReadOnlyCollection<ProcessEntry> snapshot, DateTimeOffset now, int? steamRunningAppId = null)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        if (IsFinished)
        {
            return Status;
        }

        _current = Plan.Strategy switch
        {
            GameTrackingStrategy.Process => MatchTree(snapshot),
            GameTrackingStrategy.InstallFolder => MatchFolder(snapshot),
            _ => [],
        };
        _currentPaths = [.. snapshot.Where(p => p.ExecutablePath is not null && _current.Contains(p.Id)).Select(p => p.ExecutablePath!).Distinct(StringComparer.OrdinalIgnoreCase)];

        if (IsPresent(steamRunningAppId))
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

    /// <summary>
    /// O jogo está rodando nesta leitura. Com o sinal da Steam: o id dela confirma o início mesmo sem processo visto
    /// (exe fora da pasta, acesso negado); depois de confirmado, a Steam decide o fim, e processos que ficam para trás na
    /// pasta (relatório de falha, launcher) não seguram a sessão. Leitura da Steam ilegível volta aos processos.
    /// </summary>
    private bool IsPresent(int? steamRunningAppId)
    {
        if (Plan.SteamAppId is not { } appId || steamRunningAppId is not { } running)
        {
            return _current.Count > 0;
        }

        bool steamSaysRunning = running == appId;
        SteamConfirmed |= steamSaysRunning;
        return SteamConfirmed ? steamSaysRunning : steamSaysRunning || _current.Count > 0;
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
