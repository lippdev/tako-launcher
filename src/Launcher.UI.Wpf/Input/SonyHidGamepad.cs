using System.ComponentModel;
using System.Globalization;
using System.Runtime.InteropServices;
using Launcher.Core.Input;
using Microsoft.Win32.SafeHandles;

namespace Launcher.UI.Wpf.Input;

/// <summary>
/// Controles PlayStation (DualSense, DualSense Edge, DualShock 4) lidos direto por HID, via USB ou Bluetooth, sem
/// Steam Input nem DS4Windows. Uma thread em segundo plano lê os relatórios; o launcher consulta a última leitura a cada
/// quadro. Sem controle conectado, procura de novo a cada 2 s. Guarda os últimos relatórios crus para o teste de
/// controle (Configurações › Diagnóstico do sistema).
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
    private const int HidpStatusSuccess = 0x00110000;
    private const int HistorySize = 6;
    private const long HistoryEveryMilliseconds = 500;

    private readonly Lock _gate = new();
    private readonly LinkedList<string> _history = new();
    private SafeFileHandle? _handle;
    private SonyController _model;
    private HidTransport _transport;
    private ushort _productId;
    private GamepadSnapshot? _latest;
    private SonyHidReading? _reading;
    private byte[]? _lastRaw;
    private long _nextScan;
    private long _lastHistoryAt;

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
            try
            {
                TryConnect();
            }
            catch (Win32Exception)
            {
                // Falha ao enumerar dispositivos HID (driver ocupado, sessão remota): tenta de novo no próximo ciclo,
                // sem derrubar a leitura do teclado e do XInput (MVP-031).
            }
        }

        return null;
    }

    /// <summary>
    /// Estado do controle aberto para o teste de controle e o diagnóstico: modelo, USB/Bluetooth, product id, formato do
    /// relatório, botões ao vivo e os últimos relatórios crus. Sem o caminho do dispositivo (o do Bluetooth traz o
    /// endereço do controle). Nulo sem controle PlayStation aberto.
    /// </summary>
    public ControllerProbe? Probe()
    {
        lock (_gate)
        {
            if (_handle is null)
            {
                return null;
            }

            string layout = _lastRaw is { Length: > 0 } raw ? ControllerTest.LayoutLabel(_reading?.Layout, raw[0], raw.Length) : "nenhum relatório ainda";
            string state = _reading is { } reading ? ControllerTest.Describe(reading) : "sem leitura reconhecida (veja os relatórios crus)";
            var rawReports = new List<string>();
            if (_lastRaw is { } last)
            {
                rawReports.Add("agora: " + ControllerTest.Hex(last));
            }

            rawReports.AddRange(_history);
            string model = _model switch
            {
                SonyController.DualSenseEdge => "DualSense Edge",
                SonyController.DualSense => "DualSense",
                _ => "DualShock 4",
            };
            return new ControllerProbe("HID", model, _transport, SonyHidReport.SonyVendorId, _productId, layout, state, rawReports);
        }
    }

    /// <summary>
    /// Controles PlayStation conectados, só pelos caminhos HID (sem abrir o dispositivo, para não disputar a leitura).
    /// Usado pelo diagnóstico do sistema (MVP-034).
    /// </summary>
    public static IReadOnlyList<SonyController> ListConnected()
    {
        try
        {
            return HidDevicePath.SonyControllers(HidDevicePaths());
        }
        catch (Win32Exception)
        {
            return [];
        }
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
            if (HidD_GetAttributes(handle, ref attributes)
                && SonyHidReport.Identify(attributes.VendorId, attributes.ProductId) is { } model
                && ReadCaps(handle) is var caps
                && caps is not { IsGamepad: false })
            {
                // No Bluetooth o relatório 0x01 é o simples (botões em outro byte), mesmo completado com zeros até o
                // tamanho do 0x31: o transporte vem do caminho e, na dúvida, do tamanho (USB = 64 bytes).
                var transport = HidDevicePath.IsBluetooth(path) || caps is { InputLength: > 64 } ? HidTransport.Bluetooth : HidTransport.Usb;
                int length = caps is { InputLength: > 0 and var input } ? input : 256;
                lock (_gate)
                {
                    _handle = handle;
                    _model = model;
                    _transport = transport;
                    _productId = attributes.ProductId;
                    _latest = null;
                    _reading = null;
                    _lastRaw = null;
                    _history.Clear();
                }

                new Thread(() => ReadLoop(handle, model, transport, length)) { IsBackground = true, Name = "Tako Sony HID" }.Start();
                return;
            }

            handle.Dispose();
        }
    }

    private void ReadLoop(SafeFileHandle handle, SonyController model, HidTransport transport, int length)
    {
        // Um relatório por leitura: buffer do tamanho que o Windows informa (InputReportByteLength).
        byte[] buffer = new byte[length];
        while (!handle.IsClosed)
        {
            if (!ReadFile(handle, buffer, (uint)buffer.Length, out uint read, IntPtr.Zero) || read == 0)
            {
                break; // desconectado (ou fechado no Dispose)
            }

            var report = buffer.AsSpan(0, (int)read);
            bool parsed = SonyHidReport.TryRead(model, transport, report, out var reading);
            lock (_gate)
            {
                if (parsed)
                {
                    _latest = reading.Snapshot;
                }

                Remember(report, parsed ? reading : null);
            }
        }

        lock (_gate)
        {
            if (_handle == handle)
            {
                _handle = null;
                _latest = null;
                _reading = null;
            }
        }

        handle.Dispose();
    }

    /// <summary>
    /// Último relatório cru e um histórico curto: entra quando os botões mudam (para ver o que o controle manda ao
    /// apertar ↑) e, parado, a cada meio segundo. Chamado com <see cref="_gate"/> travado.
    /// </summary>
    private void Remember(ReadOnlySpan<byte> report, SonyHidReading? reading)
    {
        bool changed = reading is { } now && _reading is { } before
            ? now.Snapshot.Buttons != before.Snapshot.Buttons || now.Hat != before.Hat || now.Extra != before.Extra
            : reading.HasValue != _reading.HasValue;
        _reading = reading;
        _lastRaw = report.ToArray();
        long tick = Environment.TickCount64;
        if (!changed && tick - _lastHistoryAt < HistoryEveryMilliseconds)
        {
            return;
        }

        _lastHistoryAt = tick;
        _history.AddFirst(string.Create(CultureInfo.InvariantCulture, $"{(changed ? "mudou" : "parado")}: {ControllerTest.Hex(report)}"));
        while (_history.Count > HistorySize)
        {
            _history.RemoveLast();
        }
    }

    /// <summary>Tamanho do relatório de entrada e se a coleção é de controle, pelo descritor HID; nulo se o Windows não informar.</summary>
    private static (int InputLength, bool IsGamepad)? ReadCaps(SafeFileHandle handle)
    {
        if (!HidD_GetPreparsedData(handle, out IntPtr preparsed))
        {
            return null;
        }

        try
        {
            if (HidP_GetCaps(preparsed, out var caps) != HidpStatusSuccess)
            {
                return null;
            }

            // Página Generic Desktop (0x01), uso Joystick (0x04) ou Gamepad (0x05).
            return (caps.InputReportByteLength, caps.UsagePage == 0x01 && caps.Usage is 0x04 or 0x05);
        }
        finally
        {
            HidD_FreePreparsedData(preparsed);
        }
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

    [LibraryImport("hid.dll")]
    [return: MarshalAs(UnmanagedType.U1)]
    private static partial bool HidD_GetPreparsedData(SafeFileHandle device, out IntPtr preparsedData);

    [LibraryImport("hid.dll")]
    [return: MarshalAs(UnmanagedType.U1)]
    private static partial bool HidD_FreePreparsedData(IntPtr preparsedData);

    [LibraryImport("hid.dll")]
    private static partial int HidP_GetCaps(IntPtr preparsedData, out HidpCaps capabilities);

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

    /// <summary>HIDP_CAPS (64 bytes): só os campos do começo interessam; o resto é reservado e contagens.</summary>
    [StructLayout(LayoutKind.Sequential, Size = 64)]
    private struct HidpCaps
    {
        public ushort Usage;
        public ushort UsagePage;
        public ushort InputReportByteLength;
        public ushort OutputReportByteLength;
        public ushort FeatureReportByteLength;
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
