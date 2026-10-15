// Adaptado de DevTechProfile/CapFrameX (MIT, ver NOTICE.md): source/CapFrameX.PresentMonInterface/PresentMonServiceConfiguration.cs
// e PresentMonCaptureService.cs (argumentos do modo em tempo real e leitura do stdout).
using System.Globalization;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>
/// Como o Tako roda o PresentMon da Intel (MIT, distribuído junto): saída CSV no stdout, sessão ETW com nome próprio
/// para conviver com CapFrameX, FrameView e Afterburner (que usam outros nomes) e sem filtro de processo: o jogo pode
/// trocar de processo (launcher → jogo, Steam) e o filtro pelos processos da sessão é feito no Tako
/// (<see cref="FrameStatsWindow"/>). Ver ADR-0017.
/// </summary>
public static class PresentMonArguments
{
    /// <summary>Versão distribuída; o build confere o SHA-256 do executável oficial.</summary>
    public const string Version = "2.6.0";

    /// <summary>Nome da sessão ETW do Tako. O padrão do PresentMon ("PresentMon") é o que outras ferramentas usam.</summary>
    public const string SessionName = "Tako_FPS";

    public const string ExeName = "PresentMon.exe";

    /// <summary>Pasta, ao lado do Tako.exe, onde o build põe o PresentMon.</summary>
    public const string Folder = "PresentMon";

    /// <summary>Processos que nunca são o jogo (o próprio Tako e o compositor do Windows): menos linhas para ler.</summary>
    public static IReadOnlyList<string> DefaultExcludes { get; } = ["Tako.exe", "dwm.exe"];

    /// <summary>
    /// Argumentos da captura. <c>--stop_existing_session</c> só derruba uma sessão com o nome do Tako (sobra de uma
    /// execução que caiu); nunca a do CapFrameX ou de outro programa.
    /// </summary>
    public static IReadOnlyList<string> Build(IEnumerable<string>? excludes = null)
    {
        var args = new List<string>
        {
            "--output_stdout",
            "--no_console_stats",
            "--session_name",
            SessionName,
            "--stop_existing_session",
            "--no_track_input",
        };
        foreach (string exclude in excludes ?? DefaultExcludes)
        {
            // Nome com espaço viraria dois argumentos; exe sem ".exe" não casa no PresentMon.
            if (string.IsNullOrWhiteSpace(exclude) || exclude.Any(char.IsWhiteSpace) || exclude.Contains('"', StringComparison.Ordinal))
            {
                continue;
            }

            args.Add("--exclude");
            args.Add(exclude.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) ? exclude : exclude + ".exe");
        }

        return args;
    }
}

/// <summary>Um quadro apresentado por um processo, lido de uma linha do CSV do PresentMon.</summary>
public readonly record struct PresentSample(int ProcessId, string Application, string Runtime, double FrameTimeMs);

/// <summary>
/// Posição das colunas que o Tako usa, tirada do cabeçalho (a ordem e os nomes mudam entre versões e opções do
/// PresentMon). O tempo de quadro é <c>FrameTime</c> (métricas 2.x) ou <c>MsBetweenPresents</c> (1.x e
/// <c>--v1_metrics</c>).
/// </summary>
public sealed record PresentMonColumns(int Count, int Application, int ProcessId, int Runtime, int FrameTime)
{
    private static readonly string[] FrameTimeNames = ["FrameTime", "MsBetweenPresents", "msBetweenPresents"];
    private static readonly string[] RuntimeNames = ["PresentRuntime", "Runtime"];

    /// <summary>Nulo quando a linha não é um cabeçalho do PresentMon com as colunas necessárias.</summary>
    public static PresentMonColumns? FromHeader(string? line)
    {
        if (string.IsNullOrWhiteSpace(line))
        {
            return null;
        }

        var names = line.Trim().Split(',').Select(n => n.Trim()).ToList();
        int Find(params string[] candidates)
        {
            foreach (string candidate in candidates)
            {
                int i = names.FindIndex(n => string.Equals(n, candidate, StringComparison.Ordinal));
                if (i >= 0)
                {
                    return i;
                }
            }

            return -1;
        }

        int app = Find("Application");
        int pid = Find("ProcessID");
        int frame = Find(FrameTimeNames);
        return app < 0 || pid < 0 || frame < 0 ? null : new PresentMonColumns(names.Count, app, pid, Find(RuntimeNames), frame);
    }
}

/// <summary>
/// Lê o stdout do PresentMon linha a linha: a primeira linha útil é o cabeçalho, as outras viram
/// <see cref="PresentSample"/>. Avisos, linhas cortadas, "&lt;error&gt;" e valores "NA" são ignorados. Não é
/// thread-safe: um leitor por processo do PresentMon.
/// </summary>
public sealed class PresentMonReader
{
    /// <summary>Quadro mais longo aceito (10 s): acima disso é pausa, tela de carregamento ou erro de medição.</summary>
    public const double MaxFrameTimeMs = 10_000;

    public PresentMonColumns? Columns { get; private set; }

    public bool TryRead(string? line, out PresentSample sample)
    {
        sample = default;
        if (string.IsNullOrWhiteSpace(line))
        {
            return false;
        }

        if (Columns is null)
        {
            // Antes do cabeçalho o PresentMon pode escrever avisos ("warning: ..."); só o cabeçalho define as colunas.
            Columns = PresentMonColumns.FromHeader(line);
            return false;
        }

        var columns = Columns;
        string[] fields = line.Trim().Split(',');
        if (fields.Length != columns.Count)
        {
            return false; // nome de exe com vírgula, linha cortada ou outro cabeçalho
        }

        string app = fields[columns.Application].Trim();
        if (app.Length == 0 || app == "<error>" || app == "Application")
        {
            return false;
        }

        if (!int.TryParse(fields[columns.ProcessId], NumberStyles.Integer, CultureInfo.InvariantCulture, out int pid) || pid <= 0)
        {
            return false;
        }

        if (!double.TryParse(fields[columns.FrameTime], NumberStyles.Float, CultureInfo.InvariantCulture, out double frameTime)
            || !double.IsFinite(frameTime) || frameTime <= 0 || frameTime > MaxFrameTimeMs)
        {
            return false;
        }

        string runtime = columns.Runtime >= 0 ? fields[columns.Runtime].Trim() : string.Empty;
        sample = new PresentSample(pid, app, runtime, frameTime);
        return true;
    }
}
