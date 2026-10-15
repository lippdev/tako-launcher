// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/AudioService.cs
namespace ConsoleMode.Runtime.Audio;

/// <summary>Saída padrão agora, com volume em porcentagem (0 a 100) e mudo.</summary>
public sealed record AudioOutputLevel(string DeviceId, string Name, int VolumePercent, bool Muted);

/// <summary>
/// Volume e saída de áudio para a Central de Controle durante o jogo: lê e ajusta a saída padrão (papel "jogos" do
/// Windows, ou multimídia se ele não existir). Falhas da plataforma chegam como <see cref="RuntimeEnvironmentException"/>.
/// </summary>
public sealed class AudioVolumeControl(IAudioEndpointService audio)
{
    /// <summary>Passo do volume com ← → (5%, como no Console Mode).</summary>
    public const int Step = 5;

    /// <summary>Pode trocar a saída padrão (ver <see cref="IAudioEndpointService.CanSetDefaultDevice"/>).</summary>
    public bool CanChangeOutput => audio.CanSetDefaultDevice;

    /// <summary>Saídas ativas (para escolher onde o som sai).</summary>
    public IReadOnlyList<AudioDevice> Outputs() => audio.Capture().Devices;

    /// <summary>Saída padrão e volume; nulo sem nenhuma saída de áudio.</summary>
    public AudioOutputLevel? Current() => LevelOf(audio.Capture());

    /// <summary>Ajusta o volume e tira do mudo, como girar o botão de volume.</summary>
    public AudioOutputLevel? SetVolumePercent(int percent)
    {
        if (Current() is not { } current)
        {
            return null;
        }

        int clamped = Math.Clamp(percent, 0, 100);
        audio.SetVolume(current.DeviceId, clamped / 100f);
        if (current.Muted)
        {
            audio.SetMute(current.DeviceId, false);
        }

        return current with { VolumePercent = clamped, Muted = false };
    }

    /// <summary>Sobe ou desce em passos de <see cref="Step"/>% (arredondando para o passo).</summary>
    public AudioOutputLevel? StepVolume(int steps)
    {
        if (Current() is not { } current)
        {
            return null;
        }

        int snapped = (int)Math.Round(current.VolumePercent / (double)Step) * Step;
        return SetVolumePercent(snapped + (steps * Step));
    }

    public AudioOutputLevel? SetMuted(bool muted)
    {
        if (Current() is not { } current)
        {
            return null;
        }

        audio.SetMute(current.DeviceId, muted);
        return current with { Muted = muted };
    }

    public AudioOutputLevel? ToggleMute() => Current() is { } current ? SetMuted(!current.Muted) : null;

    /// <summary>Faz o som sair por <paramref name="deviceId"/> em todos os papéis.</summary>
    public void SetOutput(string deviceId)
    {
        var state = audio.Capture();
        foreach (var role in AudioOutputPlanner.PlanSwitch(state, deviceId))
        {
            audio.SetDefaultDevice(deviceId, role.Role);
        }
    }

    /// <summary>Saída padrão do estado: papel "jogos", senão multimídia; volume desconhecido conta como 100%.</summary>
    public static AudioOutputLevel? LevelOf(AudioState state)
    {
        ArgumentNullException.ThrowIfNull(state);
        string? id = state.DefaultFor(AudioRole.Console) ?? state.DefaultFor(AudioRole.Multimedia);
        if (id is null)
        {
            return null;
        }

        string name = state.FindDevice(id)?.Name ?? id;
        var level = state.LevelOf(id);
        int percent = level is null ? 100 : (int)Math.Round(Math.Clamp(level.Volume, 0f, 1f) * 100);
        return new AudioOutputLevel(id, name, percent, level?.Muted ?? false);
    }
}
