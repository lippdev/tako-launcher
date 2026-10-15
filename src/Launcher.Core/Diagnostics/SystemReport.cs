using System.Globalization;
using System.Text;

namespace Launcher.Core.Diagnostics;

/// <summary>Monitor conectado à área de trabalho, em pixels físicos.</summary>
/// <param name="Device">Nome do Windows (<c>\\.\DISPLAY1</c>).</param>
/// <param name="Adapter">Placa de vídeo que alimenta o monitor, quando o Windows informa.</param>
/// <param name="ScalePercent">Escala do Windows (100, 125, 150...); nulo se não foi possível ler.</param>
/// <param name="RefreshHz">Taxa de atualização; nulo se não foi possível ler.</param>
public sealed record DisplayInfo(string Device, string? Adapter, int Width, int Height, int? ScalePercent, int? RefreshHz, bool IsPrimary)
{
    /// <summary>Proporção aproximada (16:9, 21:9...), útil para casar com o checklist de hardware.</summary>
    public string AspectRatio
    {
        get
        {
            if (Width <= 0 || Height <= 0)
            {
                return "?";
            }

            double ratio = (double)Width / Height;
            (string Label, double Value)[] known = [("4:3", 4d / 3), ("16:10", 16d / 10), ("16:9", 16d / 9), ("21:9", 21d / 9), ("32:9", 32d / 9), ("3:2", 3d / 2)];
            var best = known.MinBy(k => Math.Abs(k.Value - ratio));
            return Math.Abs(best.Value - ratio) < 0.07 ? best.Label : ratio.ToString("0.00", CultureInfo.InvariantCulture) + ":1";
        }
    }
}

/// <summary>
/// Fatos do sistema para o diagnóstico assistido (MVP-034). Só o que ajuda a reproduzir um problema de hardware:
/// nada de nome de usuário, nome do computador, contas ou lista de jogos.
/// </summary>
public sealed record SystemSnapshot
{
    public required DateTimeOffset GeneratedAt { get; init; }

    public required string TakoVersion { get; init; }

    /// <summary>"instalada", "portátil" ou "desenvolvimento".</summary>
    public required string InstallKind { get; init; }

    /// <summary>Pasta de dados (o relatório troca a pasta do perfil por <c>%USERPROFILE%</c>).</summary>
    public string? DataFolder { get; init; }

    public string? WindowsName { get; init; }

    /// <summary>Versão de marketing (24H2, 25H2...).</summary>
    public string? WindowsDisplayVersion { get; init; }

    /// <summary>Build com revisão (26100.4351).</summary>
    public string? WindowsBuild { get; init; }

    public required string Architecture { get; init; }

    public required string Runtime { get; init; }

    public string? Cpu { get; init; }

    public int LogicalProcessors { get; init; }

    public long? MemoryBytes { get; init; }

    public IReadOnlyList<DisplayInfo> Displays { get; init; } = [];

    /// <summary>Controles detectados agora (XInput e PlayStation por HID).</summary>
    public IReadOnlyList<string> Controllers { get; init; } = [];

    /// <summary>Controle em uso pela navegação do Tako, se houver.</summary>
    public string? ActiveController { get; init; }

    public bool SteamFound { get; init; }

    public int SteamGames { get; init; }

    public int ManualGames { get; init; }

    public int TotalGames { get; init; }

    /// <summary>Estado do Xbox mode / FSE (MVP-008): rótulo e motivo.</summary>
    public string? XboxModeLabel { get; init; }

    public string? XboxModeReason { get; init; }

    /// <summary>Sinais brutos lidos do Windows (build, app Xbox, DeviceForm), para conferir falsos negativos.</summary>
    public string? XboxModeSignals { get; init; }

    public bool SafeMode { get; init; }

    public string? SafeModeDetail { get; init; }

    public string? Theme { get; init; }
}

/// <summary>
/// Texto do "Diagnóstico do sistema" (MVP-034): o usuário copia ou anexa o arquivo ao rodar o checklist de hardware.
/// O texto inteiro passa pelo <see cref="LogRedactor"/>; nome de usuário e do computador viram marcadores.
/// </summary>
public static class SystemReport
{
    public const string UserMarker = "[usuário]";
    public const string MachineMarker = "[computador]";

    /// <summary>Nome do arquivo salvo na pasta de logs: <c>diagnostico-AAAAMMDD-HHMMSS.txt</c>.</summary>
    public static string FileName(DateTimeOffset time) =>
        "diagnostico-" + time.ToString("yyyyMMdd-HHmmss", CultureInfo.InvariantCulture) + ".txt";

    public static string Format(SystemSnapshot snapshot, LogRedactor redactor, string? userName = null, string? machineName = null)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        ArgumentNullException.ThrowIfNull(redactor);
        var c = CultureInfo.InvariantCulture;
        var text = new StringBuilder();
        void Line(string label, string? value) => text.Append(c, $"{label + ":",-22} {(string.IsNullOrWhiteSpace(value) ? "não informado" : value)}\n");

        text.Append("Tako · Diagnóstico do sistema\n");
        text.Append(c, $"Gerado em {snapshot.GeneratedAt.ToString("yyyy-MM-dd HH:mm:ss zzz", c)}\n\n");

        text.Append("[Tako]\n");
        Line("Versão", snapshot.TakoVersion);
        Line("Instalação", snapshot.InstallKind);
        Line("Pasta de dados", snapshot.DataFolder);
        Line("Tema", snapshot.Theme);
        Line("Modo seguro", snapshot.SafeMode ? "ativo" + (snapshot.SafeModeDetail is { Length: > 0 } d ? $" ({d})" : string.Empty) : "desligado");
        text.Append('\n');

        text.Append("[Windows]\n");
        string windows = string.Join(' ', new[] { snapshot.WindowsName, snapshot.WindowsDisplayVersion }.Where(s => !string.IsNullOrWhiteSpace(s)));
        Line("Sistema", windows);
        Line("Build", snapshot.WindowsBuild);
        Line("Arquitetura", snapshot.Architecture);
        Line(".NET", snapshot.Runtime);
        Line("Processador", snapshot.Cpu is null ? null : $"{snapshot.Cpu} ({snapshot.LogicalProcessors} threads)");
        Line("Memória", snapshot.MemoryBytes is long bytes ? (bytes / (1024d * 1024 * 1024)).ToString("0.0", c) + " GB" : null);
        text.Append('\n');

        text.Append(c, $"[Monitores] {snapshot.Displays.Count}\n");
        if (snapshot.Displays.Count == 0)
        {
            text.Append("  nenhum monitor lido\n");
        }

        foreach (var display in snapshot.Displays)
        {
            string scale = display.ScalePercent is int s ? $"escala {s}%" : "escala ?";
            string hz = display.RefreshHz is int r ? $"{r} Hz" : "? Hz";
            text.Append(c, $"  {display.Device}{(display.IsPrimary ? " (principal)" : string.Empty)}: {display.Width}x{display.Height} {display.AspectRatio}, {scale}, {hz}");
            text.Append(display.Adapter is { Length: > 0 } adapter ? $" · {adapter}\n" : "\n");
        }

        text.Append('\n');
        text.Append(c, $"[Controles] {snapshot.Controllers.Count}\n");
        if (snapshot.Controllers.Count == 0)
        {
            text.Append("  nenhum controle detectado (XInput e PlayStation por HID)\n");
        }

        foreach (var controller in snapshot.Controllers)
        {
            text.Append(c, $"  {controller}\n");
        }

        Line("  Em uso no Tako", snapshot.ActiveController ?? "teclado");
        text.Append('\n');

        text.Append("[Biblioteca]\n");
        Line("Steam", snapshot.SteamFound ? $"encontrada, {snapshot.SteamGames} jogos" : "não encontrada");
        Line("Adicionados", snapshot.ManualGames.ToString(c));
        Line("Total", snapshot.TotalGames.ToString(c));
        text.Append('\n');

        text.Append("[Xbox mode / FSE]\n");
        Line("Estado", snapshot.XboxModeLabel);
        Line("Motivo", snapshot.XboxModeReason);
        Line("Sinais", snapshot.XboxModeSignals);

        string report = redactor.Redact(text.ToString());
        // Computador antes do usuário: nomes como "PC-DO-FULANO" contêm o nome do usuário.
        report = ReplaceWord(report, machineName, MachineMarker);
        return ReplaceWord(report, userName, UserMarker);
    }

    /// <summary>Troca nomes curtos só como palavra inteira ("Ana" não vira marcador dentro de "Banana").</summary>
    private static string ReplaceWord(string text, string? word, string marker)
    {
        if (string.IsNullOrWhiteSpace(word) || word.Length < 2)
        {
            return text;
        }

        var result = new StringBuilder(text.Length);
        int start = 0;
        while (true)
        {
            int index = text.IndexOf(word, start, StringComparison.OrdinalIgnoreCase);
            if (index < 0)
            {
                break;
            }

            bool boundaryBefore = index == 0 || !char.IsLetterOrDigit(text[index - 1]);
            int end = index + word.Length;
            bool boundaryAfter = end == text.Length || !char.IsLetterOrDigit(text[end]);
            result.Append(text, start, index - start).Append(boundaryBefore && boundaryAfter ? marker : text.Substring(index, word.Length));
            start = end;
        }

        return result.Append(text, start, text.Length - start).ToString();
    }
}
