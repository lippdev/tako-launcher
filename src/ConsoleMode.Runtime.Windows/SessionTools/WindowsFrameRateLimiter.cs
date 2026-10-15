using System.Runtime.InteropServices;
using System.Security;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;
using Microsoft.Win32;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Limite de FPS pelo driver de vídeo (ADR-0018): placas pelo DXGI, preferência de GPU do Windows por exe e os backends
/// NVIDIA (NVAPI), AMD (ADLX) e Intel (IGCL), cada um carregando a biblioteca do próprio driver só quando preciso.
/// </summary>
public static partial class WindowsFrameRateLimiter
{
    public static DriverFrameRateLimiter Create(string backupFile, ILogger logger)
    {
        ArgumentNullException.ThrowIfNull(logger);
        void Log(string message) => LogDriver(logger, message);
        return new DriverFrameRateLimiter(
            () => WindowsGpuAdapters.List(Log),
            WindowsGpuAdapters.PreferenceFor,
            [new NvapiFrameRateBackend(Log), new AdlxFrameRateBackend(Log), new IgclFrameRateBackend(Log)],
            new FrameRateLimitBackupStore(backupFile),
            Log);
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Limite de FPS: {Message}")]
    private static partial void LogDriver(ILogger logger, string message);
}

/// <summary>Placas de vídeo pelo DXGI (<c>IDXGIFactory1::EnumAdapters1</c>) e a preferência de GPU do Windows por exe.</summary>
internal static unsafe partial class WindowsGpuAdapters
{
    private const uint SoftwareAdapterFlag = 2; // DXGI_ADAPTER_FLAG_SOFTWARE
    private const int NotFound = unchecked((int)0x887A0002); // DXGI_ERROR_NOT_FOUND

    // Tabelas de funções: IUnknown (0–2), IDXGIObject (3–6), IDXGIFactory (7–11), IDXGIFactory1 (12–13).
    private const int ReleaseSlot = 2;
    private const int EnumAdapters1Slot = 12;

    // IDXGIAdapter (7–9), IDXGIAdapter1::GetDesc1 (10).
    private const int GetDesc1Slot = 10;

    private static readonly Guid FactoryIid = new("770aae78-f26f-4dba-a829-253c83d1b387"); // IID_IDXGIFactory1

    public static IReadOnlyList<GpuAdapter> List(Action<string>? log = null)
    {
        nint factory = 0;
        Guid iid = FactoryIid;
        int hr;
        try
        {
            hr = CreateDXGIFactory1(&iid, &factory);
        }
        catch (DllNotFoundException)
        {
            return [];
        }

        if (hr < 0 || factory == 0)
        {
            log?.Invoke($"DXGI indisponível (0x{hr:X8})");
            return [];
        }

        var adapters = new List<GpuAdapter>();
        try
        {
            var enumAdapters = (delegate* unmanaged<nint, uint, nint*, int>)(*(nint**)factory)[EnumAdapters1Slot];
            for (uint i = 0; i < 16; i++)
            {
                nint adapter = 0;
                hr = enumAdapters(factory, i, &adapter);
                if (hr == NotFound || hr < 0 || adapter == 0)
                {
                    break;
                }

                try
                {
                    AdapterDesc1 desc;
                    if (((delegate* unmanaged<nint, AdapterDesc1*, int>)(*(nint**)adapter)[GetDesc1Slot])(adapter, &desc) >= 0)
                    {
                        string name = new string(desc.Description).TrimEnd('\0').Trim();
                        adapters.Add(new GpuAdapter(name, desc.VendorId, desc.DedicatedVideoMemory, (desc.Flags & SoftwareAdapterFlag) != 0));
                    }
                }
                finally
                {
                    Release(adapter);
                }
            }
        }
        finally
        {
            Release(factory);
        }

        log?.Invoke($"placas: {string.Join(", ", adapters)}");
        return adapters;
    }

    /// <summary>Preferência de GPU escolhida no Windows para o exe (HKCU), ou padrão.</summary>
    public static GpuPreference PreferenceFor(string? exePath)
    {
        if (string.IsNullOrWhiteSpace(exePath))
        {
            return GpuPreference.Default;
        }

        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(GpuSelection.PreferencesKey);
            return GpuSelection.ParsePreference(key?.GetValue(exePath) as string);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or SecurityException)
        {
            return GpuPreference.Default;
        }
    }

    private static void Release(nint unknown) => _ = ((delegate* unmanaged<nint, uint>)(*(nint**)unknown)[ReleaseSlot])(unknown);

    [LibraryImport("dxgi.dll")]
    private static partial int CreateDXGIFactory1(Guid* riid, nint* factory);

    /// <summary><c>DXGI_ADAPTER_DESC1</c>.</summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct AdapterDesc1
    {
        public fixed char Description[128];
        public uint VendorId;
        public uint DeviceId;
        public uint SubSysId;
        public uint Revision;
        public nuint DedicatedVideoMemory;
        public nuint DedicatedSystemMemory;
        public nuint SharedSystemMemory;
        public uint LuidLow;
        public int LuidHigh;
        public uint Flags;
    }
}
