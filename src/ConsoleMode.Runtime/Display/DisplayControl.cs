using System.Globalization;

namespace ConsoleMode.Runtime.Display;

/// <summary>Retângulo de um monitor na área de trabalho, em pixels físicos.</summary>
public readonly record struct ScreenBounds(int X, int Y, int Width, int Height)
{
    public override string ToString() => string.Create(CultureInfo.InvariantCulture, $"{Width}×{Height} em ({X},{Y})");
}

/// <summary>HDR (cor avançada) de um monitor.</summary>
public enum HdrStatus
{
    /// <summary>Não foi possível ler (monitor desligado, driver sem suporte à consulta).</summary>
    Unknown,
    Unsupported,
    Off,
    On,
}

/// <summary>Resolução e taxa de atualização em Hz inteiros (como o Windows lista os modos).</summary>
public sealed record DisplayMode(int Width, int Height, int RefreshHz)
{
    /// <summary>Formato salvo nas configurações: <c>3840x2160@120</c>.</summary>
    public string Key => string.Create(CultureInfo.InvariantCulture, $"{Width}x{Height}@{RefreshHz}");

    public bool IsValid => Width > 0 && Height > 0 && RefreshHz > 0;

    public static DisplayMode? TryParse(string? key)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            return null;
        }

        string[] sizeAndRate = key.Trim().Split('@');
        string[] size = sizeAndRate[0].Split('x', 'X', '×');
        if (sizeAndRate.Length != 2 || size.Length != 2
            || !int.TryParse(size[0], NumberStyles.None, CultureInfo.InvariantCulture, out int width)
            || !int.TryParse(size[1], NumberStyles.None, CultureInfo.InvariantCulture, out int height)
            || !int.TryParse(sizeAndRate[1], NumberStyles.None, CultureInfo.InvariantCulture, out int hz))
        {
            return null;
        }

        var mode = new DisplayMode(width, height, hz);
        return mode.IsValid ? mode : null;
    }

    /// <summary>Lista para escolher: sem repetidos nem modos inválidos, do maior para o menor (resolução, depois taxa).</summary>
    public static IReadOnlyList<DisplayMode> Normalize(IEnumerable<DisplayMode> modes)
    {
        ArgumentNullException.ThrowIfNull(modes);
        return [.. modes.Where(m => m.IsValid).Distinct().OrderByDescending(m => m.Width).ThenByDescending(m => m.Height).ThenByDescending(m => m.RefreshHz)];
    }

    public override string ToString() => string.Create(CultureInfo.InvariantCulture, $"{Width} × {Height} · {RefreshHz} Hz");
}

/// <summary>
/// Monitor conectado, ativo ou não. <see cref="Id"/> é o mesmo de <see cref="MonitorState.Id"/> (no Windows, o caminho
/// do dispositivo do monitor), estável entre sessões. Inativo: sem <see cref="Bounds"/> nem <see cref="CurrentMode"/>.
/// </summary>
public sealed record ConnectedDisplay(
    string Id,
    string Name,
    bool IsActive,
    bool IsPrimary,
    ScreenBounds? Bounds,
    DisplayMode? CurrentMode,
    HdrStatus Hdr)
{
    public override string ToString() => $"{Name}{(IsActive ? string.Empty : " (desligada)")}{(IsPrimary ? " principal" : string.Empty)}";
}

/// <summary>
/// Comandos sobre monitores para a "Tela de jogo": listar os conectados (inclusive os desligados), religar e desconectar
/// da área de trabalho, trocar o principal, a resolução e o HDR. Falha da plataforma:
/// <see cref="RuntimeEnvironmentException"/>. O estado anterior volta pelo <see cref="IDisplayConfigurationService"/>.
/// </summary>
public interface IDisplayControlService
{
    IReadOnlyList<ConnectedDisplay> ListDisplays();

    /// <summary>Modos que o monitor aceita (vazio se estiver desligado ou não der para ler).</summary>
    IReadOnlyList<DisplayMode> ListModes(string displayId);

    /// <summary>Liga na área de trabalho (estendida) os monitores conectados e desligados.</summary>
    void Enable(IReadOnlyCollection<string> displayIds);

    /// <summary>Desconecta da área de trabalho. Recusa desligar todos os monitores ativos.</summary>
    void Disable(IReadOnlyCollection<string> displayIds);

    void SetPrimary(string displayId);

    void SetMode(string displayId, DisplayMode mode);

    void SetHdr(string displayId, bool enabled);
}

/// <summary>Janelas pretas por cima de monitores (alternativa a desconectar). Fechar sem nada aberto não faz nada.</summary>
public interface IScreenCurtain
{
    void Show(IReadOnlyList<ScreenBounds> screens);

    void Close();
}
