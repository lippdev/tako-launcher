// Adaptado de lippdev/consolemode: src/ConsoleMode/Native/CcdHelper.cs e src/ConsoleMode/Native/NativeDisplays.cs
using System.Globalization;
using ConsoleMode.Runtime.Display;
using static ConsoleMode.Runtime.Windows.DisplayNative;

namespace ConsoleMode.Runtime.Windows;

/// <summary>Consultas da API CCD compartilhadas pela leitura/restauração (ADR-0013) e pelos comandos da "Tela de jogo".</summary>
internal static unsafe class Ccd
{
    public const uint ApplyFlags = SdcApply | SdcUseSuppliedDisplayConfig | SdcSaveToDatabase | SdcAllowChanges;

    public static (PathInfo[] Paths, ModeInfo[] Modes) Query(uint flags)
    {
        // A configuração pode mudar entre a consulta do tamanho e a leitura: tenta de novo algumas vezes.
        for (int attempt = 0; attempt < 5; attempt++)
        {
            uint pathCount;
            uint modeCount;
            int sizes = GetDisplayConfigBufferSizes(flags, &pathCount, &modeCount);
            if (sizes != ErrorSuccess)
            {
                throw new RuntimeEnvironmentException($"GetDisplayConfigBufferSizes falhou ({sizes})");
            }

            var paths = new PathInfo[pathCount];
            var modes = new ModeInfo[modeCount];
            int result;
            fixed (PathInfo* p = paths)
            fixed (ModeInfo* m = modes)
            {
                result = QueryDisplayConfig(flags, &pathCount, p, &modeCount, m, 0);
            }

            if (result == ErrorSuccess)
            {
                return (paths[..(int)pathCount], modes[..(int)modeCount]);
            }

            if (result != ErrorInsufficientBuffer)
            {
                throw new RuntimeEnvironmentException($"QueryDisplayConfig falhou ({result})");
            }
        }

        throw new RuntimeEnvironmentException("QueryDisplayConfig: a configuração de monitores não parou de mudar");
    }

    public static int Set(PathInfo[] paths, ModeInfo[] modes, uint flags = ApplyFlags)
    {
        fixed (PathInfo* p = paths)
        fixed (ModeInfo* m = modes)
        {
            return SetDisplayConfig((uint)paths.Length, p, (uint)modes.Length, m, flags);
        }
    }

    /// <summary>Liga todos os monitores conectados em modo estendido (último recurso do Console Mode para religar a TV).</summary>
    public static int ExtendAll() => SetDisplayConfig(0, null, 0, null, SdcApply | SdcTopologyExtend);

    /// <summary>Identificador estável do monitor: o caminho do dispositivo, ou adaptador:alvo se o driver não informar.</summary>
    public static string MonitorId(Luid adapter, uint targetId, string? devicePath) =>
        devicePath ?? string.Create(CultureInfo.InvariantCulture, $"{DescribeLuid(adapter)}:{targetId}");

    public static (string? FriendlyName, string? DevicePath) TargetName(Luid adapter, uint targetId)
    {
        TargetDeviceName request = default;
        request.Header.Type = DeviceInfoGetTargetName;
        request.Header.Size = (uint)sizeof(TargetDeviceName);
        request.Header.AdapterId = adapter;
        request.Header.Id = targetId;
        if (DisplayConfigGetDeviceInfo(&request.Header) != ErrorSuccess)
        {
            return (null, null);
        }

        return (NullIfEmpty(new string(request.MonitorFriendlyDeviceName, 0, 64)), NullIfEmpty(new string(request.MonitorDevicePath, 0, 128)));
    }

    public static string? SourceGdiName(Luid adapter, uint sourceId)
    {
        SourceDeviceName request = default;
        request.Header.Type = DeviceInfoGetSourceName;
        request.Header.Size = (uint)sizeof(SourceDeviceName);
        request.Header.AdapterId = adapter;
        request.Header.Id = sourceId;
        return DisplayConfigGetDeviceInfo(&request.Header) == ErrorSuccess ? NullIfEmpty(new string(request.ViewGdiDeviceName, 0, 32)) : null;
    }

    public static HdrStatus GetHdr(Luid adapter, uint targetId)
    {
        GetAdvancedColorInfo info = default;
        info.Header.Type = DeviceInfoGetAdvancedColorInfo;
        info.Header.Size = (uint)sizeof(GetAdvancedColorInfo);
        info.Header.AdapterId = adapter;
        info.Header.Id = targetId;
        if (DisplayConfigGetDeviceInfo(&info.Header) != ErrorSuccess)
        {
            return HdrStatus.Unknown;
        }

        bool supported = (info.Value & 0x1) != 0;
        bool enabled = (info.Value & 0x2) != 0;
        return !supported ? HdrStatus.Unsupported : enabled ? HdrStatus.On : HdrStatus.Off;
    }

    public static int SetHdr(Luid adapter, uint targetId, bool enabled)
    {
        SetAdvancedColorState state = default;
        state.Header.Type = DeviceInfoSetAdvancedColorState;
        state.Header.Size = (uint)sizeof(SetAdvancedColorState);
        state.Header.AdapterId = adapter;
        state.Header.Id = targetId;
        state.EnableAdvancedColor = enabled ? 1u : 0u;
        return DisplayConfigSetDeviceInfo(&state.Header);
    }

    public static long Key(Luid id) => ((long)id.HighPart << 32) | id.LowPart;

    public static string DescribeLuid(Luid luid) => string.Create(CultureInfo.InvariantCulture, $"{luid.HighPart:X8}{luid.LowPart:X8}");

    private static string? NullIfEmpty(string text)
    {
        int end = text.IndexOf('\0', StringComparison.Ordinal);
        string value = end >= 0 ? text[..end] : text;
        return value.Length == 0 ? null : value;
    }
}
