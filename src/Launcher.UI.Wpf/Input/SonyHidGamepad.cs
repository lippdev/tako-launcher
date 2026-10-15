using System.ComponentModel;
using System.Runtime.InteropServices;
using Launcher.Core.Input;
using Microsoft.Win32.SafeHandles;

namespace Launcher.UI.Wpf.Input;

/// <summary>
/// Controles PlayStation (DualSense, DualSense Edge, DualShock 4) lidos direto por HID, via USB ou Bluetooth, sem
/// Steam Input nem DS4Windows. Uma thread em segundo plano lê os relatórios; o launcher consulta a última leitura a cada
/// quadro. Sem controle conectado, procura de novo a cada 2 s.
/// </summary>
public sealed partial class SonyHidGamepad : IDisposable
{
    private const uint DigcfPresent = 0x02;
    private const uint DigcfDeviceInterface = 0x10;
    private const uint GenericRead = 0x80000000;
    private const uint GenericWrite = 0x40000000;
    private const uint FileShareReadWrite = 0x03;
    private const uint OpenExisting = 3;
    private const int RescanMilliseconds = 2000;

    private readonly Lock _gate = new();
    private SafeFileHandle? _handle;
    private SonyController _model;
    private GamepadSnapshot? _latest;
    private long _nextScan;

    public SonyController? Model
    {
        get
        {
            lock (_gate)
            {
                return _handle is null ? null : _model;
            }
        }
    }

    /// <summary>Última leitura do controle conectado, ou nulo.</summary>
    public GamepadSnapshot? Read()
    {
        lock (_gate)
        {
            if (_handle is not null)
            {
                return _latest;
            }
        }

        if (Environment.TickCount64 >= _nextScan)
        {
            _nextScan = Environment.TickCount64 + RescanMilliseconds;
            TryConnect();
        }

        return null;
    }

    public void Dispose()
    {
        SafeFileHandle? handle;
        lock (_gate)
        {
            handle = _handle;
            _handle = null;
        }

        // A thread de leitura está bloqueada em ReadFile: cancela a leitura para ela sair e fechar o handle.
        if (handle is { IsClosed: false })
        {
            CancelIoEx(handle, IntPtr.Zero);
        }
    }

    private void TryConnect()
    {
        foreach (string path in HidDevicePaths())
        {
            var handle = CreateFile(path, GenericRead | GenericWrite, FileShareReadWrite, IntPtr.Zero, OpenExisting, 0, IntPtr.Zero);
            if (handle.IsInvalid)
            {
                handle.Dispose();
                continue;
            }

            var attributes = new HiddAttributes { Size = (uint)Marshal.SizeOf<HiddAttributes>() };
            if (HidD_GetAttributes(handle, ref attributes) && SonyHidReport.Identify(attributes.VendorId, attributes.ProductId) is { } model)
            {
                lock (_gate)
                {
                    _handle = handle;
                    _model = model;
                    _latest = null;
                }

                new Thread(() => ReadLoop(handle, model)) { IsBackground = true, Name = "Tako Sony HID" }.Start();
                return;
            }

            handle.Dispose();
        }
    }

    private void ReadLoop(SafeFileHandle handle, SonyController model)
    {
        byte[] buffer = new byte[256];
        while (!handle.IsClosed)
        {
            if (!ReadFile(handle, buffer, (uint)buffer.Length, out uint read, IntPtr.Zero) || read == 0)
            {
                break; // desconectado (ou fechado no Dispose)
            }

            if (SonyHidReport.TryParse(model, buffer.AsSpan(0, (int)read), out var snapshot))
            {
                lock (_gate)
                {
                    _latest = snapshot;
                }
            }
        }

        lock (_gate)
        {
            if (_handle == handle)
            {
                _handle = null;
                _latest = null;
            }
        }

        handle.Dispose();
    }

    private static List<string> HidDevicePaths()
    {
        var paths = new List<string>();
        HidD_GetHidGuid(out Guid hidGuid);
        IntPtr set = SetupDiGetClassDevs(ref hidGuid, IntPtr.Zero, IntPtr.Zero, DigcfPresent | DigcfDeviceInterface);
        if (set == new IntPtr(-1))
        {
            throw new Win32Exception(Marshal.GetLastPInvokeError());
        }

        try
        {
            var data = new DeviceInterfaceData { Size = (uint)Marshal.SizeOf<DeviceInterfaceData>() };
            for (uint i = 0; SetupDiEnumDeviceInterfaces(set, IntPtr.Zero, ref hidGuid, i, ref data); i++)
            {
                SetupDiGetDeviceInterfaceDetail(set, ref data, IntPtr.Zero, 0, out uint required, IntPtr.Zero);
                if (required == 0)
                {
                    continue;
                }

                IntPtr detail = Marshal.AllocHGlobal((int)required);
                try
                {
                    // SP_DEVICE_INTERFACE_DETAIL_DATA_W: cbSize (8 em x64, 6 em x86) seguido do caminho UTF-16.
                    Marshal.WriteInt32(detail, IntPtr.Size == 8 ? 8 : 6);
                    if (SetupDiGetDeviceInterfaceDetail(set, ref data, detail, required, out _, IntPtr.Zero)
                        && Marshal.PtrToStringUni(detail + 4) is { Length: > 0 } path)
                    {
                        paths.Add(path);
                    }
                }
                finally
                {
                    Marshal.FreeHGlobal(detail);
                }
            }
        }
        finally
        {
            SetupDiDestroyDeviceInfoList(set);
        }

        return paths;
    }

    [LibraryImport("hid.dll")]
    private static partial void HidD_GetHidGuid(out Guid hidGuid);

    [LibraryImport("hid.dll")]
    [return: MarshalAs(UnmanagedType.U1)]
    private static partial bool HidD_GetAttributes(SafeFileHandle device, ref HiddAttributes attributes);

    [LibraryImport("setupapi.dll", EntryPoint = "SetupDiGetClassDevsW", SetLastError = true)]
    private static partial IntPtr SetupDiGetClassDevs(ref Guid classGuid, IntPtr enumerator, IntPtr parent, uint flags);

    [LibraryImport("setupapi.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetupDiEnumDeviceInterfaces(IntPtr deviceInfoSet, IntPtr deviceInfoData, ref Guid interfaceClassGuid, uint memberIndex, ref DeviceInterfaceData deviceInterfaceData);

    [LibraryImport("setupapi.dll", EntryPoint = "SetupDiGetDeviceInterfaceDetailW", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetupDiGetDeviceInterfaceDetail(IntPtr deviceInfoSet, ref DeviceInterfaceData deviceInterfaceData, IntPtr detail, uint detailSize, out uint requiredSize, IntPtr deviceInfoData);

    [LibraryImport("setupapi.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetupDiDestroyDeviceInfoList(IntPtr deviceInfoSet);

    [LibraryImport("kernel32.dll", EntryPoint = "CreateFileW", SetLastError = true, StringMarshalling = StringMarshalling.Utf16)]
    private static partial SafeFileHandle CreateFile(string fileName, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool ReadFile(SafeFileHandle file, [Out] byte[] buffer, uint bytesToRead, out uint bytesRead, IntPtr overlapped);

    [LibraryImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool CancelIoEx(SafeFileHandle file, IntPtr overlapped);

    [StructLayout(LayoutKind.Sequential)]
    private struct HiddAttributes
    {
        public uint Size;
        public ushort VendorId;
        public ushort ProductId;
        public ushort VersionNumber;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct DeviceInterfaceData
    {
        public uint Size;
        public Guid InterfaceClassGuid;
        public uint Flags;
        public IntPtr Reserved;
    }
}
