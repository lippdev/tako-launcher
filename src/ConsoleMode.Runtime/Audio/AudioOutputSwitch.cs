// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/AudioService.cs
namespace ConsoleMode.Runtime.Audio;

/// <summary>Saída de áudio escolhida nas Configurações: id estável e o nome visto por último (para achar de novo e exibir).</summary>
public sealed record AudioOutputChoice(string DeviceId, string Name)
{
    /// <summary>
    /// O dispositivo ativo desta escolha: pelo id e, se o id não estiver ativo, pelo nome (a mesma TV numa porta HDMI
    /// diferente ganha outro id). Nome repetido em mais de um dispositivo não conta: seria um palpite.
    /// </summary>
    public AudioDevice? FindIn(AudioState state)
    {
        ArgumentNullException.ThrowIfNull(state);
        if (state.FindDevice(DeviceId) is { } byId)
        {
            return byId;
        }

        var byName = state.Devices.Where(d => string.Equals(d.Name, Name, StringComparison.OrdinalIgnoreCase)).ToList();
        return byName.Count == 1 ? byName[0] : null;
    }
}

/// <summary>Papel trocado pela sessão e quem era o padrão antes.</summary>
public sealed record AudioRoleSwitch(AudioRole Role, string? PreviousDeviceId, string DeviceId);

/// <summary>O que a troca de saída fez, para desfazer no fim.</summary>
public sealed record AudioOutputSwitchRecord(string DeviceId, IReadOnlyList<AudioRoleSwitch> Roles)
{
    public bool IsEmpty => Roles.Count == 0;
}

/// <summary>
/// Planos puros da troca de saída de áudio durante o jogo: no início, torna a saída escolhida o padrão de todos os papéis
/// (jogos, multimídia, comunicação); no fim, devolve cada papel ao dispositivo anterior, só se ele continuar com a saída
/// do jogo (se o usuário trocou durante o jogo, a escolha dele fica).
/// </summary>
public static class AudioOutputPlanner
{
    /// <summary>Papéis a trocar para <paramref name="deviceId"/> (os que já estão nele ficam de fora).</summary>
    public static IReadOnlyList<AudioRoleSwitch> PlanSwitch(AudioState current, string deviceId)
    {
        ArgumentNullException.ThrowIfNull(current);
        return [.. Enum.GetValues<AudioRole>()
            .Where(role => !string.Equals(current.DefaultFor(role), deviceId, StringComparison.OrdinalIgnoreCase))
            .Select(role => new AudioRoleSwitch(role, current.DefaultFor(role), deviceId))];
    }

    /// <summary>Papéis a devolver: ainda na saída do jogo, com um dispositivo anterior que continua ativo.</summary>
    public static IReadOnlyList<AudioRoleSwitch> PlanRestore(AudioState current, AudioOutputSwitchRecord record)
    {
        ArgumentNullException.ThrowIfNull(current);
        ArgumentNullException.ThrowIfNull(record);
        return [.. record.Roles.Where(r =>
            r.PreviousDeviceId is { } previous
            && current.FindDevice(previous) is not null
            && string.Equals(current.DefaultFor(r.Role), r.DeviceId, StringComparison.OrdinalIgnoreCase))];
    }
}

/// <summary>Resultado de uma troca de saída (o motivo vai para o log quando nada mudou).</summary>
public enum AudioSwitchOutcome
{
    Switched,

    /// <summary>A saída escolhida já era a padrão.</summary>
    AlreadyDefault,

    /// <summary>A saída escolhida não está ativa (ex.: TV desligada ou desconectada).</summary>
    DeviceMissing,

    /// <summary>Este Windows não deixa trocar a saída padrão (ADR-0013, <c>IPolicyConfig</c>).</summary>
    Unsupported,
}

/// <summary>Troca e devolve a saída padrão pelo <see cref="IAudioEndpointService"/>. Falhas da plataforma passam adiante.</summary>
public sealed class AudioOutputSwitch(IAudioEndpointService audio)
{
    /// <summary>Torna <paramref name="choice"/> a saída padrão. O registro devolvido desfaz exatamente o que mudou.</summary>
    public (AudioSwitchOutcome Outcome, AudioOutputSwitchRecord? Record) Apply(AudioOutputChoice choice)
    {
        ArgumentNullException.ThrowIfNull(choice);
        if (!audio.CanSetDefaultDevice)
        {
            return (AudioSwitchOutcome.Unsupported, null);
        }

        var state = audio.Capture();
        if (choice.FindIn(state) is not { } device)
        {
            return (AudioSwitchOutcome.DeviceMissing, null);
        }

        var roles = AudioOutputPlanner.PlanSwitch(state, device.Id);
        if (roles.Count == 0)
        {
            return (AudioSwitchOutcome.AlreadyDefault, null);
        }

        var applied = new List<AudioRoleSwitch>();
        try
        {
            foreach (var role in roles)
            {
                audio.SetDefaultDevice(device.Id, role.Role);
                applied.Add(role);
            }
        }
        catch (RuntimeEnvironmentException) when (applied.Count > 0)
        {
            // Troca pela metade: desfaz o que já mudou antes de passar o erro adiante.
            try
            {
                Restore(new AudioOutputSwitchRecord(device.Id, applied));
            }
            catch (RuntimeEnvironmentException)
            {
                // O erro que importa é o da troca; a restauração do fim da sessão tenta de novo.
            }

            throw;
        }

        return (AudioSwitchOutcome.Switched, new AudioOutputSwitchRecord(device.Id, applied));
    }

    /// <summary>Devolve os papéis que ainda estão na saída do jogo. Devolve quantos voltaram.</summary>
    public int Restore(AudioOutputSwitchRecord record)
    {
        ArgumentNullException.ThrowIfNull(record);
        var roles = AudioOutputPlanner.PlanRestore(audio.Capture(), record);
        foreach (var role in roles)
        {
            audio.SetDefaultDevice(role.PreviousDeviceId!, role.Role); // PlanRestore só devolve papéis com anterior
        }

        return roles.Count;
    }
}
