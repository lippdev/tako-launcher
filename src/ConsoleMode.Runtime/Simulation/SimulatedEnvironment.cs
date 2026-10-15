using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;

namespace ConsoleMode.Runtime.Simulation;

/// <summary>
/// Monitores em memória, sem tocar no sistema: testes, teste de fumaça da UI e CI. Registra cada aplicação em
/// <see cref="AppliedCount"/> e pode simular falha da plataforma.
/// </summary>
public sealed class SimulatedDisplayConfiguration : IDisplayConfigurationService
{
    private readonly object _gate = new();

    public SimulatedDisplayConfiguration(DisplayTopology? initial = null) => Current = initial ?? Default;

    /// <summary>Um monitor 1920×1080 a 60 Hz.</summary>
    public static DisplayTopology Default { get; } = new(
    [
        new MonitorState("SIM\\MONITOR1", "Monitor simulado", IsPrimary: true, 0, 0, 1920, 1080, RefreshRate.FromHz(60), DisplayRotation.None),
    ]);

    public DisplayTopology Current { get; set; }

    public int AppliedCount { get; private set; }

    public IReadOnlyCollection<string> LastChangedMonitorIds { get; private set; } = [];

    /// <summary>Próximas chamadas falham como falharia a plataforma.</summary>
    public bool FailNext { get; set; }

    public DisplayTopology Capture()
    {
        lock (_gate)
        {
            ThrowIfFailing();
            return Current;
        }
    }

    public void Apply(DisplayTopology target, IReadOnlyCollection<string> changedMonitorIds)
    {
        ArgumentNullException.ThrowIfNull(target);
        lock (_gate)
        {
            ThrowIfFailing();
            Current = target with { PlatformData = null };
            LastChangedMonitorIds = changedMonitorIds;
            AppliedCount++;
        }
    }

    private void ThrowIfFailing()
    {
        if (FailNext)
        {
            throw new RuntimeEnvironmentException("Falha simulada de monitores");
        }
    }
}

/// <summary>Áudio em memória, sem tocar no sistema: testes, teste de fumaça da UI e CI.</summary>
public sealed class SimulatedAudioEndpoints : IAudioEndpointService
{
    private readonly object _gate = new();

    public SimulatedAudioEndpoints(AudioState? initial = null) => Current = initial ?? Default;

    /// <summary>Alto-falantes (70%) como padrão de todos os papéis e um fone (50%) disponível.</summary>
    public static AudioState Default { get; } = new(
        [new AudioDevice("sim-speakers", "Alto-falantes (simulado)"), new AudioDevice("sim-headset", "Fone (simulado)")],
        [
            new AudioDefaultDevice(AudioRole.Console, "sim-speakers"),
            new AudioDefaultDevice(AudioRole.Multimedia, "sim-speakers"),
            new AudioDefaultDevice(AudioRole.Communications, "sim-speakers"),
        ],
        [new AudioEndpointLevel("sim-speakers", 0.7f, Muted: false), new AudioEndpointLevel("sim-headset", 0.5f, Muted: false)]);

    public AudioState Current { get; set; }

    public bool CanSetDefaultDevice { get; set; } = true;

    /// <summary>Próximas chamadas falham como falharia a plataforma.</summary>
    public bool FailNext { get; set; }

    /// <summary>Alterações feitas, na ordem (ex.: "default Console sim-speakers", "volume sim-speakers 0.7").</summary>
    public IList<string> Calls { get; } = [];

    public AudioState Capture()
    {
        lock (_gate)
        {
            ThrowIfFailing();
            return Current;
        }
    }

    public void SetDefaultDevice(string deviceId, AudioRole role)
    {
        lock (_gate)
        {
            ThrowIfFailing();
            if (!CanSetDefaultDevice || Current.FindDevice(deviceId) is null)
            {
                throw new RuntimeEnvironmentException($"Dispositivo indisponível: {deviceId}");
            }

            Current = Current with { Defaults = [.. Current.Defaults.Where(d => d.Role != role), new AudioDefaultDevice(role, deviceId)] };
            Calls.Add($"default {role} {deviceId}");
        }
    }

    public void SetVolume(string deviceId, float volume)
    {
        lock (_gate)
        {
            ThrowIfFailing();
            var level = LevelFor(deviceId);
            Current = Current with { Levels = [.. Current.Levels.Where(l => l != level), level with { Volume = volume }] };
            Calls.Add(FormattableString.Invariant($"volume {deviceId} {volume:0.##}"));
        }
    }

    public void SetMute(string deviceId, bool muted)
    {
        lock (_gate)
        {
            ThrowIfFailing();
            var level = LevelFor(deviceId);
            Current = Current with { Levels = [.. Current.Levels.Where(l => l != level), level with { Muted = muted }] };
            Calls.Add($"mute {deviceId} {muted}");
        }
    }

    private AudioEndpointLevel LevelFor(string deviceId)
    {
        if (Current.FindDevice(deviceId) is null)
        {
            throw new RuntimeEnvironmentException($"Dispositivo indisponível: {deviceId}");
        }

        return Current.LevelOf(deviceId) ?? new AudioEndpointLevel(deviceId, 1f, Muted: false);
    }

    private void ThrowIfFailing()
    {
        if (FailNext)
        {
            throw new RuntimeEnvironmentException("Falha simulada de áudio");
        }
    }
}
