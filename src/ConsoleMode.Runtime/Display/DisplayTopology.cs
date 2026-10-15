namespace ConsoleMode.Runtime.Display;

/// <summary>Rotação da imagem no monitor, em graus no sentido horário.</summary>
public enum DisplayRotation
{
    None = 0,
    Rotate90 = 90,
    Rotate180 = 180,
    Rotate270 = 270,
}

/// <summary>
/// Taxa de atualização como fração (o Windows guarda 59,94 Hz como 60000/1001). A comparação usa
/// <see cref="IsSameRate"/>, que tolera frações equivalentes e arredondamentos de driver.
/// </summary>
public readonly record struct RefreshRate(uint Numerator, uint Denominator)
{
    /// <summary>Tolerância para considerar duas taxas iguais (ex.: 59,94 e 59,95).</summary>
    public const double ToleranceHz = 0.05;

    public double Hz => Denominator == 0 ? 0 : (double)Numerator / Denominator;

    public static RefreshRate FromHz(uint hz) => new(hz, 1);

    public bool IsSameRate(RefreshRate other) => Math.Abs(Hz - other.Hz) <= ToleranceHz;

    public override string ToString() => $"{Hz:0.##} Hz";
}

/// <summary>
/// Estado de um monitor ativo. <see cref="Id"/> identifica o monitor físico de forma estável entre sessões (no
/// Windows, o caminho do dispositivo do monitor); <see cref="Name"/> é só para logs e telas.
/// </summary>
public sealed record MonitorState(
    string Id,
    string Name,
    bool IsPrimary,
    int X,
    int Y,
    int Width,
    int Height,
    RefreshRate RefreshRate,
    DisplayRotation Rotation)
{
    public bool HasSameMode(MonitorState other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return Width == other.Width && Height == other.Height && RefreshRate.IsSameRate(other.RefreshRate);
    }

    public bool HasSamePosition(MonitorState other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return X == other.X && Y == other.Y;
    }

    /// <summary>Mesmo estado visível (ignora o nome e diferenças de arredondamento da taxa).</summary>
    public bool IsEquivalentTo(MonitorState other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return string.Equals(Id, other.Id, StringComparison.OrdinalIgnoreCase)
            && IsPrimary == other.IsPrimary
            && HasSamePosition(other)
            && HasSameMode(other)
            && Rotation == other.Rotation;
    }

    public override string ToString() => $"{Name} {Width}×{Height} @ {RefreshRate} ({X},{Y}){(IsPrimary ? " principal" : string.Empty)}";
}

/// <summary>
/// Topologia de monitores: os monitores ativos com modo, posição e rotação. <see cref="PlatformData"/> é opcional e
/// opaco: a implementação da plataforma pode guardar ali o que precisa para uma restauração exata (no Windows, a
/// configuração do <c>QueryDisplayConfig</c>); quem não a conhece restaura pelos campos descritivos.
/// </summary>
public sealed record DisplayTopology(IReadOnlyList<MonitorState> Monitors, string? PlatformData = null)
{
    public static DisplayTopology Empty { get; } = new([]);

    public MonitorState? Primary => Monitors.FirstOrDefault(m => m.IsPrimary);

    public MonitorState? Find(string id) => Monitors.FirstOrDefault(m => string.Equals(m.Id, id, StringComparison.OrdinalIgnoreCase));

    /// <summary>Mesmos monitores ativos, cada um no mesmo estado (a ordem e <see cref="PlatformData"/> não contam).</summary>
    public bool IsEquivalentTo(DisplayTopology other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return Monitors.Count == other.Monitors.Count
            && Monitors.All(m => other.Find(m.Id) is { } match && m.IsEquivalentTo(match));
    }
}

/// <summary>Leitura e aplicação da configuração de monitores (implementação por plataforma).</summary>
public interface IDisplayConfigurationService
{
    /// <summary>Configuração atual. Falha da plataforma: <see cref="RuntimeEnvironmentException"/>.</summary>
    DisplayTopology Capture();

    /// <summary>
    /// Aplica <paramref name="target"/>. <paramref name="changedMonitorIds"/> lista os monitores que mudaram, para
    /// quem só consegue restaurar monitor a monitor. Falha da plataforma: <see cref="RuntimeEnvironmentException"/>.
    /// </summary>
    void Apply(DisplayTopology target, IReadOnlyCollection<string> changedMonitorIds);
}
