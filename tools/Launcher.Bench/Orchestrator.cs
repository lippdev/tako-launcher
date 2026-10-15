using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Text.Json;
using Launcher.Core.Diagnostics;
using Launcher.UI.Wpf.Services;

namespace Launcher.Bench;

/// <summary>Uma falha que reprova o benchmark (processo travou, quebrou ou não chegou à tela interativa).</summary>
internal sealed class BenchFailureException(string message) : Exception(message);

/// <summary>Resultado de um tamanho de biblioteca.</summary>
internal sealed record SizeResult(
    int Games,
    double ColdStartMs,
    StartupPhases ColdPhases,
    IReadOnlyList<double> WarmStartMs,
    Distribution WarmStart,
    StartupPhases WarmPhasesMedian,
    ChildReport Full,
    LoadResult Baseline,
    GameOverheadResult Game);

internal sealed record BenchmarkReport(
    string Label,
    DateTimeOffset Date,
    string TakoVersion,
    string? Commit,
    SystemSnapshot Machine,
    int WarmRuns,
    int IdleSeconds,
    int LoadSeconds,
    IReadOnlyList<SizeResult> Sizes);

/// <summary>
/// Roda o benchmark: para cada tamanho de biblioteca, uma abertura a frio (pasta de dados nova), N aberturas a quente,
/// a carga sozinha (linha de base) e uma rodada completa (repouso, navegação, jogo). Cada medição é um processo novo.
/// </summary>
internal static class Orchestrator
{
    private static readonly TimeSpan ChildTimeout = TimeSpan.FromMinutes(5);

    public static int Run(Options options)
    {
        bool quick = options.Has("quick");
        var sizes = options.Ints("sizes", quick ? [20] : [20, 200, 1000]);
        int warm = options.Int("warm", quick ? 2 : 5);
        int idle = options.Int("idle-seconds", quick ? 3 : 10);
        int load = options.Int("load-seconds", quick ? 4 : 10);
        if (warm < 1 || idle < 1 || load < 1 || sizes.Count == 0 || sizes.Any(s => s < 1))
        {
            throw new ArgumentException("--warm, --idle-seconds, --load-seconds e --sizes precisam ser 1 ou mais");
        }

        string output = Path.GetFullPath(options.Text("out") ?? "benchmark");
        string label = options.Text("label") ?? (Environment.GetEnvironmentVariable("GITHUB_ACTIONS") == "true" ? "GitHub Actions windows-latest" : "local");
        string self = Environment.ProcessPath ?? string.Empty;
        if (!File.Exists(self) || string.Equals(Path.GetFileNameWithoutExtension(self), "dotnet", StringComparison.OrdinalIgnoreCase))
        {
            Console.Error.WriteLine("Rode pelo executável (Launcher.Bench.exe), não por 'dotnet Launcher.Bench.dll': o benchmark abre a si mesmo.");
            return 2;
        }

        string art = Path.Combine(output, "art");
        string runs = Path.Combine(output, "runs");
        Directory.CreateDirectory(runs);
        try
        {
            Console.WriteLine($"Gerando artes de {sizes.Max()} jogos em {art}...");
            SyntheticLibrary.EnsureArt(art, sizes.Max());

            var results = new List<SizeResult>();
            foreach (int games in sizes)
            {
                results.Add(MeasureSize(self, games, art, runs, warm, idle, load));
            }

            string takoVersion = SystemDiagnostics.TakoVersion;
            var machine = SystemDiagnostics.Collect(new DiagnosticsContext([], null, false, null, new FseDetector().Assessment, "Padrão"), TimeProvider.System) with
            {
                TakoVersion = takoVersion,
                InstallKind = "benchmark (Launcher.Bench)",
                DataFolder = null,
            };
            // No CI de PR, GITHUB_SHA é o merge temporário; o job passa o commit da branch em TAKO_BENCH_COMMIT.
            string? commit = (Environment.GetEnvironmentVariable("TAKO_BENCH_COMMIT") ?? Environment.GetEnvironmentVariable("GITHUB_SHA")) is { Length: >= 7 } sha ? sha[..7] : null;
            var report = new BenchmarkReport(label, DateTimeOffset.Now, takoVersion, commit, machine, warm, idle, load, results);

            File.WriteAllText(Path.Combine(output, "benchmark.json"), JsonSerializer.Serialize(report, Json.Options));
            string markdown = MarkdownReport.Render(report);
            File.WriteAllText(Path.Combine(output, "benchmark.md"), markdown);
            Console.WriteLine();
            Console.WriteLine(markdown);
            Console.WriteLine($"Relatório em {output}");
            return 0;
        }
        catch (Exception ex) when (ex is BenchFailureException or IOException or JsonException or TimeoutException or InvalidOperationException)
        {
            Console.Error.WriteLine($"Benchmark falhou: {ex.Message}");
            return 1;
        }
    }

    private static SizeResult MeasureSize(string self, int games, string art, string runs, int warm, int idle, int load)
    {
        Console.WriteLine($"[{games} jogos] abertura a frio (pasta de dados nova)...");
        if (Directory.Exists(AppPaths.DataRoot))
        {
            Directory.Delete(AppPaths.DataRoot, recursive: true);
        }

        var cold = RunChild(self, runs, $"cold-{games}", ["--mode", "startup", "--games", Text(games), "--art", art]);

        var warmRuns = new List<(double Ms, ChildReport Report)>();
        for (int i = 0; i < warm; i++)
        {
            Console.WriteLine($"[{games} jogos] abertura a quente {i + 1}/{warm}...");
            warmRuns.Add(RunChild(self, runs, $"warm-{games}-{i + 1}", ["--mode", "startup", "--games", Text(games), "--art", art]));
        }

        // Linha de base: a melhor de duas rodadas da carga sozinha (a primeira costuma sair mais lenta com a máquina fria).
        Console.WriteLine($"[{games} jogos] carga sozinha (linha de base do jogo, 2 rodadas)...");
        var baseline = new[] { RunLoad(self, runs, $"{games}-1", load), RunLoad(self, runs, $"{games}-2", load) }.MaxBy(r => r.OpsPerSecond) ?? throw new BenchFailureException("Sem linha de base da carga");

        Console.WriteLine($"[{games} jogos] rodada completa: repouso {idle} s, navegação, jogo {load} s...");
        var full = RunChild(self, runs, $"full-{games}", ["--mode", "full", "--games", Text(games), "--art", art, "--idle-seconds", Text(idle), "--load-seconds", Text(load)]);
        var game = (full.Report.Game ?? throw new BenchFailureException("A rodada completa não mediu o jogo em execução")) with { Baseline = baseline };

        var phases = warmRuns.Select(r => r.Report.Startup).ToList();
        double MedianOf(Func<StartupPhases, double> pick) => Distribution.Round(Distribution.Percentile([.. phases.Select(pick).Order()], 50));
        var warmPhases = new StartupPhases(MedianOf(p => p.MainEntered), MedianOf(p => p.ServicesBuilt), MedianOf(p => p.WindowShown), MedianOf(p => p.FirstRender), MedianOf(p => p.Interactive));

        var warmMs = warmRuns.Select(r => r.Ms).ToList();
        return new SizeResult(games, cold.Ms, cold.Report.Startup, warmMs, Distribution.Of(warmMs), warmPhases, full.Report with { Game = null }, baseline, game);
    }

    /// <summary>Abre um processo filho e mede da criação do processo até a linha "pronto" (primeira tela interativa).</summary>
    private static (double Ms, ChildReport Report) RunChild(string self, string runs, string name, IReadOnlyList<string> arguments)
    {
        string reportFile = Path.Combine(runs, name + ".json");
        File.Delete(reportFile);
        var start = new ProcessStartInfo(self)
        {
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };
        start.ArgumentList.Add("child");
        foreach (string argument in arguments)
        {
            start.ArgumentList.Add(argument);
        }

        start.ArgumentList.Add("--report");
        start.ArgumentList.Add(reportFile);

        long readyTicks = 0;
        var errors = new List<string>();
        var clock = Stopwatch.StartNew();
        using var process = Process.Start(start) ?? throw new BenchFailureException("Não foi possível iniciar o processo filho");
        process.OutputDataReceived += (_, e) =>
        {
            if (e.Data == ChildRunner.ReadyLine)
            {
                Interlocked.CompareExchange(ref readyTicks, clock.Elapsed.Ticks, 0);
            }
        };
        process.ErrorDataReceived += (_, e) =>
        {
            if (e.Data is not null)
            {
                lock (errors)
                {
                    errors.Add(e.Data);
                }
            }
        };
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();
        if (!process.WaitForExit(ChildTimeout))
        {
            process.Kill(entireProcessTree: true);
            throw new BenchFailureException($"{name}: o Tako travou (sem terminar em {ChildTimeout.TotalMinutes:0} min)");
        }

        process.WaitForExit(); // esvazia stdout/stderr
        string stderr;
        lock (errors)
        {
            stderr = string.Join(Environment.NewLine, errors.TakeLast(30));
        }

        if (process.ExitCode != 0)
        {
            throw new BenchFailureException($"{name}: o processo terminou com código {process.ExitCode}{Environment.NewLine}{stderr}");
        }

        long ticks = Interlocked.Read(ref readyTicks);
        if (ticks == 0 || !File.Exists(reportFile))
        {
            throw new BenchFailureException($"{name}: o Tako não chegou à primeira tela interativa{Environment.NewLine}{stderr}");
        }

        var report = JsonSerializer.Deserialize<ChildReport>(File.ReadAllText(reportFile), Json.Options)
            ?? throw new BenchFailureException($"{name}: relatório vazio");
        return (Distribution.Round(TimeSpan.FromTicks(ticks).TotalMilliseconds), report);
    }

    private static LoadResult RunLoad(string self, string runs, string name, int seconds)
    {
        string result = Path.Combine(runs, $"load-baseline-{name}.json");
        File.Delete(result);
        var start = new ProcessStartInfo(self) { UseShellExecute = false };
        foreach (string argument in new[] { "load", "--seconds", Text(seconds), "--warmup", "2", "--result", result })
        {
            start.ArgumentList.Add(argument);
        }

        using var process = Process.Start(start) ?? throw new BenchFailureException("Não foi possível iniciar a carga");
        if (!process.WaitForExit(TimeSpan.FromSeconds(seconds + 60)))
        {
            process.Kill(entireProcessTree: true);
            throw new BenchFailureException("A carga de CPU travou");
        }

        return process.ExitCode == 0 && File.Exists(result)
            ? JsonSerializer.Deserialize<LoadResult>(File.ReadAllText(result), Json.Options) ?? throw new BenchFailureException("Resultado da carga vazio")
            : throw new BenchFailureException($"A carga de CPU terminou com código {process.ExitCode}");
    }

    private static string Text(int value) => value.ToString(CultureInfo.InvariantCulture);
}
