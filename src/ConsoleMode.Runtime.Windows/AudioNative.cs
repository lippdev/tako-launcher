using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Core Audio (mmdeviceapi.h / endpointvolume.h), declarado a partir da documentação pública com interfaces COM
/// geradas em tempo de compilação. Todos os métodos devolvem o HRESULT: quem chama decide o que é falha.
/// </summary>
internal static partial class AudioNative
{
    public const int ERender = 0;
    public const uint DeviceStateActive = 0x00000001;
    public const uint ClsctxInprocServer = 0x1;
    public const uint ClsctxAll = 0x17;
    public const uint StgmRead = 0x0;
    public const ushort VtLpwstr = 31;

    /// <summary>HRESULT_FROM_WIN32(ERROR_NOT_FOUND): não há dispositivo padrão para o papel.</summary>
    public const int ENotFound = unchecked((int)0x80070490);

    public static readonly Guid MMDeviceEnumeratorClsid = new("BCDE0395-E52F-467C-8E3D-C4579291692E");
    public static readonly Guid IMMDeviceEnumeratorIid = new("A95664D2-9614-4F35-A746-DE8DB63617E6");
    public static readonly Guid IAudioEndpointVolumeIid = new("5CDF2C82-841E-4546-9722-0CF74078229A");

    /// <summary>PKEY_Device_FriendlyName.</summary>
    public static readonly PropertyKey DeviceFriendlyName = new(new Guid("A45C254E-DF1C-4EFD-8020-67D146A850E0"), 14);

    public static readonly StrategyBasedComWrappers Wrappers = new();

    [LibraryImport("ole32.dll")]
    public static partial int CoCreateInstance(in Guid clsid, nint outer, uint clsContext, in Guid iid, out nint instance);

    [LibraryImport("ole32.dll")]
    public static unsafe partial int PropVariantClear(PropVariant* value);

    /// <summary>Cria o objeto COM e devolve a interface pedida; nulo se a classe ou a interface não existir.</summary>
    public static T? Create<T>(Guid clsid, Guid iid)
        where T : class
    {
        int hr = CoCreateInstance(clsid, 0, ClsctxInprocServer, iid, out nint pointer);
        return hr < 0 || pointer == 0 ? null : Wrap<T>(pointer);
    }

    /// <summary>Envolve um ponteiro de interface recebido (assume a referência).</summary>
    public static T Wrap<T>(nint pointer)
        where T : class
    {
        try
        {
            return (T)Wrappers.GetOrCreateObjectForComInstance(pointer, CreateObjectFlags.None);
        }
        finally
        {
            Marshal.Release(pointer);
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    public readonly struct PropertyKey(Guid formatId, uint propertyId)
    {
        public readonly Guid FormatId = formatId;
        public readonly uint PropertyId = propertyId;
    }

    /// <summary>PROPVARIANT: só o tipo e o primeiro ponteiro da união são lidos.</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct PropVariant
    {
        public ushort VarType;
        public ushort Reserved1;
        public ushort Reserved2;
        public ushort Reserved3;
        public nint Pointer;
        public nint Extra;
    }
}

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6")]
internal partial interface IMMDeviceEnumerator
{
    [PreserveSig]
    int EnumAudioEndpoints(int dataFlow, uint stateMask, out IMMDeviceCollection? devices);

    [PreserveSig]
    int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice? endpoint);

    [PreserveSig]
    int GetDevice(string id, out IMMDevice? device);
}

[GeneratedComInterface]
[Guid("0BD7A1BE-7A1A-44DB-8397-CC5392387B5E")]
internal partial interface IMMDeviceCollection
{
    [PreserveSig]
    int GetCount(out uint count);

    [PreserveSig]
    int Item(uint index, out IMMDevice? device);
}

[GeneratedComInterface]
[Guid("D666063F-1587-4E43-81F1-B948E807363F")]
internal partial interface IMMDevice
{
    [PreserveSig]
    int Activate(in Guid iid, uint clsContext, nint activationParams, out nint instance);

    [PreserveSig]
    int OpenPropertyStore(uint access, out IPropertyStore? properties);

    /// <summary>O texto é alocado pelo Windows (CoTaskMemAlloc): quem chama libera.</summary>
    [PreserveSig]
    int GetId(out nint id);

    [PreserveSig]
    int GetState(out uint state);
}

[GeneratedComInterface]
[Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99")]
internal partial interface IPropertyStore
{
    [PreserveSig]
    int GetCount(out uint count);

    [PreserveSig]
    int GetAt(uint index, out AudioNative.PropertyKey key);

    [PreserveSig]
    int GetValue(in AudioNative.PropertyKey key, out AudioNative.PropVariant value);
}

/// <summary>Só os métodos usados, na ordem da tabela virtual (os anteriores ficam como marcadores, nunca chamados).</summary>
[GeneratedComInterface]
[Guid("5CDF2C82-841E-4546-9722-0CF74078229A")]
internal partial interface IAudioEndpointVolume
{
    [PreserveSig]
    int RegisterControlChangeNotify(nint notify);

    [PreserveSig]
    int UnregisterControlChangeNotify(nint notify);

    [PreserveSig]
    int GetChannelCount(out uint count);

    [PreserveSig]
    int SetMasterVolumeLevel(float levelDb, nint eventContext);

    [PreserveSig]
    int SetMasterVolumeLevelScalar(float level, nint eventContext);

    [PreserveSig]
    int GetMasterVolumeLevel(out float levelDb);

    [PreserveSig]
    int GetMasterVolumeLevelScalar(out float level);

    [PreserveSig]
    int SetChannelVolumeLevel(uint channel, float levelDb, nint eventContext);

    [PreserveSig]
    int SetChannelVolumeLevelScalar(uint channel, float level, nint eventContext);

    [PreserveSig]
    int GetChannelVolumeLevel(uint channel, out float levelDb);

    [PreserveSig]
    int GetChannelVolumeLevelScalar(uint channel, out float level);

    [PreserveSig]
    int SetMute(int mute, nint eventContext);

    [PreserveSig]
    int GetMute(out int mute);
}

/// <summary>
/// <c>IPolicyConfig</c>: interface NÃO documentada do Windows, a única forma de trocar o dispositivo de áudio padrão
/// (usada há anos por ferramentas de troca de saída). Pode sumir ou mudar numa atualização do Windows; por isso fica
/// isolada aqui, é criada sob demanda e qualquer falha só desliga a troca do dispositivo padrão (volume e mudo continuam).
/// Só <see cref="SetDefaultEndpoint"/> é chamado; os anteriores ocupam as posições da tabela virtual.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("F8679F50-850A-41CF-9C72-430F290290C8")]
internal partial interface IPolicyConfig
{
    [PreserveSig]
    int GetMixFormat(nint deviceId, nint format);

    [PreserveSig]
    int GetDeviceFormat(nint deviceId, int defaultFormat, nint format);

    [PreserveSig]
    int ResetDeviceFormat(nint deviceId);

    [PreserveSig]
    int SetDeviceFormat(nint deviceId, nint endpointFormat, nint mixFormat);

    [PreserveSig]
    int GetProcessingPeriod(nint deviceId, int defaultPeriod, nint defaultPeriodOut, nint minimumPeriod);

    [PreserveSig]
    int SetProcessingPeriod(nint deviceId, nint period);

    [PreserveSig]
    int GetShareMode(nint deviceId, nint mode);

    [PreserveSig]
    int SetShareMode(nint deviceId, nint mode);

    [PreserveSig]
    int GetPropertyValue(nint deviceId, nint key, nint value);

    [PreserveSig]
    int SetPropertyValue(nint deviceId, nint key, nint value);

    [PreserveSig]
    int SetDefaultEndpoint(string deviceId, int role);
}

internal static class PolicyConfigIds
{
    public static readonly Guid PolicyConfigClientClsid = new("870AF99C-171D-4F9E-AF0D-E63DF40C2BC9");
    public static readonly Guid IPolicyConfigIid = new("F8679F50-850A-41CF-9C72-430F290290C8");
}
