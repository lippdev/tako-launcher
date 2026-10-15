// Adaptado de lippdev/consolemode: src/ConsoleMode/Native/NativeWindows.cs, Native/WindowSwitcher.cs, Native/WindowIcons.cs,
// Native/PerfCounters.cs, Native/GpuKmt.cs, Native/SystemMemory.cs, Services/ControllerMouse.cs e Services/CursorHider.cs
using System.Runtime.InteropServices;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Declarações Win32 das ferramentas da Central de Controle (janelas, ícones, ponteiro, contadores de desempenho e
/// driver de vídeo), escritas a partir da documentação pública (winuser.h, wingdi.h, pdh.h, d3dkmthk.h, xinput.h).
/// </summary>
internal static unsafe partial class SessionToolsNative
{
    public const int GwlExStyle = -20;
    public const long WsExToolWindow = 0x00000080;
    public const long WsExAppWindow = 0x00040000;
    public const uint GwOwner = 4;
    public const int DwmaCloaked = 14;
    public const int SwRestore = 9;
    public const int SwMaximize = 3;
    public const uint WmClose = 0x0010;
    public const uint WmGetIcon = 0x007F;
    public const nint IconBig = 1;
    public const nint IconSmall2 = 2;
    public const int GclpHIcon = -14;
    public const int GclpHIconSm = -34;
    public const uint SmtoAbortIfHung = 0x0002;
    public const uint ProcessQueryLimitedInformation = 0x1000;

    [LibraryImport("user32.dll")]
    public static partial int EnumWindows(delegate* unmanaged<nint, nint, int> callback, nint lParam);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsWindowVisible(nint hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsWindow(nint hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsIconic(nint hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool ShowWindow(nint hwnd, int command);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool BringWindowToTop(nint hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SetForegroundWindow(nint hwnd);

    [LibraryImport("user32.dll")]
    public static partial nint GetForegroundWindow();

    [LibraryImport("user32.dll")]
    public static partial nint GetWindow(nint hwnd, uint command);

    [LibraryImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    public static partial nint GetWindowLongPtr(nint hwnd, int index);

    [LibraryImport("user32.dll", EntryPoint = "GetWindowTextW")]
    public static partial int GetWindowText(nint hwnd, char* text, int max);

    [LibraryImport("user32.dll", EntryPoint = "GetClassNameW")]
    public static partial int GetClassName(nint hwnd, char* text, int max);

    [LibraryImport("user32.dll")]
    public static partial uint GetWindowThreadProcessId(nint hwnd, uint* processId);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool AttachThreadInput(uint attach, uint to, [MarshalAs(UnmanagedType.Bool)] bool on);

    [LibraryImport("user32.dll", EntryPoint = "PostMessageW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool PostMessage(nint hwnd, uint message, nint wParam, nint lParam);

    [LibraryImport("user32.dll", EntryPoint = "SendMessageTimeoutW")]
    public static partial nint SendMessageTimeout(nint hwnd, uint message, nint wParam, nint lParam, uint flags, uint timeoutMs, nint* result);

    [LibraryImport("user32.dll", EntryPoint = "GetClassLongPtrW")]
    public static partial nint GetClassLongPtr(nint hwnd, int index);

    [LibraryImport("user32.dll", EntryPoint = "PrivateExtractIconsW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial uint PrivateExtractIcons(string file, int index, int width, int height, nint* icons, uint* ids, uint count, uint flags);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool DestroyIcon(nint icon);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool GetIconInfo(nint icon, IconInfo* info);

    [LibraryImport("user32.dll")]
    public static partial nint GetDC(nint hwnd);

    [LibraryImport("user32.dll")]
    public static partial int ReleaseDC(nint hwnd, nint dc);

    [LibraryImport("user32.dll", SetLastError = true)]
    public static partial uint SendInput(uint count, Input* inputs, int size);

    [LibraryImport("user32.dll", SetLastError = true)]
    public static partial nint CreateCursor(nint instance, int hotX, int hotY, int width, int height, byte* andPlane, byte* xorPlane);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool DestroyCursor(nint cursor);

    [LibraryImport("user32.dll")]
    public static partial nint CopyIcon(nint icon);

    [LibraryImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SetSystemCursor(nint cursor, uint id);

    [LibraryImport("user32.dll", EntryPoint = "SystemParametersInfoW", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SystemParametersInfo(uint action, uint param, nint value, uint winIni);

    [LibraryImport("user32.dll")]
    public static partial int GetSystemMetrics(int index);

    [LibraryImport("kernel32.dll")]
    public static partial uint GetCurrentThreadId();

    [LibraryImport("kernel32.dll")]
    public static partial nint OpenProcess(uint access, [MarshalAs(UnmanagedType.Bool)] bool inherit, uint processId);

    [LibraryImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool CloseHandle(nint handle);

    [LibraryImport("kernel32.dll", EntryPoint = "QueryFullProcessImageNameW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool QueryFullProcessImageName(nint process, uint flags, char* name, int* size);

    [LibraryImport("kernel32.dll", EntryPoint = "GetModuleHandleW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial nint GetModuleHandle(string? moduleName);

    [LibraryImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool GlobalMemoryStatusEx(MemoryStatusEx* status);

    [LibraryImport("dwmapi.dll")]
    public static partial int DwmGetWindowAttribute(nint hwnd, int attribute, int* value, int size);

    [LibraryImport("gdi32.dll", EntryPoint = "GetObjectW")]
    public static partial int GetObject(nint handle, int size, Bitmap* bitmap);

    [LibraryImport("gdi32.dll")]
    public static partial int GetDIBits(nint dc, nint bitmap, uint start, uint lines, byte* bits, BitmapInfoHeader* info, uint usage);

    [LibraryImport("gdi32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool DeleteObject(nint handle);

    [LibraryImport("gdi32.dll")]
    public static partial int D3DKMTOpenAdapterFromLuid(OpenAdapterFromLuid* open);

    [LibraryImport("gdi32.dll")]
    public static partial int D3DKMTCloseAdapter(uint* adapter);

    [LibraryImport("gdi32.dll")]
    public static partial int D3DKMTQueryAdapterInfo(QueryAdapterInfo* query);

    [LibraryImport("pdh.dll", EntryPoint = "PdhOpenQueryW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial int PdhOpenQuery(string? dataSource, nint userData, nint* query);

    [LibraryImport("pdh.dll", EntryPoint = "PdhAddEnglishCounterW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial int PdhAddEnglishCounter(nint query, string path, nint userData, nint* counter);

    [LibraryImport("pdh.dll")]
    public static partial int PdhCollectQueryData(nint query);

    [LibraryImport("pdh.dll")]
    public static partial int PdhGetFormattedCounterValue(nint counter, uint format, uint* type, FmtCounterValue* value);

    [LibraryImport("pdh.dll", EntryPoint = "PdhGetFormattedCounterArrayW")]
    public static partial int PdhGetFormattedCounterArray(nint counter, uint format, uint* bufferSize, uint* itemCount, byte* buffer);

    [LibraryImport("pdh.dll")]
    public static partial int PdhCloseQuery(nint query);

    [LibraryImport("xinput1_4.dll", EntryPoint = "XInputGetState")]
    public static partial uint XInputGetState(uint userIndex, XInputState* state);

    [StructLayout(LayoutKind.Sequential)]
    public struct IconInfo
    {
        public int IsIcon;
        public int XHotspot;
        public int YHotspot;
        public nint Mask;
        public nint Color;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Bitmap
    {
        public int Type;
        public int Width;
        public int Height;
        public int WidthBytes;
        public ushort Planes;
        public ushort BitsPixel;
        public nint Bits;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct BitmapInfoHeader
    {
        public int Size;
        public int Width;
        public int Height;
        public ushort Planes;
        public ushort BitCount;
        public int Compression;
        public int SizeImage;
        public int XPelsPerMeter;
        public int YPelsPerMeter;
        public int ClrUsed;
        public int ClrImportant;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct MouseInput
    {
        public int Dx;
        public int Dy;
        public uint MouseData;
        public uint Flags;
        public uint Time;
        public nint ExtraInfo;
    }

    /// <summary>INPUT só com o ramo de mouse (o maior da união em x64).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Input
    {
        public uint Type;
        public MouseInput Mouse;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct XInputGamepad
    {
        public ushort Buttons;
        public byte LeftTrigger;
        public byte RightTrigger;
        public short ThumbLX;
        public short ThumbLY;
        public short ThumbRX;
        public short ThumbRY;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct XInputState
    {
        public uint PacketNumber;
        public XInputGamepad Gamepad;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct MemoryStatusEx
    {
        public uint Length;
        public uint MemoryLoad;
        public ulong TotalPhys;
        public ulong AvailPhys;
        public ulong TotalPageFile;
        public ulong AvailPageFile;
        public ulong TotalVirtual;
        public ulong AvailVirtual;
        public ulong AvailExtendedVirtual;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct FmtCounterValue
    {
        public uint Status;
        public double Value;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct FmtCounterValueItem
    {
        public nint Name;
        public FmtCounterValue Value;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct OpenAdapterFromLuid
    {
        public uint LuidLow;
        public int LuidHigh;
        public uint Adapter;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct QueryAdapterInfo
    {
        public uint Adapter;
        public int Type;
        public nint Data;
        public uint DataSize;
    }

    // d3dkmthk.h: D3DKMT_SEGMENTSIZEINFO, D3DKMT_NODE_PERFDATA, D3DKMT_ADAPTER_PERFDATA (ULONGLONGs alinhados em 8).
    [StructLayout(LayoutKind.Sequential)]
    public struct SegmentSizeInfo
    {
        public ulong DedicatedVideoMemorySize;
        public ulong DedicatedSystemMemorySize;
        public ulong SharedSystemMemorySize;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct NodePerfData
    {
        public uint NodeOrdinal;
        public uint PhysicalAdapterIndex;
        public ulong Frequency;
        public ulong MaxFrequency;
        public ulong MaxFrequencyOc;
        public uint Voltage;
        public uint VoltageMax;
        public uint VoltageMaxOc;
        public ulong MaxTransitionLatency;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct AdapterPerfData
    {
        public uint PhysicalAdapterIndex;
        public ulong MemoryFrequency;
        public ulong MaxMemoryFrequency;
        public ulong MaxMemoryFrequencyOc;
        public ulong MemoryBandwidth;
        public ulong PcieBandwidth;
        public uint FanRpm;
        public uint Power;
        public uint Temperature;
        public byte PowerStateOverride;
    }
}
