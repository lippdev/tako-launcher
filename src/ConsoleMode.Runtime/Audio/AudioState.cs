namespace ConsoleMode.Runtime.Audio;

/// <summary>Papel do dispositivo padrão de saída (mesmos papéis do Windows: jogos/sistema, multimídia, comunicação).</summary>
public enum AudioRole
{
    Console = 0,
    Multimedia = 1,
    Communications = 2,
}

/// <summary>Dispositivo de saída ativo. <see cref="Id"/> é estável entre sessões; <see cref="Name"/> é só para exibição.</summary>
public sealed record AudioDevice(string Id, string Name);

/// <summary>Dispositivo padrão de um papel.</summary>
public sealed record AudioDefaultDevice(AudioRole Role, string DeviceId);

/// <summary>Volume (0 a 1) e mudo de um dispositivo.</summary>
public sealed record AudioEndpointLevel(string DeviceId, float Volume, bool Muted)
{
    /// <summary>Diferença de volume que conta como mudança (1%: a barra do Windows anda de 1 em 1).</summary>
    public const float VolumeTolerance = 0.005f;

    public bool HasSameVolume(AudioEndpointLevel other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return Math.Abs(Volume - other.Volume) <= VolumeTolerance;
    }
}

/// <summary>
/// Estado de áudio de saída: dispositivos ativos, o padrão de cada papel e o volume/mudo de cada dispositivo ativo.
/// </summary>
public sealed record AudioState(
    IReadOnlyList<AudioDevice> Devices,
    IReadOnlyList<AudioDefaultDevice> Defaults,
    IReadOnlyList<AudioEndpointLevel> Levels)
{
    public static AudioState Empty { get; } = new([], [], []);

    public string? DefaultFor(AudioRole role) => Defaults.FirstOrDefault(d => d.Role == role)?.DeviceId;

    public AudioDevice? FindDevice(string id) => Devices.FirstOrDefault(d => string.Equals(d.Id, id, StringComparison.OrdinalIgnoreCase));

    public AudioEndpointLevel? LevelOf(string deviceId) => Levels.FirstOrDefault(l => string.Equals(l.DeviceId, deviceId, StringComparison.OrdinalIgnoreCase));

    /// <summary>
    /// Mesmo dispositivo padrão em cada papel e mesmo volume/mudo nos dispositivos presentes nos dois lados
    /// (dispositivos conectados ou desconectados depois não contam como diferença).
    /// </summary>
    public bool IsEquivalentTo(AudioState other)
    {
        ArgumentNullException.ThrowIfNull(other);
        foreach (var role in Enum.GetValues<AudioRole>())
        {
            if (!string.Equals(DefaultFor(role), other.DefaultFor(role), StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }
        }

        return Levels.All(l => other.LevelOf(l.DeviceId) is not { } match || (l.Muted == match.Muted && l.HasSameVolume(match)));
    }
}

/// <summary>Leitura e alteração do áudio de saída (implementação por plataforma).</summary>
public interface IAudioEndpointService
{
    /// <summary>
    /// A plataforma consegue trocar o dispositivo padrão. No Windows isso depende de uma interface não documentada;
    /// sem ela, só volume e mudo são restaurados.
    /// </summary>
    bool CanSetDefaultDevice { get; }

    /// <summary>Estado atual. Falha da plataforma: <see cref="RuntimeEnvironmentException"/>.</summary>
    AudioState Capture();

    void SetDefaultDevice(string deviceId, AudioRole role);

    void SetVolume(string deviceId, float volume);

    void SetMute(string deviceId, bool muted);
}
