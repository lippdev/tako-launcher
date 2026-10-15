using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;

namespace Launcher.Core.Session.Restore;

/// <summary>O que mudou num monitor.</summary>
public enum MonitorChangeKind
{
    /// <summary>Estava ativo antes e não está mais (desligado pelo jogo ou desconectado).</summary>
    Missing,

    /// <summary>Não estava ativo antes.</summary>
    Added,

    /// <summary>Resolução ou taxa de atualização.</summary>
    Mode,

    Position,
    Rotation,
    Primary,
}

public sealed record MonitorChange(string MonitorId, string Name, MonitorChangeKind Kind, string Expected, string Actual)
{
    public override string ToString() => $"{Name}: {Kind} ({Actual} → {Expected})";
}

public sealed record AudioDefaultChange(AudioRole Role, string ExpectedDeviceId, string? ActualDeviceId)
{
    public override string ToString() => $"padrão {Role}: {ActualDeviceId ?? "nenhum"} → {ExpectedDeviceId}";
}

/// <summary>Volume ou mudo de um dispositivo. <see cref="Actual"/> nulo: o dispositivo não está mais disponível.</summary>
public sealed record AudioLevelChange(AudioEndpointLevel Expected, AudioEndpointLevel? Actual)
{
    public string DeviceId => Expected.DeviceId;

    public bool VolumeChanged => Actual is null || !Expected.HasSameVolume(Actual);

    public bool MuteChanged => Actual is null || Expected.Muted != Actual.Muted;

    public override string ToString() => Actual is null
        ? $"{DeviceId}: indisponível"
        : FormattableString.Invariant($"{DeviceId}: volume {Actual.Volume:P0} → {Expected.Volume:P0}, mudo {Actual.Muted} → {Expected.Muted}");
}

/// <summary>
/// Diferenças entre o estado registrado antes da sessão (<c>expected</c>) e o atual. Partes que faltam em qualquer
/// um dos snapshots (leitura falhou) não geram diferença: sem saber o estado, nada é alterado.
/// </summary>
public sealed record EnvironmentDiff(
    IReadOnlyList<MonitorChange> Monitors,
    IReadOnlyList<AudioDefaultChange> AudioDefaults,
    IReadOnlyList<AudioLevelChange> AudioLevels)
{
    public static EnvironmentDiff None { get; } = new([], [], []);

    public bool IsEmpty => Monitors.Count == 0 && AudioDefaults.Count == 0 && AudioLevels.Count == 0;

    public bool HasDisplayChanges => Monitors.Count > 0;

    public bool HasAudioChanges => AudioDefaults.Count > 0 || AudioLevels.Count > 0;

    public static EnvironmentDiff Between(SessionEnvironmentSnapshot expected, SessionEnvironmentSnapshot actual)
    {
        ArgumentNullException.ThrowIfNull(expected);
        ArgumentNullException.ThrowIfNull(actual);
        var monitors = expected.Display is { } wantedDisplay && actual.Display is { } currentDisplay ? CompareDisplay(wantedDisplay, currentDisplay) : [];
        if (expected.Audio is not { } wantedAudio || actual.Audio is not { } currentAudio)
        {
            return new EnvironmentDiff(monitors, [], []);
        }

        return new EnvironmentDiff(monitors, CompareDefaults(wantedAudio, currentAudio), CompareLevels(wantedAudio, currentAudio));
    }

    /// <summary>Só as diferenças que <paramref name="plan"/> tentou corrigir (para verificar a restauração).</summary>
    public EnvironmentDiff RestrictTo(RestorePlan plan)
    {
        ArgumentNullException.ThrowIfNull(plan);
        bool display = plan.Steps.OfType<RestoreDisplayStep>().Any();
        var roles = plan.Steps.OfType<SetDefaultAudioStep>().Select(s => s.Role).ToHashSet();
        var devices = plan.Steps.OfType<SetVolumeStep>().Select(s => s.DeviceId)
            .Concat(plan.Steps.OfType<SetMuteStep>().Select(s => s.DeviceId))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        return new EnvironmentDiff(
            display ? Monitors : [],
            [.. AudioDefaults.Where(c => roles.Contains(c.Role))],
            [.. AudioLevels.Where(c => devices.Contains(c.DeviceId))]);
    }

    public override string ToString() => IsEmpty ? "sem diferenças" : string.Join("; ", Monitors.Cast<object>().Concat(AudioDefaults).Concat(AudioLevels));

    private static List<MonitorChange> CompareDisplay(DisplayTopology expected, DisplayTopology actual)
    {
        var changes = new List<MonitorChange>();
        foreach (var wanted in expected.Monitors)
        {
            if (actual.Find(wanted.Id) is not { } current)
            {
                changes.Add(new MonitorChange(wanted.Id, wanted.Name, MonitorChangeKind.Missing, "ativo", "inativo"));
                continue;
            }

            if (!wanted.HasSameMode(current))
            {
                changes.Add(new MonitorChange(wanted.Id, wanted.Name, MonitorChangeKind.Mode, Mode(wanted), Mode(current)));
            }

            if (!wanted.HasSamePosition(current))
            {
                changes.Add(new MonitorChange(wanted.Id, wanted.Name, MonitorChangeKind.Position, $"({wanted.X},{wanted.Y})", $"({current.X},{current.Y})"));
            }

            if (wanted.Rotation != current.Rotation)
            {
                changes.Add(new MonitorChange(wanted.Id, wanted.Name, MonitorChangeKind.Rotation, wanted.Rotation.ToString(), current.Rotation.ToString()));
            }

            if (wanted.IsPrimary != current.IsPrimary)
            {
                changes.Add(new MonitorChange(wanted.Id, wanted.Name, MonitorChangeKind.Primary, Primary(wanted), Primary(current)));
            }
        }

        changes.AddRange(actual.Monitors
            .Where(m => expected.Find(m.Id) is null)
            .Select(m => new MonitorChange(m.Id, m.Name, MonitorChangeKind.Added, "inativo", "ativo")));
        return changes;
    }

    private static List<AudioDefaultChange> CompareDefaults(AudioState expected, AudioState actual)
    {
        var changes = new List<AudioDefaultChange>();
        foreach (var role in Enum.GetValues<AudioRole>())
        {
            string? current = actual.DefaultFor(role);
            if (expected.DefaultFor(role) is { } wanted && !string.Equals(wanted, current, StringComparison.OrdinalIgnoreCase))
            {
                changes.Add(new AudioDefaultChange(role, wanted, current));
            }
        }

        return changes;
    }

    private static List<AudioLevelChange> CompareLevels(AudioState expected, AudioState actual) =>
        [.. expected.Levels
            .Select(wanted => new AudioLevelChange(wanted, actual.LevelOf(wanted.DeviceId)))
            .Where(c => c.VolumeChanged || c.MuteChanged)];

    private static string Mode(MonitorState m) => $"{m.Width}×{m.Height} @ {m.RefreshRate}";

    private static string Primary(MonitorState m) => m.IsPrimary ? "principal" : "secundário";
}
