// Estilos e textos adaptados de lippdev/consolemode: src/ConsoleMode/Services/RtssOverlay.cs (sem o RTSS: o Tako
// desenha o contador numa janela própria, ADR-0017).
using System.Globalization;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>Aparência do contador de FPS: canto da tela do jogo, tamanho e cartão escuro atrás dos números.</summary>
public sealed record FpsOverlayLayout
{
    /// <summary>Um de <see cref="FpsOverlay.Positions"/>.</summary>
    public string Position { get; init; } = FpsOverlay.TopLeft;

    /// <summary>Um de <see cref="FpsOverlay.Sizes"/>.</summary>
    public string Size { get; init; } = FpsOverlay.Small;

    /// <summary>Desenha o contador num cartão translúcido arredondado em vez de direto sobre o jogo.</summary>
    public bool Card { get; init; } = true;
}

/// <summary>Papel de um pedaço de texto do contador: define cor e tamanho na janela.</summary>
public enum OverlayRunKind
{
    /// <summary>O número grande do FPS.</summary>
    Fps,

    /// <summary>Outro número (tempo de quadro, CPU...).</summary>
    Value,

    /// <summary>Unidade depois de um número ("FPS", "ms", "%").</summary>
    Unit,

    /// <summary>Nome de um grupo ("CPU", "1% low").</summary>
    Label,

    /// <summary>API gráfica do jogo (DXGI, D3D9...).</summary>
    Accent,
}

public sealed record OverlayRun(string Text, OverlayRunKind Kind);

/// <summary>
/// O que o contador de FPS do Tako mostra em cada estilo. FPS, tempo de quadro e 1% low vêm do PresentMon
/// (<see cref="FrameStats"/>); CPU, GPU e memória de um <see cref="HardwareSample"/>. Nada aqui desenha: a janela do
/// overlay transforma as linhas em texto.
/// </summary>
public static class FpsOverlay
{
    public const string Off = "off";

    /// <summary>Só o FPS. O id é "compact", o nome que este estilo tinha no Console Mode até a 1.6.0.</summary>
    public const string FpsOnly = "compact";

    /// <summary>FPS e uso de CPU, GPU e RAM numa linha.</summary>
    public const string Compact = "stats";

    /// <summary>FPS, tempo de quadro, 1% low, API e uma linha por peça do PC.</summary>
    public const string Detailed = "detailed";

    public const string Small = "small";
    public const string Medium = "medium";
    public const string Large = "large";

    public const string TopLeft = "top-left";
    public const string TopRight = "top-right";
    public const string BottomLeft = "bottom-left";
    public const string BottomRight = "bottom-right";

    /// <summary>Valor ainda não medido.</summary>
    public const string Missing = "--";

    /// <summary>Ordem do menu da Central de Controle.</summary>
    public static IReadOnlyList<string> Styles { get; } = [Off, FpsOnly, Compact, Detailed];

    public static IReadOnlyList<string> Sizes { get; } = [Small, Medium, Large];

    public static IReadOnlyList<string> Positions { get; } = [TopLeft, TopRight, BottomLeft, BottomRight];

    /// <summary>
    /// Estilo salvo. Os do tempo do RTSS viram o mais próximo: "external" (o OSD do RTSS) vira Só FPS e
    /// "custom" vira Detalhado.
    /// </summary>
    public static string Normalize(string? style)
    {
        if (string.Equals(style, "custom", StringComparison.OrdinalIgnoreCase))
        {
            return Detailed;
        }

        return Styles.FirstOrDefault(s => string.Equals(s, style, StringComparison.OrdinalIgnoreCase)) ?? FpsOnly;
    }

    public static string NormalizeSize(string? size) =>
        Sizes.FirstOrDefault(s => string.Equals(s, size, StringComparison.OrdinalIgnoreCase)) ?? Small;

    /// <summary>Posição salva; a antiga "rtss" (onde o RTSS punha o OSD) vira o canto superior esquerdo, o padrão dele.</summary>
    public static string NormalizePosition(string? position) =>
        Positions.FirstOrDefault(p => string.Equals(p, position, StringComparison.OrdinalIgnoreCase)) ?? TopLeft;

    /// <summary>Escala do texto para cada tamanho.</summary>
    public static double Scale(string? size) => NormalizeSize(size) switch
    {
        Medium => 1.5,
        Large => 2.0,
        _ => 1.0,
    };

    /// <summary>Se o estilo mostra CPU/GPU/memória, que precisam de uma leitura do PC a cada segundo.</summary>
    public static bool NeedsHardware(string? style) => Normalize(style) is Compact or Detailed;

    /// <summary>
    /// Linhas do contador; vazio para <see cref="Off"/>. <paramref name="frames"/> nulo = jogo ainda sem quadros
    /// medidos; <paramref name="sample"/> nulo = PC ainda não lido. Os dois aparecem como "--".
    /// </summary>
    public static IReadOnlyList<IReadOnlyList<OverlayRun>> Rows(string? style, FrameStats? frames, HardwareSample? sample = null)
    {
        style = Normalize(style);
        var rows = new List<IReadOnlyList<OverlayRun>>();
        switch (style)
        {
            case FpsOnly:
                rows.Add(FpsRun(frames));
                break;
            case Compact:
                var line = new List<OverlayRun>(FpsRun(frames));
                AddGroup(line, "CPU", Percent(sample, sample?.CpuUsage));
                AddGroup(line, "GPU", Percent(sample, sample?.GpuUsage));
                AddGroup(line, "RAM", Percent(sample, sample?.RamUsage));
                rows.Add(line);
                break;
            case Detailed:
                var game = new List<OverlayRun>(FpsRun(frames))
                {
                    Gap,
                    new(Number(frames?.FrameTimeMs, "F1"), OverlayRunKind.Value),
                    new(" ms", OverlayRunKind.Unit),
                };
                if (frames is { Runtime.Length: > 0 })
                {
                    game.Add(Gap);
                    game.Add(new OverlayRun(frames.Runtime, OverlayRunKind.Accent));
                }

                rows.Add(game);
                rows.Add([new("1% low ", OverlayRunKind.Label), new(Number(frames?.OnePercentLow, "F0"), OverlayRunKind.Value), new(" FPS", OverlayRunKind.Unit)]);
                AddRow(rows, "CPU", Percent(sample, sample?.CpuUsage), Unit(sample, sample?.CpuClockMhz / 1000, "F1", " GHz"));
                AddRow(rows, "GPU", Percent(sample, sample?.GpuUsage), Unit(sample, sample?.GpuClockMhz, "F0", " MHz"), Unit(sample, sample?.GpuTempC, "F0", "°C"));
                AddRow(rows, "VRAM", Memory(sample, sample?.VramUsedMb, sample?.VramTotalMb));
                AddRow(rows, "RAM", Memory(sample, sample?.RamUsedMb, sample?.RamTotalMb));
                break;
        }

        return rows;
    }

    /// <summary>As linhas como texto simples (uma por linha), para logs, testes e leitores de tela.</summary>
    public static string PlainText(IReadOnlyList<IReadOnlyList<OverlayRun>> rows)
    {
        ArgumentNullException.ThrowIfNull(rows);
        return string.Join("\n", rows.Select(r => string.Concat(r.Select(run => run.Text))));
    }

    /// <summary>"60" para a Central de Controle; "--" sem medida.</summary>
    public static string FpsText(FrameStats? frames) => Number(frames?.Fps, "F0");

    private static OverlayRun Gap => new("   ", OverlayRunKind.Unit);

    private static List<OverlayRun> FpsRun(FrameStats? frames) =>
        [new(FpsText(frames), OverlayRunKind.Fps), new(" FPS", OverlayRunKind.Unit)];

    private static string Number(double? value, string format) =>
        value is { } v && double.IsFinite(v) ? v.ToString(format, CultureInfo.InvariantCulture) : Missing;

    /// <summary>Valor com unidade; nulo quando a peça não informa (some da linha). Sem leitura ainda: "--".</summary>
    private static OverlayRun[]? Unit(HardwareSample? sample, double? value, string format, string unit)
    {
        if (sample is not null && value is not { } v)
        {
            return null;
        }

        return [new(sample is null ? Missing : Number(value, format), OverlayRunKind.Value), new(unit, OverlayRunKind.Unit)];
    }

    private static OverlayRun[]? Percent(HardwareSample? sample, double? value) => Unit(sample, value, "F0", "%");

    /// <summary>"5.2 / 8 GB".</summary>
    private static OverlayRun[]? Memory(HardwareSample? sample, double? usedMb, double? totalMb)
    {
        if (sample is null)
        {
            return [new(Missing, OverlayRunKind.Value), new(" GB", OverlayRunKind.Unit)];
        }

        if (usedMb is not { } used)
        {
            return null;
        }

        string total = totalMb is > 0 ? " / " + (totalMb.Value / 1024).ToString("F0", CultureInfo.InvariantCulture) : string.Empty;
        return [new((used / 1024).ToString("F1", CultureInfo.InvariantCulture), OverlayRunKind.Value), new(total + " GB", OverlayRunKind.Unit)];
    }

    private static void AddGroup(List<OverlayRun> line, string label, OverlayRun[]? value)
    {
        if (value is null)
        {
            return;
        }

        line.Add(Gap);
        line.Add(new OverlayRun(label + " ", OverlayRunKind.Label));
        line.AddRange(value);
    }

    private static void AddRow(List<IReadOnlyList<OverlayRun>> rows, string label, params OverlayRun[]?[] values)
    {
        var shown = values.Where(v => v is not null).ToList();
        if (shown.Count == 0)
        {
            return;
        }

        var row = new List<OverlayRun> { new(label + " ", OverlayRunKind.Label) };
        for (int i = 0; i < shown.Count; i++)
        {
            if (i > 0)
            {
                row.Add(Gap);
            }

            row.AddRange(shown[i]!);
        }

        rows.Add(row);
    }
}

/// <summary>Retângulo em pixels físicos da tela (coordenadas do Windows).</summary>
public readonly record struct PixelRect(int Left, int Top, int Right, int Bottom)
{
    public int Width => Right - Left;

    public int Height => Bottom - Top;
}

/// <summary>Onde pôr a janela do contador num monitor, em pixels físicos (vale em 1080p, 2K e 4K com qualquer escala).</summary>
public static class OverlayPlacement
{
    /// <summary>Distância da borda em pixels lógicos (96 DPI).</summary>
    public const int Margin = 16;

    /// <param name="monitor">Retângulo do monitor do jogo.</param>
    /// <param name="width">Largura da janela em pixels físicos.</param>
    /// <param name="height">Altura da janela em pixels físicos.</param>
    /// <param name="dpiScale">DPI do monitor / 96 (1,5 em 150%).</param>
    public static (int X, int Y) Place(PixelRect monitor, int width, int height, string? position, double dpiScale)
    {
        int margin = (int)Math.Round(Margin * (dpiScale > 0 ? dpiScale : 1));
        width = Math.Clamp(width, 0, Math.Max(0, monitor.Width));
        height = Math.Clamp(height, 0, Math.Max(0, monitor.Height));
        string corner = FpsOverlay.NormalizePosition(position);
        bool right = corner is FpsOverlay.TopRight or FpsOverlay.BottomRight;
        bool bottom = corner is FpsOverlay.BottomLeft or FpsOverlay.BottomRight;
        int x = right ? monitor.Right - margin - width : monitor.Left + margin;
        int y = bottom ? monitor.Bottom - margin - height : monitor.Top + margin;
        return (Math.Max(monitor.Left, x), Math.Max(monitor.Top, y));
    }
}
