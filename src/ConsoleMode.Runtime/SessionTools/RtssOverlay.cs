// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/RtssOverlay.cs
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>
/// Aparência do contador de FPS. Tamanho, fundo, cor e posição valem para todo estilo; os itens e a linha única são o
/// que o estilo Personalizado mostra.
/// </summary>
public sealed record FpsOverlayLayout
{
    public bool ShowApi { get; init; } = true;

    public bool ShowFps { get; init; } = true;

    public bool ShowFrameTime { get; init; } = true;

    public bool ShowCpu { get; init; }

    public bool ShowCpuClock { get; init; }

    public bool ShowGpu { get; init; }

    public bool ShowGpuClock { get; init; }

    public bool ShowGpuTemp { get; init; }

    public bool ShowVram { get; init; }

    public bool ShowRam { get; init; }

    /// <summary>Tudo numa linha em vez de uma linha por grupo.</summary>
    public bool SingleLine { get; init; }

    /// <summary>RRGGBB dos números; um de <see cref="RtssOverlay.Colors"/>.</summary>
    public string Color { get; init; } = RtssOverlay.Colors[0];

    /// <summary>Um de <see cref="RtssOverlay.Sizes"/>.</summary>
    public string Size { get; init; } = RtssOverlay.Small;

    /// <summary>Desenha o contador num cartão translúcido arredondado em vez de direto sobre o jogo.</summary>
    public bool Card { get; init; } = true;

    /// <summary>Um de <see cref="RtssOverlay.Positions"/>: onde o RTSS põe o OSD, ou um canto da tela.</summary>
    public string Position { get; init; } = RtssOverlay.PositionRtss;
}

/// <summary>
/// Contador de FPS do Tako sobre o jogo, desenhado pelo RTSS num espaço de OSD próprio (<see cref="Owner"/>).
/// Todo estilo, menos <see cref="External"/>, toma a tela: o OSD do MSI Afterburner fica escondido enquanto ele está
/// ligado, então "Desligado" é mesmo nada na tela. External deixa Afterburner/RTSS como estão.
/// FPS e tempo de quadro vêm do próprio RTSS; CPU, GPU e memória de um <see cref="HardwareSample"/>, então os estilos
/// que os mostram são reescritos a cada segundo (<see cref="NeedsHardware"/>).
/// </summary>
public static partial class RtssOverlay
{
    public const string External = "external";
    public const string Off = "off";

    /// <summary>Só o FPS. O id é "compact", o nome que este estilo tinha no Console Mode até a 1.6.0.</summary>
    public const string FpsOnly = "compact";

    /// <summary>FPS e uso de CPU, GPU e RAM numa linha.</summary>
    public const string Compact = "stats";
    public const string Detailed = "detailed";
    public const string Custom = "custom";

    /// <summary>szOSDOwner do espaço do Tako.</summary>
    public const string Owner = "Tako";

    public const string Small = "small";
    public const string Medium = "medium";
    public const string Large = "large";

    public const string PositionRtss = "rtss";
    public const string TopLeft = "top-left";
    public const string TopRight = "top-right";
    public const string BottomLeft = "bottom-left";
    public const string BottomRight = "bottom-right";

    // Hipertexto do RTSS (amostra do SDK e OverlayEditor): <Cn=AARRGGBB> / <Sn=tamanho> / <An=largura> definem uma
    // variável, <Cn>…<C> / <Sn>…<S> / <An>…<A> a usam (<S> e <C> voltam a 100% / cor padrão). Tamanhos em porcentagem,
    // negativo = apoia na linha de base; larguras em caracteres, negativo = alinhado à direita, para os números não
    // pularem. <FR> FPS, <FT> tempo de quadro e <APP> API 3D são preenchidos pelo RTSS. O cartão é uma camada (<L>) com
    // margem (<M>) cujo fundo é uma barra arredondada (<B=0,0,Rn>, 0,0 = a camada toda) na cor C3; \b volta o cursor.
    private const string Value = "C0";
    private const string Label = "C1";
    private const string Accent = "C2";
    private const string CardFill = "C3";
    private const string ValueSize = "S0";
    private const string UnitSize = "S1";
    private const string LabelSize = "S2";
    private const string FpsSize = "S3";
    private const string LabelColor = "9AA4B2";
    private const string AccentColor = "4CC2FF";
    private const string CardColor = "C0101418";
    private const string Missing = "--";

    private const Item PcItems = Item.Cpu | Item.CpuClock | Item.Gpu | Item.GpuClock | Item.GpuTemp | Item.Vram | Item.Ram;

    [Flags]
    private enum Item
    {
        None = 0,
        Fps = 1,
        FrameTime = 2,
        Api = 4,
        Cpu = 8,
        CpuClock = 16,
        Gpu = 32,
        GpuClock = 64,
        GpuTemp = 128,
        Vram = 256,
        Ram = 512,
    }

    /// <summary>Ordem do menu; External primeiro, o padrão, para o Afterburner de ninguém sumir sem pedir.</summary>
    public static IReadOnlyList<string> Styles { get; } = [External, Off, FpsOnly, Compact, Detailed, Custom];

    /// <summary>Branco, azul, verde, amarelo, laranja.</summary>
    public static IReadOnlyList<string> Colors { get; } = ["FFFFFF", "4CC2FF", "6FE07A", "FFD54A", "FF8A3D"];

    public static IReadOnlyList<string> Sizes { get; } = [Small, Medium, Large];

    /// <summary>A posição do RTSS primeiro (o padrão): mantém o que o usuário definiu no RTSS.</summary>
    public static IReadOnlyList<string> Positions { get; } = [PositionRtss, TopLeft, TopRight, BottomLeft, BottomRight];

    private static string Gap => Run(LabelSize, Label, "  ");

    public static string Normalize(string? style) =>
        Styles.FirstOrDefault(s => string.Equals(s, style, StringComparison.OrdinalIgnoreCase)) ?? External;

    /// <summary>Se o estilo mantém o OSD do Afterburner fora da tela.</summary>
    public static bool HidesOthers(string? style) => Normalize(style) != External;

    /// <summary>Cor salva, de volta a branco se não for RRGGBB simples (ela vai dentro de uma tag do RTSS).</summary>
    public static string NormalizeColor(string? color) =>
        color is not null && HexColor().IsMatch(color) ? color.ToUpperInvariant() : Colors[0];

    public static string NormalizeSize(string? size) =>
        Sizes.FirstOrDefault(s => string.Equals(s, size, StringComparison.OrdinalIgnoreCase)) ?? Small;

    public static string NormalizePosition(string? position) =>
        Positions.FirstOrDefault(p => string.Equals(p, position, StringComparison.OrdinalIgnoreCase)) ?? PositionRtss;

    /// <summary>Se o estilo mostra CPU/GPU/memória, que o RTSS não conhece: o texto precisa de leitura nova a cada segundo.</summary>
    public static bool NeedsHardware(string? style, FpsOverlayLayout? layout = null) =>
        (ItemsOf(Normalize(style), layout) & PcItems) != 0;

    /// <summary>
    /// Texto do OSD para um estilo; vazio para Off e External (nada do Tako na tela).
    /// <paramref name="sample"/> nulo = ainda não medido: valores do PC aparecem como "--". Texto em Latin-1 (°), o que
    /// o espaço do RTSS guarda.
    /// </summary>
    public static string Text(string? style, FpsOverlayLayout? layout = null, HardwareSample? sample = null)
    {
        style = Normalize(style);
        var items = ItemsOf(style, layout);
        if (items == Item.None)
        {
            return string.Empty;
        }

        layout ??= new FpsOverlayLayout();
        bool singleLine = style switch
        {
            FpsOnly or Compact => true,
            Custom => layout.SingleLine,
            _ => false,
        };

        double scale = NormalizeSize(layout.Size) switch { Medium => 1.5, Large => 2.0, _ => 1.0 };
        var rows = new List<string>();
        var game = new List<string>();
        if (items.HasFlag(Item.Fps))
        {
            game.Add(Run(FpsSize, Value, "<FR>") + Run(UnitSize, Label, " FPS"));
        }

        if (items.HasFlag(Item.FrameTime))
        {
            game.Add(Run(ValueSize, Value, "<FT>") + Run(UnitSize, Label, " ms"));
        }

        if (items.HasFlag(Item.Api))
        {
            game.Add(Run(LabelSize, Accent, "<APP>"));
        }

        if (game.Count > 0)
        {
            rows.Add(string.Join(Gap, game));
        }

        // Compacto é uma linha curta: RAM em porcentagem, como as outras duas.
        bool ramAsPercent = style == Compact;
        AddRow(
            rows,
            "CPU",
            singleLine,
            items.HasFlag(Item.Cpu) ? Number(sample, sample?.CpuUsage, "F0", 3, "%") : null,
            items.HasFlag(Item.CpuClock) ? Number(sample, sample?.CpuClockMhz / 1000, "F1", 3, " GHz") : null);
        AddRow(
            rows,
            "GPU",
            singleLine,
            items.HasFlag(Item.Gpu) ? Number(sample, sample?.GpuUsage, "F0", 3, "%") : null,
            items.HasFlag(Item.GpuClock) ? Number(sample, sample?.GpuClockMhz, "F0", 4, " MHz") : null,
            items.HasFlag(Item.GpuTemp) ? Number(sample, sample?.GpuTempC, "F0", 3, "°C") : null);
        AddRow(rows, "VRAM", singleLine, items.HasFlag(Item.Vram) ? Memory(sample, sample?.VramUsedMb, sample?.VramTotalMb, false) : null);
        AddRow(rows, "RAM", singleLine, items.HasFlag(Item.Ram) ? Memory(sample, sample?.RamUsedMb, sample?.RamTotalMb, ramAsPercent) : null);
        if (rows.Count == 0)
        {
            rows.Add(Run(FpsSize, Value, "<FR>") + Run(UnitSize, Label, " FPS"));
        }

        var header = new StringBuilder()
            .Append(CultureInfo.InvariantCulture, $"<{Value}={NormalizeColor(layout.Color)}><{Label}={LabelColor}><{Accent}={AccentColor}><{CardFill}={CardColor}>")
            .Append(CultureInfo.InvariantCulture, $"<{ValueSize}={Percent(100, scale)}><{UnitSize}=-{Percent(55, scale)}><{LabelSize}=-{Percent(70, scale)}>")
            .Append(CultureInfo.InvariantCulture, $"<{FpsSize}={Percent(singleLine ? 100 : 130, scale)}>")
            .Append("<A0=-3><A1=-4><A2=5>");

        // Um canto é uma posição fixa (<P0> superior esquerdo, <P2> superior direito, <P6> inferior esquerdo, <P8>
        // inferior direito da grade 3x3 do RTSS), que ignora a posição do OSD definida no RTSS; precisa de uma camada.
        string corner = NormalizePosition(layout.Position) switch
        {
            TopLeft => "<P0>",
            TopRight => "<P2>",
            BottomLeft => "<P6>",
            BottomRight => "<P8>",
            _ => string.Empty,
        };
        header.Append(corner);
        if (layout.Card)
        {
            int pad = (int)Math.Round(6 * scale);
            header.Append(CultureInfo.InvariantCulture, $"<M={pad + 2},{pad},{pad + 2},{pad}><L0><{CardFill}><B=0,0,R{pad + 2}>\b<C>");
        }
        else if (corner.Length > 0)
        {
            header.Append("<L0>");
        }

        return header + string.Join(singleLine ? Gap + Gap : "\n", rows);
    }

    private static Item ItemsOf(string style, FpsOverlayLayout? layout)
    {
        switch (style)
        {
            case FpsOnly:
                return Item.Fps;
            case Compact:
                return Item.Fps | Item.Cpu | Item.Gpu | Item.Ram;
            case Detailed:
                return Item.Fps | Item.FrameTime | Item.Api | PcItems;
            case Custom:
                layout ??= new FpsOverlayLayout();
                var items = Item.None;
                items |= layout.ShowFps ? Item.Fps : Item.None;
                items |= layout.ShowFrameTime ? Item.FrameTime : Item.None;
                items |= layout.ShowApi ? Item.Api : Item.None;
                items |= layout.ShowCpu ? Item.Cpu : Item.None;
                items |= layout.ShowCpuClock ? Item.CpuClock : Item.None;
                items |= layout.ShowGpu ? Item.Gpu : Item.None;
                items |= layout.ShowGpuClock ? Item.GpuClock : Item.None;
                items |= layout.ShowGpuTemp ? Item.GpuTemp : Item.None;
                items |= layout.ShowVram ? Item.Vram : Item.None;
                items |= layout.ShowRam ? Item.Ram : Item.None;
                return items == Item.None ? Item.Fps : items; // nada marcado ainda mostra alguma coisa
            default:
                return Item.None;
        }
    }

    private static string Run(string size, string color, string text) => $"<{size}><{color}>{text}<C><S>";

    private static string Percent(int percent, double scale) =>
        ((int)Math.Round(percent * scale)).ToString(CultureInfo.InvariantCulture);

    /// <summary>"CPU 45% 4.2 GHz": a coluna do rótulo tem largura fixa quando empilhada, para os valores alinharem.</summary>
    private static void AddRow(List<string> rows, string label, bool singleLine, params string?[] values)
    {
        var shown = values.Where(v => !string.IsNullOrEmpty(v)).ToList();
        if (shown.Count == 0)
        {
            return;
        }

        string name = singleLine ? Run(LabelSize, Label, label + " ") : $"<{LabelSize}><{Label}><A2>{label}<A><C><S>";
        rows.Add(name + string.Join(Gap, shown));
    }

    /// <summary>Valor alinhado à direita numa caixa de <paramref name="width"/> caracteres e a unidade. "" = indisponível aqui.</summary>
    private static string Number(HardwareSample? sample, double? value, string format, int width, string unit)
    {
        string text;
        if (sample is null)
        {
            text = Missing;
        }
        else if (value is { } v && double.IsFinite(v))
        {
            text = v.ToString(format, CultureInfo.InvariantCulture);
        }
        else
        {
            return string.Empty;
        }

        string box = width >= 4 ? "A1" : "A0";
        return $"<{ValueSize}><{Value}><{box}>{text}<A><C><S>" + Run(UnitSize, Label, unit);
    }

    /// <summary>"5.2 / 8 GB", ou porcentagem na linha do Compacto.</summary>
    private static string Memory(HardwareSample? sample, double? usedMb, double? totalMb, bool asPercent)
    {
        if (asPercent)
        {
            return Number(sample, usedMb is { } u && totalMb is > 0 ? u / totalMb * 100 : null, "F0", 3, "%");
        }

        if (sample is null)
        {
            return Number(null, null, "F1", 3, " GB");
        }

        if (usedMb is not { } used)
        {
            return string.Empty;
        }

        string text = Number(sample, used / 1024, "F1", 3, string.Empty);
        string total = totalMb is > 0 ? " / " + (totalMb.Value / 1024).ToString("F0", CultureInfo.InvariantCulture) : string.Empty;
        return text + Run(UnitSize, Label, total + " GB");
    }

    [GeneratedRegex("^[0-9A-Fa-f]{6}$")]
    private static partial Regex HexColor();
}
