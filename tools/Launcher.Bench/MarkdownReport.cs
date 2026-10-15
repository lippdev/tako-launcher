using System.Globalization;
using System.Text;

namespace Launcher.Bench;

/// <summary>Relatório em Markdown (o mesmo formato da linha de base em <c>docs/marco-1/benchmark.md</c>).</summary>
internal static class MarkdownReport
{
    public static string Render(BenchmarkReport report)
    {
        var c = CultureInfo.InvariantCulture;
        var m = report.Machine;
        var text = new StringBuilder();
        text.Append(c, $"## Benchmark do Tako · {report.Label}\n\n");
        text.Append(c, $"{report.Date:yyyy-MM-dd HH:mm zzz} · Tako {report.TakoVersion}{(report.Commit is null ? string.Empty : $" · commit {report.Commit}")} · ");
        text.Append(c, $"{report.WarmRuns} aberturas a quente por tamanho · repouso {report.IdleSeconds} s · jogo {report.LoadSeconds} s\n\n");

        string gpus = string.Join(", ", m.Displays.Select(d => d.Adapter).OfType<string>().Distinct(StringComparer.Ordinal));
        string screens = string.Join(", ", m.Displays.Select(d => $"{d.Width}x{d.Height} {(d.RefreshHz is int hz ? hz + " Hz" : "? Hz")} {(d.ScalePercent is int s ? s + "%" : "?%")}"));
        text.Append("| Máquina | CPU | GPU | RAM | Tela | Windows |\n|---|---|---|---|---|---|\n");
        text.Append(c, $"| {report.Label} | {m.Cpu ?? "?"} ({m.LogicalProcessors} threads) | {(gpus.Length > 0 ? gpus : "?")} | {(m.MemoryBytes is long b ? (b / (1024d * 1024 * 1024)).ToString("0.0", c) + " GB" : "?")} | {(screens.Length > 0 ? screens : "?")} | {m.WindowsName} {m.WindowsDisplayVersion} ({m.WindowsBuild}) |\n\n");

        text.Append("### Abertura até a primeira tela interativa\n\n");
        text.Append("| Jogos | A frio (ms) | A quente, mediana (ms) | A quente, p95 (ms) | Dentro do processo: serviços / janela / 1º quadro / interativa (ms, mediana a quente) |\n|---:|---:|---:|---:|---|\n");
        foreach (var s in report.Sizes)
        {
            var p = s.WarmPhasesMedian;
            text.Append(c, $"| {s.Games} | {s.ColdStartMs:0} | {s.WarmStart.P50:0} | {s.WarmStart.P95:0} | {p.ServicesBuilt:0} / {p.WindowShown:0} / {p.FirstRender:0} / {p.Interactive:0} |\n");
        }

        text.Append("\n### Memória e CPU\n\n");
        text.Append("| Jogos | RAM ao abrir: WS / privada (MB) | Em repouso: WS / privada (MB) | Após navegar: WS / privada / heap .NET (MB) | CPU em repouso: % da máquina (% de 1 núcleo) |\n|---:|---:|---:|---:|---:|\n");
        foreach (var s in report.Sizes)
        {
            var f = s.Full;
            text.Append(c, $"| {s.Games} | {Mem(f.MemoryAtStartup)} | {Mem(f.MemoryIdle)} | {Mem(f.MemoryAfterNavigation, heap: true)} | {Cpu(f.IdleCpu)} |\n");
        }

        text.Append("\n### Fluidez na navegação roteirizada\n\n");
        text.Append("| Jogos | Quadro esperado (ms) | FPS médio | Tempo de quadro p50 / p95 / p99 / máx (ms) | Quadros lentos (>1,5×) | Quadros perdidos | Comando → próximo quadro p50 / p95 (ms) | Tratamento do comando p95 (ms) |\n|---:|---:|---:|---|---:|---:|---|---:|\n");
        foreach (var s in report.Sizes)
        {
            if (s.Full.Fluidity is not { } f)
            {
                continue;
            }

            text.Append(c, $"| {s.Games} | {f.ExpectedFrameMs:0.0} | {f.AverageFps:0.0} | {f.FrameMs.P50:0.0} / {f.FrameMs.P95:0.0} / {f.FrameMs.P99:0.0} / {f.FrameMs.Max:0.0} | {f.SlowFrames} de {f.FrameMs.Count} | {f.DroppedFrames} | {f.CommandToFrameMs.P50:0.0} / {f.CommandToFrameMs.P95:0.0} | {f.DispatchMs.P95:0.0} |\n");
        }

        text.Append("\n### Com um jogo em execução (carga de CPU em todos os núcleos)\n\n");
        text.Append("| Jogos | CPU do Tako: % da máquina (% de 1 núcleo) | RAM do Tako: WS / privada (MB) | Jogo sozinho (ops/s) | Jogo com o Tako (ops/s) | Perda do jogo |\n|---:|---:|---:|---:|---:|---:|\n");
        foreach (var s in report.Sizes)
        {
            var g = s.Game;
            string loss = g.ThroughputLossPercent is double l ? l.ToString("0.0", c) + "%" : "?";
            text.Append(c, $"| {s.Games} | {Cpu(g.TakoCpu)} | {Mem(g.TakoMemory)} | {s.Baseline.OpsPerSecond:0} | {(g.WithTako is { } w ? w.OpsPerSecond.ToString("0", c) : "?")} | {loss} |\n");
        }

        text.Append("\nWS = conjunto de trabalho (RAM física em uso). Quadros perdidos = intervalos que somam mais de um quadro esperado. ");
        text.Append("Perda do jogo negativa = dentro do ruído do runner.\n");
        return text.ToString();

        string Mem(MemorySample? sample, bool heap = false) => sample is null
            ? "?"
            : heap
                ? string.Create(c, $"{sample.WorkingSetMb:0} / {sample.PrivateMb:0} / {sample.ManagedHeapMb:0}")
                : string.Create(c, $"{sample.WorkingSetMb:0} / {sample.PrivateMb:0}");

        string Cpu(CpuSample? sample) => sample is null ? "?" : string.Create(c, $"{sample.MachinePercent:0.00}% ({sample.CorePercent:0.0}%)");
    }
}
