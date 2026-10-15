using System.Runtime.InteropServices;
using ConsoleMode.Runtime.Audio;
using static ConsoleMode.Runtime.Windows.AudioNative;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Áudio de saída no Windows pelo Core Audio: dispositivos ativos, padrão por papel (<c>IMMDeviceEnumerator</c>) e
/// volume/mudo (<c>IAudioEndpointVolume</c>). Trocar o dispositivo padrão usa <see cref="IPolicyConfig"/>, não
/// documentada (ver ADR-0013): se não estiver disponível, <see cref="CanSetDefaultDevice"/> fica falso.
/// Toda falha do Windows vira <see cref="RuntimeEnvironmentException"/>.
/// </summary>
public sealed class WindowsAudioEndpoints : IAudioEndpointService
{
    private readonly Lazy<IPolicyConfig?> _policy = new(CreatePolicyConfig);

    public bool CanSetDefaultDevice => _policy.Value is not null;

    public AudioState Capture() => Guard("ler o áudio", () =>
    {
        var enumerator = Enumerator();
        Check(enumerator.EnumAudioEndpoints(ERender, DeviceStateActive, out var collection), "EnumAudioEndpoints");
        if (collection is null)
        {
            throw new RuntimeEnvironmentException("EnumAudioEndpoints não devolveu a lista de dispositivos");
        }

        var devices = new List<AudioDevice>();
        var levels = new List<AudioEndpointLevel>();
        Check(collection.GetCount(out uint count), "IMMDeviceCollection.GetCount");
        for (uint i = 0; i < count; i++)
        {
            if (collection.Item(i, out var device) < 0 || device is null || IdOf(device) is not { } id)
            {
                continue;
            }

            devices.Add(new AudioDevice(id, FriendlyName(device) ?? id));
            if (Volume(device) is { } volume
                && volume.GetMasterVolumeLevelScalar(out float level) >= 0
                && volume.GetMute(out int muted) >= 0)
            {
                levels.Add(new AudioEndpointLevel(id, level, muted != 0));
            }
        }

        var defaults = new List<AudioDefaultDevice>();
        foreach (var role in Enum.GetValues<AudioRole>())
        {
            int hr = enumerator.GetDefaultAudioEndpoint(ERender, (int)role, out var endpoint);
            if (hr == ENotFound || endpoint is null)
            {
                continue; // nenhuma saída de áudio para este papel
            }

            Check(hr, "GetDefaultAudioEndpoint");
            if (IdOf(endpoint) is { } id)
            {
                defaults.Add(new AudioDefaultDevice(role, id));
            }
        }

        return new AudioState(devices, defaults, levels);
    });

    public void SetDefaultDevice(string deviceId, AudioRole role) => Guard("trocar a saída padrão", () =>
    {
        var policy = _policy.Value ?? throw new RuntimeEnvironmentException("Trocar a saída padrão não está disponível neste Windows (IPolicyConfig)");
        Check(policy.SetDefaultEndpoint(deviceId, (int)role), "IPolicyConfig.SetDefaultEndpoint");
        return true;
    });

    public void SetVolume(string deviceId, float volume) => Guard("ajustar o volume", () =>
    {
        Check(VolumeOf(deviceId).SetMasterVolumeLevelScalar(Math.Clamp(volume, 0f, 1f), 0), "SetMasterVolumeLevelScalar");
        return true;
    });

    public void SetMute(string deviceId, bool muted) => Guard("ajustar o mudo", () =>
    {
        Check(VolumeOf(deviceId).SetMute(muted ? 1 : 0, 0), "SetMute");
        return true;
    });

    private static IMMDeviceEnumerator Enumerator() =>
        Create<IMMDeviceEnumerator>(MMDeviceEnumeratorClsid, IMMDeviceEnumeratorIid)
            ?? throw new RuntimeEnvironmentException("MMDeviceEnumerator indisponível");

    private static IAudioEndpointVolume VolumeOf(string deviceId)
    {
        Check(Enumerator().GetDevice(deviceId, out var device), "GetDevice");
        return (device is null ? null : Volume(device)) ?? throw new RuntimeEnvironmentException($"Sem controle de volume em {deviceId}");
    }

    private static IAudioEndpointVolume? Volume(IMMDevice device)
    {
        int hr = device.Activate(IAudioEndpointVolumeIid, ClsctxAll, 0, out nint pointer);
        return hr < 0 || pointer == 0 ? null : Wrap<IAudioEndpointVolume>(pointer);
    }

    private static string? IdOf(IMMDevice device)
    {
        if (device.GetId(out nint text) < 0 || text == 0)
        {
            return null;
        }

        try
        {
            return Marshal.PtrToStringUni(text);
        }
        finally
        {
            Marshal.FreeCoTaskMem(text);
        }
    }

    private static unsafe string? FriendlyName(IMMDevice device)
    {
        if (device.OpenPropertyStore(StgmRead, out var store) < 0 || store is null)
        {
            return null;
        }

        if (store.GetValue(DeviceFriendlyName, out var value) < 0)
        {
            return null;
        }

        try
        {
            return value.VarType == VtLpwstr && value.Pointer != 0 ? Marshal.PtrToStringUni(value.Pointer) : null;
        }
        finally
        {
            _ = PropVariantClear(&value); // falha ao liberar só vaza o texto; o nome já foi lido
        }
    }

    /// <summary>Interface não documentada: qualquer falha ao criar só desliga a troca do dispositivo padrão.</summary>
    private static IPolicyConfig? CreatePolicyConfig()
    {
        try
        {
            return Create<IPolicyConfig>(PolicyConfigIds.PolicyConfigClientClsid, PolicyConfigIds.IPolicyConfigIid);
        }
        catch (Exception ex) when (IsPlatformFailure(ex))
        {
            return null;
        }
    }

    private static void Check(int hr, string call)
    {
        if (hr < 0)
        {
            throw new RuntimeEnvironmentException($"{call} falhou (0x{hr:X8})");
        }
    }

    private static T Guard<T>(string action, Func<T> body)
    {
        try
        {
            return body();
        }
        catch (Exception ex) when (IsPlatformFailure(ex))
        {
            throw new RuntimeEnvironmentException($"Não foi possível {action}: {ex.Message}", ex);
        }
    }

    private static bool IsPlatformFailure(Exception ex) =>
        ex is ExternalException or InvalidCastException or DllNotFoundException or EntryPointNotFoundException;
}
