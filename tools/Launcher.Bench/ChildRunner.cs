using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Windows.Media;
using System.Windows.Threading;
using ConsoleMode.Runtime;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.UI.Wpf;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Microsoft.Extensions.DependencyInjection;

namespace Launcher.Bench;

/// <summary>
/// Processo filho: monta o Tako real (mesma composição de <see cref="App"/>, mesmo tema, mesma janela) com a biblioteca
/// fictícia e mede. <c>--mode startup</c> só mede a abertura; <c>--mode full</c> segue com repouso, navegação
/// roteirizada e jogo em execução. Escreve <see cref="ReadyLine"/> no stdout quando a primeira tela fica interativa.
/// </summary>
internal static class ChildRunner
{
    public const string ReadyLine = "TAKO-BENCH READY";

    private static readonly DateTime ProcessStart = GetProcessStart();

    public static int Run(Options options)
    {
        double mainEntered = SinceProcessStart();
        string mode = options.Require("mode");
        int games = options.Int("games", 20);
        string art = options.Require("art");
        string reportPath = options.Require("report");

        var app = new App();
        app.InitializeComponent();
        var library = new InMemoryGameLibrary(SyntheticLibrary.Create(games, art, DateTimeOffset.Now), TimeProvider.System);
        using var services = App.BuildServices(library);
        var theme = services.GetRequiredService<ThemeService>().Apply(ThemeService.DefaultTheme);
        if (!theme.Succeeded)
        {
            throw new InvalidOperationException($"Tema padrão não carregou: {theme.Error}");
        }

        var shell = services.GetRequiredService<ShellViewModel>();
        var window = services.GetRequiredService<MainWindow>();
        double servicesBuilt = SinceProcessStart();
        double firstRender = 0;
        window.ContentRendered += (_, _) => firstRender = firstRender == 0 ? SinceProcessStart() : firstRender;
        window.Show();
        double shown = SinceProcessStart();

        // Interativa = primeiro quadro desenhado, um elemento com foco e a fila do dispatcher vazia (nada pendente da abertura).
        WaitUntil(() => firstRender > 0 && window.Navigator.Focused is not null, TimeSpan.FromSeconds(90), "primeira tela interativa");
        Dispatcher.CurrentDispatcher.Invoke(static () => { }, DispatcherPriority.ApplicationIdle);
        double interactive = SinceProcessStart();
        Console.WriteLine(ReadyLine);
        Console.Out.Flush();

        var report = new ChildReport
        {
            Mode = mode,
            Games = games,
            Startup = new StartupPhases(Round(mainEntered), Round(servicesBuilt), Round(shown), Round(firstRender), Round(interactive)),
            MemoryAtStartup = MemorySample.Now(),
        };

        if (mode == "full")
        {
            report = RunFull(report, options, shell, window, Path.GetDirectoryName(Path.GetFullPath(reportPath)) ?? ".");
        }

        if (shell.Overlays.OfType<RecoveryViewModel>().FirstOrDefault(r => r.IsError) is { } error)
        {
            throw new InvalidOperationException($"O Tako mostrou a tela de erro durante o benchmark: {error.Detail}");
        }

        File.WriteAllText(reportPath, JsonSerializer.Serialize(report, Json.Options));
        window.Close();
        return 0;
    }

    private static ChildReport RunFull(ChildReport report, Options options, ShellViewModel shell, MainWindow window, string workDirectory)
    {
        int idleSeconds = options.Int("idle-seconds", 10);
        int loadSeconds = options.Int("load-seconds", 10);

        // Artes da tela inicial e animações de entrada terminam antes do repouso.
        Pump(3);
        var idle = new CpuMeter();
        Pump(idleSeconds);
        var idleCpu = idle.Stop();
        var memoryIdle = MemorySample.Now();

        var fluidity = MeasureFluidity(shell, window);
        Pump(2);
        var memoryAfterNavigation = MemorySample.Now();

        var game = MeasureGame(shell, loadSeconds, workDirectory, report.Games);
        return report with
        {
            IdleCpu = idleCpu,
            MemoryIdle = memoryIdle,
            Fluidity = fluidity,
            MemoryAfterNavigation = memoryAfterNavigation,
            Game = game,
        };
    }

    /// <summary>
    /// Navegação roteirizada (mesmos comandos do controle): grade da biblioteca, detalhes, Início, Configurações e uma
    /// descida longa na grade. Cada comando espera 120 ms, como o direcional segurado.
    /// </summary>
    private static FluidityResult MeasureFluidity(ShellViewModel shell, MainWindow window)
    {
        var frames = new List<double>();
        var commandToFrame = new List<double>();
        var dispatch = new List<double>();
        var last = TimeSpan.Zero;
        long pending = 0;
        void OnRendering(object? sender, EventArgs e)
        {
            var time = ((RenderingEventArgs)e).RenderingTime;
            if (time == last)
            {
                return; // o evento pode disparar mais de uma vez no mesmo quadro
            }

            if (last != TimeSpan.Zero)
            {
                frames.Add((time - last).TotalMilliseconds);
            }

            last = time;
            if (pending != 0)
            {
                commandToFrame.Add(Stopwatch.GetElapsedTime(pending).TotalMilliseconds);
                pending = 0;
            }
        }

        CompositionTarget.Rendering += OnRendering;
        try
        {
            // Calibração: com a UI parada, o intervalo entre quadros é a taxa de atualização efetiva.
            Pump(1.5);
            double expected = frames.Count > 10 ? Distribution.Percentile([.. frames.Order()], 50) : 1000d / 60;
            frames.Clear();

            int commands = 0;
            void Press(AppCommand command, double wait = 0.12)
            {
                long start = Stopwatch.GetTimestamp();
                window.Dispatch(command);
                dispatch.Add(Stopwatch.GetElapsedTime(start).TotalMilliseconds);
                pending = start;
                commands++;
                Pump(wait);
            }

            void Repeat(AppCommand command, int times, double wait = 0.12)
            {
                for (int i = 0; i < times; i++)
                {
                    Press(command, wait);
                }
            }

            var clock = Stopwatch.StartNew();
            shell.OpenLibrary(LibraryFilter.All);
            Pump(1);
            Repeat(AppCommand.Right, 24);
            Repeat(AppCommand.Down, 12);
            Repeat(AppCommand.Up, 6);
            Press(AppCommand.Accept, 1);
            Press(AppCommand.Back, 0.8);
            shell.GoHome();
            Pump(1);
            Press(AppCommand.Down);
            Repeat(AppCommand.Right, 10);
            Repeat(AppCommand.Left, 4);
            shell.OpenSettings();
            Pump(0.8);
            Repeat(AppCommand.Down, 10);
            Repeat(AppCommand.Up, 10);
            shell.OpenLibrary(LibraryFilter.All);
            Pump(1);
            Repeat(AppCommand.Down, 25, 0.1);
            Pump(0.5);
            double seconds = clock.Elapsed.TotalSeconds;

            int slow = frames.Count(f => f > expected * 1.5);
            int dropped = frames.Sum(f => Math.Max(0, (int)Math.Round(f / expected) - 1));
            return new FluidityResult(
                Round(seconds),
                Round(expected),
                Round(frames.Count / seconds),
                Distribution.Of(frames),
                slow,
                dropped,
                Distribution.Of(dispatch),
                Distribution.Of(commandToFrame),
                commands);
        }
        finally
        {
            CompositionTarget.Rendering -= OnRendering;
        }
    }

    /// <summary>
    /// Abre a carga de CPU como jogo adicionado (o Tako inicia o processo e acompanha até ele fechar) e mede a CPU e a
    /// memória do Tako enquanto o "jogo" roda. A perda de desempenho do jogo sai da comparação com a carga sozinha.
    /// </summary>
    private static GameOverheadResult MeasureGame(ShellViewModel shell, int loadSeconds, string workDirectory, int games)
    {
        const int Warmup = 2;
        string self = Environment.ProcessPath ?? throw new InvalidOperationException("Caminho do executável desconhecido");
        string result = Path.Combine(workDirectory, $"load-with-tako-{games}.json");
        File.Delete(result);
        var record = ManualGameRecord.Create("Carga do benchmark", self, DateTimeOffset.Now) with
        {
            Arguments = $"load --seconds {loadSeconds} --warmup {Warmup} --result \"{result}\"",
        };
        var game = shell.Library.AddManual(record);
        Pump(0.5);
        shell.Launch(shell.ItemFor(game));
        WaitUntil(() => shell.IsTrackingRealGame, TimeSpan.FromSeconds(30), "jogo de carga iniciado pelo Tako");

        Pump(Warmup);
        var meter = new CpuMeter();
        Pump(loadSeconds);
        var cpu = meter.Stop();
        var memory = MemorySample.Now();
        WaitUntil(() => shell.Session == SessionState.Browsing, TimeSpan.FromSeconds(loadSeconds + 60), "volta do jogo de carga");

        var withTako = File.Exists(result) ? JsonSerializer.Deserialize<LoadResult>(File.ReadAllText(result), Json.Options) : null;
        return new GameOverheadResult(cpu, memory, withTako, Baseline: null);
    }

    private static void WaitUntil(Func<bool> condition, TimeSpan timeout, string what)
    {
        var clock = Stopwatch.StartNew();
        while (!condition())
        {
            if (clock.Elapsed > timeout)
            {
                throw new TimeoutException($"Tempo esgotado esperando: {what} ({timeout.TotalSeconds:0} s)");
            }

            Pump(0.01);
        }
    }

    private static void Pump(double seconds)
    {
        var frame = new DispatcherFrame();
        var timer = new DispatcherTimer(DispatcherPriority.Background) { Interval = TimeSpan.FromSeconds(seconds) };
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            frame.Continue = false;
        };
        timer.Start();
        Dispatcher.PushFrame(frame);
    }

    private static double SinceProcessStart() => (DateTime.Now - ProcessStart).TotalMilliseconds;

    private static DateTime GetProcessStart()
    {
        using var process = Process.GetCurrentProcess();
        return process.StartTime;
    }

    private static double Round(double value) => Distribution.Round(value);
}
