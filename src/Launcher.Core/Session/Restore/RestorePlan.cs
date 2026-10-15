using System.Globalization;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;

namespace Launcher.Core.Session.Restore;

/// <summary>Uma alteração a fazer no Windows para voltar ao estado registrado.</summary>
public abstract record RestoreStep
{
    public abstract string Description { get; }

    public override string ToString() => Description;
}

/// <summary>Volta a configuração de monitores inteira (topologia, principal, resolução, taxa, rotação, posição).</summary>
public sealed record RestoreDisplayStep(DisplayTopology Target, IReadOnlyList<string> ChangedMonitorIds) : RestoreStep
{
    public override string Description => $"monitores ({ChangedMonitorIds.Count} alterado(s))";
}

public sealed record SetDefaultAudioStep(AudioRole Role, string DeviceId, string DeviceName) : RestoreStep
{
    public override string Description => $"saída padrão {Role}: {DeviceName}";
}

public sealed record SetVolumeStep(string DeviceId, string DeviceName, float Volume) : RestoreStep
{
    public override string Description => string.Create(CultureInfo.InvariantCulture, $"volume de {DeviceName}: {Volume:P0}");
}

public sealed record SetMuteStep(string DeviceId, string DeviceName, bool Muted) : RestoreStep
{
    public override string Description => $"{DeviceName}: {(Muted ? "mudo" : "com som")}";
}

/// <summary>Diferença que não será corrigida, com o motivo (vai para o log).</summary>
public sealed record SkippedRestore(string What, string Reason)
{
    public override string ToString() => $"{What}: {Reason}";
}

/// <summary>Plano de restauração: passos na ordem de execução e o que fica como está.</summary>
public sealed record RestorePlan(IReadOnlyList<RestoreStep> Steps, IReadOnlyList<SkippedRestore> Skipped)
{
    public static RestorePlan Empty { get; } = new([], []);

    public bool IsEmpty => Steps.Count == 0;
}

/// <summary>
/// Decide o que restaurar (sem E/S): só o que mudou, só o que ainda existe e só o que a plataforma consegue fazer.
/// Monitores primeiro (trocar de monitor pode trocar a saída HDMI), depois dispositivo padrão, depois volume/mudo.
/// </summary>
public static class RestorePlanner
{
    public static RestorePlan Plan(SessionEnvironmentSnapshot expected, SessionEnvironmentSnapshot actual, bool canSetDefaultDevice)
    {
        ArgumentNullException.ThrowIfNull(expected);
        ArgumentNullException.ThrowIfNull(actual);
        return Plan(EnvironmentDiff.Between(expected, actual), expected, actual, canSetDefaultDevice);
    }

    public static RestorePlan Plan(EnvironmentDiff diff, SessionEnvironmentSnapshot expected, SessionEnvironmentSnapshot actual, bool canSetDefaultDevice)
    {
        ArgumentNullException.ThrowIfNull(diff);
        ArgumentNullException.ThrowIfNull(expected);
        ArgumentNullException.ThrowIfNull(actual);
        var steps = new List<RestoreStep>();
        var skipped = new List<SkippedRestore>();

        if (diff.HasDisplayChanges && expected.Display is { } display)
        {
            if (display.Monitors.Count == 0 || display.Primary is null)
            {
                skipped.Add(new SkippedRestore("monitores", "o estado registrado não tem monitor principal"));
            }
            else
            {
                steps.Add(new RestoreDisplayStep(display, [.. diff.Monitors.Select(m => m.MonitorId).Distinct(StringComparer.OrdinalIgnoreCase)]));
            }
        }

        var audioNow = actual.Audio ?? AudioState.Empty;
        foreach (var change in diff.AudioDefaults)
        {
            string name = NameOf(expected.Audio, change.ExpectedDeviceId);
            if (audioNow.FindDevice(change.ExpectedDeviceId) is null)
            {
                skipped.Add(new SkippedRestore($"saída padrão {change.Role}", $"{name} não está conectado"));
            }
            else if (!canSetDefaultDevice)
            {
                skipped.Add(new SkippedRestore($"saída padrão {change.Role}", "trocar o dispositivo padrão não está disponível neste Windows"));
            }
            else
            {
                steps.Add(new SetDefaultAudioStep(change.Role, change.ExpectedDeviceId, name));
            }
        }

        foreach (var change in diff.AudioLevels)
        {
            string name = NameOf(expected.Audio, change.DeviceId);
            if (change.Actual is null || audioNow.FindDevice(change.DeviceId) is null)
            {
                skipped.Add(new SkippedRestore($"volume de {name}", "dispositivo não está conectado"));
                continue;
            }

            var volume = change.VolumeChanged ? new SetVolumeStep(change.DeviceId, name, change.Expected.Volume) : null;
            var mute = change.MuteChanged ? new SetMuteStep(change.DeviceId, name, change.Expected.Muted) : null;
            // Sem pico de som: para voltar ao mudo, silencia antes de mexer no volume; para tirar do mudo, ajusta antes.
            RestoreStep?[] ordered = change.Expected.Muted ? [mute, volume] : [volume, mute];
            steps.AddRange(ordered.OfType<RestoreStep>());
        }

        return new RestorePlan(steps, skipped);
    }

    private static string NameOf(AudioState? state, string deviceId) => state?.FindDevice(deviceId)?.Name ?? deviceId;
}
