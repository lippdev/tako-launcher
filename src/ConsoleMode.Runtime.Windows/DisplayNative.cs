using System.Runtime.InteropServices;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Declarações da API de configuração de monitores do Windows (CCD: <c>QueryDisplayConfig</c>/<c>SetDisplayConfig</c>)
/// e do caminho GDI (<c>EnumDisplaySettingsEx</c>/<c>ChangeDisplaySettingsEx</c>), escritas a partir da documentação
/// pública (wingdi.h / winuser.h). Estruturas passadas só por ponteiro: nada de marshalling em tempo de execução.
/// </summary>
internal static unsafe partial class DisplayNative
{
    public const int ErrorSuccess = 0;
    public const int ErrorInsufficientBuffer = 122;

    public const uint QdcOnlyActivePaths = 0x00000002;

    public const uint SdcUseSuppliedDisplayConfig = 0x00000020;
    public const uint SdcApply = 0x00000080;
    public const uint SdcSaveToDatabase = 0x00000200;
    public const uint SdcAllowChanges = 0x00000400;

    public const uint PathActive = 0x00000001;
    public const uint PathModeIdxInvalid = 0xffffffff;
    public const uint ModeInfoTypeSource = 1;

    public const uint DeviceInfoGetSourceName = 1;
    public const uint DeviceInfoGetTargetName = 2;

    // DISPLAYCONFIG_ROTATION
    public const uint RotationIdentity = 1;
    public const uint Rotation90 = 2;
    public const uint Rotation180 = 3;
    public const uint Rotation270 = 4;

    // GDI (DEVMODE / ChangeDisplaySettingsEx)
    public const int EnumCurrentSettings = -1;
    public const uint DmPosition = 0x00000020;
    public const uint DmDisplayOrientation = 0x00000080;
    public const uint DmPelsWidth = 0x00080000;
    public const uint DmPelsHeight = 0x00100000;
    public const uint DmDisplayFrequency = 0x00400000;
    public const uint CdsUpdateRegistry = 0x00000001;
    public const uint CdsSetPrimary = 0x00000010;
    public const uint CdsNoReset = 0x10000000;
    public const int DispChangeSuccessful = 0;
    public const int DispChangeRestart = 1;

    [LibraryImport("user32.dll")]
    public static partial int GetDisplayConfigBufferSizes(uint flags, uint* numPathArrayElements, uint* numModeInfoArrayElements);

    [LibraryImport("user32.dll")]
    public static partial int QueryDisplayConfig(uint flags, uint* numPathArrayElements, PathInfo* pathArray, uint* numModeInfoArrayElements, ModeInfo* modeInfoArray, nint currentTopologyId);

    [LibraryImport("user32.dll")]
    public static partial int SetDisplayConfig(uint numPathArrayElements, PathInfo* pathArray, uint numModeInfoArrayElements, ModeInfo* modeInfoArray, uint flags);

    [LibraryImport("user32.dll")]
    public static partial int DisplayConfigGetDeviceInfo(DeviceInfoHeader* requestPacket);

    [LibraryImport("user32.dll", EntryPoint = "EnumDisplaySettingsExW", StringMarshalling = StringMarshalling.Utf16)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool EnumDisplaySettingsEx(string deviceName, int modeNum, DevMode* devMode, uint flags);

    [LibraryImport("user32.dll", EntryPoint = "ChangeDisplaySettingsExW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial int ChangeDisplaySettingsEx(string? deviceName, DevMode* devMode, nint hwnd, uint flags, nint lParam);

    [StructLayout(LayoutKind.Sequential)]
    public struct Luid
    {
        public uint LowPart;
        public int HighPart;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Rational
    {
        public uint Numerator;
        public uint Denominator;
    }

    /// <summary>DISPLAYCONFIG_PATH_SOURCE_INFO (20 bytes).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct PathSourceInfo
    {
        public Luid AdapterId;
        public uint Id;
        public uint ModeInfoIdx;
        public uint StatusFlags;
    }

    /// <summary>DISPLAYCONFIG_PATH_TARGET_INFO (48 bytes).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct PathTargetInfo
    {
        public Luid AdapterId;
        public uint Id;
        public uint ModeInfoIdx;
        public uint OutputTechnology;
        public uint Rotation;
        public uint Scaling;
        public Rational RefreshRate;
        public uint ScanLineOrdering;
        public int TargetAvailable;
        public uint StatusFlags;
    }

    /// <summary>DISPLAYCONFIG_PATH_INFO (72 bytes).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct PathInfo
    {
        public PathSourceInfo SourceInfo;
        public PathTargetInfo TargetInfo;
        public uint Flags;
    }

    /// <summary>DISPLAYCONFIG_SOURCE_MODE (20 bytes).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct SourceMode
    {
        public uint Width;
        public uint Height;
        public uint PixelFormat;
        public int X;
        public int Y;
    }

    /// <summary>DISPLAYCONFIG_MODE_INFO (64 bytes; a união começa no deslocamento 16, alinhada a 8).</summary>
    [StructLayout(LayoutKind.Explicit, Size = 64)]
    public struct ModeInfo
    {
        [FieldOffset(0)]
        public uint InfoType;

        [FieldOffset(4)]
        public uint Id;

        [FieldOffset(8)]
        public Luid AdapterId;

        [FieldOffset(16)]
        public SourceMode SourceMode;
    }

    /// <summary>DISPLAYCONFIG_DEVICE_INFO_HEADER (20 bytes).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct DeviceInfoHeader
    {
        public uint Type;
        public uint Size;
        public Luid AdapterId;
        public uint Id;
    }

    /// <summary>DISPLAYCONFIG_SOURCE_DEVICE_NAME (84 bytes): nome GDI (<c>\\.\DISPLAY1</c>).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct SourceDeviceName
    {
        public DeviceInfoHeader Header;
        public fixed char ViewGdiDeviceName[32];
    }

    /// <summary>DISPLAYCONFIG_TARGET_DEVICE_NAME (420 bytes): nome amigável e caminho estável do monitor.</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct TargetDeviceName
    {
        public DeviceInfoHeader Header;
        public uint Flags;
        public uint OutputTechnology;
        public ushort EdidManufactureId;
        public ushort EdidProductCodeId;
        public uint ConnectorInstance;
        public fixed char MonitorFriendlyDeviceName[64];
        public fixed char MonitorDevicePath[128];
    }

    /// <summary>DEVMODEW (220 bytes), só com os campos de monitor usados aqui.</summary>
    [StructLayout(LayoutKind.Explicit, Size = 220)]
    public struct DevMode
    {
        [FieldOffset(68)]
        public ushort Size;

        [FieldOffset(72)]
        public uint Fields;

        [FieldOffset(76)]
        public int PositionX;

        [FieldOffset(80)]
        public int PositionY;

        [FieldOffset(84)]
        public uint DisplayOrientation;

        [FieldOffset(172)]
        public uint PelsWidth;

        [FieldOffset(176)]
        public uint PelsHeight;

        [FieldOffset(184)]
        public uint DisplayFrequency;
    }
}
