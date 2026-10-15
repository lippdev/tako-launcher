// Adaptado de DevTechProfile/CapFrameX (MIT, ver NOTICE.md): source/CapFrameX.Statistics.NetStandard/FrametimeStatisticProvider.cs
// (FPS médio, percentil e "1% low" a partir dos tempos de quadro).
namespace ConsoleMode.Runtime.SessionTools;

/// <summary>
/// Números do contador de FPS. <see cref="Fps"/> e <see cref="FrameTimeMs"/> são do último segundo;
/// <see cref="OnePercentLow"/> dos últimos <see cref="FrameStatsWindow.LowWindow"/>. Nulo quando ainda não há quadros
/// suficientes.
/// </summary>
public sealed record FrameStats(double Fps, double FrameTimeMs, double? OnePercentLow, string Runtime, int ProcessId);

/// <summary>Estatística de tempos de quadro, como no CapFrameX.</summary>
public static class FrameMath
{
    /// <summary>FPS médio: quadros por segundo de tempo somado (não a média dos FPS de cada quadro).</summary>
    public static double AverageFps(IReadOnlyCollection<double> frameTimesMs)
    {
        ArgumentNullException.ThrowIfNull(frameTimesMs);
        double sum = frameTimesMs.Sum();
        return frameTimesMs.Count == 0 || sum <= 0 ? double.NaN : frameTimesMs.Count * 1000 / sum;
    }

    /// <summary>
    /// Quantil <paramref name="p"/> (0..1) pelo método R-8 (aproximadamente não enviesado), o mesmo do
    /// <c>Quantile</c> do MathNet que o CapFrameX usa.
    /// </summary>
    public static double Quantile(IReadOnlyList<double> values, double p)
    {
        ArgumentNullException.ThrowIfNull(values);
        if (values.Count == 0 || double.IsNaN(p))
        {
            return double.NaN;
        }

        var sorted = values.OrderBy(v => v).ToArray();
        int n = sorted.Length;
        if (n == 1)
        {
            return sorted[0];
        }

        const double third = 1.0 / 3.0;
        if (p <= 2 * third / (n + third))
        {
            return sorted[0];
        }

        if (p >= (n - third) / (n + third))
        {
            return sorted[n - 1];
        }

        double h = ((n + third) * p) + third;
        int low = (int)Math.Floor(h); // posição de 1 a n
        return sorted[low - 1] + ((h - low) * (sorted[low] - sorted[low - 1]));
    }

    /// <summary>
    /// "1% low" no sentido do CapFrameX (<c>OnePercentLowAverage</c>): o FPS da média do 1% de quadros mais lentos,
    /// isto é, dos tempos de quadro a partir do percentil 99.
    /// </summary>
    public static double OnePercentLow(IReadOnlyList<double> frameTimesMs)
    {
        ArgumentNullException.ThrowIfNull(frameTimesMs);
        if (frameTimesMs.Count == 0)
        {
            return double.NaN;
        }

        double p99 = Quantile(frameTimesMs, 0.99);
        double slowest = frameTimesMs.Where(t => t >= p99).Average();
        return slowest <= 0 ? double.NaN : 1000 / slowest;
    }
}

/// <summary>
/// Janela deslizante dos quadros do jogo da sessão. Recebe todo quadro que o PresentMon vê, guarda só os dos
/// processos do jogo e, quando o jogo tem mais de uma janela que apresenta (launcher e jogo, por exemplo), mostra a
/// que tem mais FPS. As janelas são medidas em tempo de quadro somado, não em hora de chegada: o PresentMon entrega
/// as linhas em rajadas. Pura: o relógio (só para saber se o jogo parou de apresentar) entra como parâmetro. Não é
/// thread-safe.
/// </summary>
public sealed class FrameStatsWindow
{
    /// <summary>Janela do FPS e do tempo de quadro.</summary>
    public static readonly TimeSpan FpsWindow = TimeSpan.FromSeconds(1);

    /// <summary>Janela do 1% low: curta demais e ele vira só o pior quadro.</summary>
    public static readonly TimeSpan LowWindow = TimeSpan.FromSeconds(10);

    /// <summary>Sem quadros há este tempo (jogo minimizado, tela de carregamento parada): o contador mostra "--".</summary>
    public static readonly TimeSpan StaleAfter = TimeSpan.FromSeconds(2);

    /// <summary>Quadros mínimos para o 1% low ter um número que faça sentido.</summary>
    public const int MinFramesForLow = 30;

    private const int MaxProcesses = 16;
    private readonly Dictionary<int, Track> _tracks = [];
    private HashSet<int> _gameProcesses = [];

    /// <summary>Processos do jogo agora (a lista muda: launcher que abre o jogo, Steam). Quadros de outros são ignorados.</summary>
    public void SetGameProcesses(IEnumerable<int> processIds)
    {
        ArgumentNullException.ThrowIfNull(processIds);
        _gameProcesses = [.. processIds];
        foreach (int gone in _tracks.Keys.Where(pid => !_gameProcesses.Contains(pid)).ToList())
        {
            _tracks.Remove(gone);
        }
    }

    /// <summary>Um quadro que chegou em <paramref name="now"/>. Verdadeiro se for do jogo.</summary>
    public bool Add(PresentSample sample, TimeSpan now)
    {
        if (!_gameProcesses.Contains(sample.ProcessId))
        {
            return false;
        }

        if (!_tracks.TryGetValue(sample.ProcessId, out var track))
        {
            if (_tracks.Count >= MaxProcesses)
            {
                return false;
            }

            track = new Track();
            _tracks[sample.ProcessId] = track;
        }

        track.Add(sample, now);
        return true;
    }

    /// <summary>Números de agora; nulo sem quadros recentes do jogo.</summary>
    public FrameStats? Snapshot(TimeSpan now)
    {
        var best = _tracks
            .Where(t => now - t.Value.LastAt <= StaleAfter)
            .Select(t => (Pid: t.Key, Track: t.Value, Recent: t.Value.Latest(FpsWindow)))
            .OrderByDescending(t => FrameMath.AverageFps(t.Recent))
            .ThenBy(t => t.Pid)
            .FirstOrDefault();
        if (best.Track is null)
        {
            return null;
        }

        double fps = FrameMath.AverageFps(best.Recent);
        double frameTime = best.Recent.Average();
        var all = best.Track.Latest(LowWindow);
        double? low = all.Count >= MinFramesForLow ? FrameMath.OnePercentLow(all) : null;
        return new FrameStats(fps, frameTime, low is { } l && double.IsFinite(l) ? l : null, best.Track.Runtime, best.Pid);
    }

    public void Clear()
    {
        _tracks.Clear();
        _gameProcesses = [];
    }

    private sealed class Track
    {
        // Mais recente no fim; a soma cobre pelo menos LowWindow (quando há quadros para isso).
        private readonly LinkedList<double> _frames = new();
        private double _sum;

        public TimeSpan LastAt { get; private set; }

        public string Runtime { get; private set; } = string.Empty;

        public void Add(PresentSample sample, TimeSpan now)
        {
            _frames.AddLast(sample.FrameTimeMs);
            _sum += sample.FrameTimeMs;
            LastAt = now;
            if (sample.Runtime.Length > 0)
            {
                Runtime = sample.Runtime;
            }

            // Tira os mais antigos enquanto o resto ainda cobre a janela do 1% low.
            double window = LowWindow.TotalMilliseconds;
            while (_frames.First is { } first && _sum - first.Value >= window)
            {
                _sum -= first.Value;
                _frames.RemoveFirst();
            }
        }

        /// <summary>Os quadros mais recentes cuja soma chega a <paramref name="span"/> (ou todos, se não chega). Ao menos um.</summary>
        public List<double> Latest(TimeSpan span)
        {
            double target = span.TotalMilliseconds;
            var result = new List<double>();
            double sum = 0;
            for (var node = _frames.Last; node is not null && (sum < target || result.Count == 0); node = node.Previous)
            {
                result.Add(node.Value);
                sum += node.Value;
            }

            return result;
        }
    }
}
