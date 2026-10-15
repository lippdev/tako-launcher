using System.Diagnostics;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Launcher.Bench;

internal static class Json
{
    public static JsonSerializerOptions Options { get; } = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };
}

/// <summary>Resumo de uma série (milissegundos ou outra unidade).</summary>
internal sealed record Distribution(int Count, double Mean, double P50, double P95, double P99, double Max)
{
    public static Distribution Of(IReadOnlyCollection<double> values)
    {
        if (values.Count == 0)
        {
            return new(0, 0, 0, 0, 0, 0);
        }

        var sorted = values.Order().ToArray();
        return new(sorted.Length, Round(sorted.Average()), Round(Percentile(sorted, 50)), Round(Percentile(sorted, 95)), Round(Percentile(sorted, 99)), Round(sorted[^1]));
    }

    /// <summary>Percentil com interpolação linear entre vizinhos (série já ordenada).</summary>
    public static double Percentile(double[] sorted, double percentile)
    {
        if (sorted.Length == 1)
        {
            return sorted[0];
        }

        double rank = percentile / 100 * (sorted.Length - 1);
        int low = (int)Math.Floor(rank);
        int high = Math.Min(low + 1, sorted.Length - 1);
        return sorted[low] + ((sorted[high] - sorted[low]) * (rank - low));
    }

    public static double Round(double value) => Math.Round(value, 2);
}

/// <summary>Memória do processo em MB: conjunto de trabalho, bytes privados e heap gerenciado.</summary>
internal sealed record MemorySample(double WorkingSetMb, double PrivateMb, double ManagedHeapMb)
{
    private const double Mb = 1024 * 1024;

    public static MemorySample Now()
    {
        using var process = Process.GetCurrentProcess();
        return new(Distribution.Round(process.WorkingSet64 / Mb), Distribution.Round(process.PrivateMemorySize64 / Mb), Distribution.Round(GC.GetTotalMemory(false) / Mb));
    }
}

/// <summary>
/// CPU do processo num intervalo: <see cref="MachinePercent"/> é a fração da máquina inteira (todos os núcleos);
/// <see cref="CorePercent"/>, a fração de um núcleo (100% = um núcleo ocupado o tempo todo).
/// </summary>
internal sealed record CpuSample(double Seconds, double MachinePercent, double CorePercent);

internal sealed class CpuMeter
{
    private readonly Stopwatch _clock = Stopwatch.StartNew();
    private readonly TimeSpan _start = CurrentCpu();

    public CpuSample Stop()
    {
        double wall = _clock.Elapsed.TotalSeconds;
        double cpu = (CurrentCpu() - _start).TotalSeconds;
        double core = wall > 0 ? cpu / wall * 100 : 0;
        return new(Distribution.Round(wall), Distribution.Round(core / Environment.ProcessorCount), Distribution.Round(core));
    }

    private static TimeSpan CurrentCpu()
    {
        using var process = Process.GetCurrentProcess();
        return process.TotalProcessorTime;
    }
}

/// <summary>Abertura medida dentro do processo filho, em ms desde o início do processo.</summary>
internal sealed record StartupPhases(double MainEntered, double ServicesBuilt, double WindowShown, double FirstRender, double Interactive);

/// <summary>
/// Fluidez durante a navegação roteirizada. Intervalos entre quadros vêm do <c>CompositionTarget.Rendering</c>;
/// <see cref="ExpectedFrameMs"/> é a mediana medida com a UI parada (a taxa de atualização efetiva da tela).
/// <see cref="OpenLibraryMs"/>: do comando de abrir a grade da biblioteca até o primeiro quadro desenhado depois dele.
/// </summary>
internal sealed record FluidityResult(
    double Seconds,
    double ExpectedFrameMs,
    double AverageFps,
    Distribution FrameMs,
    int SlowFrames,
    int DroppedFrames,
    Distribution DispatchMs,
    Distribution CommandToFrameMs,
    int Commands,
    Distribution OpenLibraryMs);

/// <summary>Tako com um jogo (carga de CPU) em execução, comparado com a carga sozinha.</summary>
internal sealed record GameOverheadResult(CpuSample TakoCpu, MemorySample TakoMemory, LoadResult? WithTako, LoadResult? Baseline)
{
    /// <summary>Quanto o jogo perdeu com o Tako aberto (%; negativo = ruído a favor).</summary>
    public double? ThroughputLossPercent => WithTako is null || Baseline is null || Baseline.OpsPerSecond <= 0
        ? null
        : Distribution.Round((1 - (WithTako.OpsPerSecond / Baseline.OpsPerSecond)) * 100);
}

/// <summary>Relatório de um processo filho.</summary>
internal sealed record ChildReport
{
    public required string Mode { get; init; }

    public required int Games { get; init; }

    public required StartupPhases Startup { get; init; }

    public MemorySample? MemoryAtStartup { get; init; }

    public CpuSample? IdleCpu { get; init; }

    public MemorySample? MemoryIdle { get; init; }

    public FluidityResult? Fluidity { get; init; }

    public MemorySample? MemoryAfterNavigation { get; init; }

    public GameOverheadResult? Game { get; init; }
}
