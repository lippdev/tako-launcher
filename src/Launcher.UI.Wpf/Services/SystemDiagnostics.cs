using System.Reflection;
using System.Runtime.InteropServices;
using System.Security;
using Launcher.Core.Diagnostics;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Platform;
using Launcher.UI.Wpf.Input;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.Services;

/// <summary>Estado do Tako que entra no diagnóstico (vem da Shell; o resto é lido do Windows).</summary>
public sealed record DiagnosticsContext(
    IReadOnlyCollection<Game> Games,
    string? ActiveController,
    bool SafeMode,
    string? SafeModeDetail,
    FseAssessment XboxMode,
    string? Theme);

/// <summary>
/// Coleta os fatos do "Diagnóstico do sistema" (MVP-034): Windows, monitores, controles, Steam e Xbox mode. Só lê
/// (registro, APIs de vídeo, caminhos HID); não abre dispositivos nem manda nada para a rede. O texto sai de
/// <see cref="SystemReport"/>, que remove dados pessoais. Também alimenta o cabeçalho do benchmark (MVP-035).
/// </summary>
public static partial class SystemDiagnostics
{
    private const string WindowsKey = @"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    private const string CpuKey = @"HKEY_LOCAL_MACHINE\HARDWARE\DESCRIPTION\System\CentralProcessor\0";

    /// <summary>Versão do Tako (do assembly do app, não do processo que hospeda: testes e benchmark também dão a versão certa).</summary>
    public static string TakoVersion { get; } =
        typeof(SystemDiagnostics).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion.Split('+')[0] ?? UpdateService.CurrentVersion;

    public static SystemSnapshot Collect(DiagnosticsContext context, TimeProvider time)
    {
        ArgumentNullException.ThrowIfNull(context);
        ArgumentNullException.ThrowIfNull(time);
        var windows = ReadWindows();
        var signals = FseDetector.ReadSignals();
        return new SystemSnapshot
        {
            GeneratedAt = time.GetLocalNow(),
            TakoVersion = TakoVersion,
            InstallKind = UpdateService.IsPublishedBuild ? UpdateService.InstallKind : "desenvolvimento (dotnet run / build local)",
            DataFolder = AppPaths.DataRoot,
            WindowsName = windows.Name,
            WindowsDisplayVersion = windows.DisplayVersion,
            WindowsBuild = windows.Build,
            Architecture = RuntimeInformation.OSArchitecture.ToString(),
            Runtime = RuntimeInformation.FrameworkDescription,
            Cpu = ReadCpu(),
            LogicalProcessors = Environment.ProcessorCount,
            MemoryBytes = GC.GetGCMemoryInfo().TotalAvailableMemoryBytes is > 0 and var bytes ? bytes : null,
            Displays = ReadDisplays(),
            Controllers = ReadControllers(),
            ActiveController = context.ActiveController,
            SteamFound = Safe(SteamLocator.FindRoot) is not null,
            SteamGames = context.Games.Count(g => g.Store == GameStore.Steam),
            ManualGames = context.Games.Count(g => g.Store == GameStore.Manual),
            TotalGames = context.Games.Count,
            XboxModeLabel = context.XboxMode.Label,
            XboxModeReason = context.XboxMode.Reason,
            XboxModeSignals = $"build {signals.OsBuild?.ToString(System.Globalization.CultureInfo.InvariantCulture) ?? "?"}, app Xbox: {signals.XboxAppInstalled switch { true => "sim", false => "não", null => "?" }}, DeviceForm: {signals.DeviceForm?.ToString(System.Globalization.CultureInfo.InvariantCulture) ?? "?"}",
            SafeMode = context.SafeMode,
            SafeModeDetail = context.SafeModeDetail,
            Theme = context.Theme,
        };
    }

    /// <summary>Texto pronto, sem dados pessoais (perfil vira <c>%USERPROFILE%</c>; usuário e computador viram marcadores).</summary>
    public static string Format(SystemSnapshot snapshot) =>
        SystemReport.Format(snapshot, LogRedactor.Default, Environment.UserName, Environment.MachineName);

    /// <summary>Nome, versão de marketing (24H2) e build com revisão. O registro ainda diz "Windows 10" no Windows 11.</summary>
    public static (string? Name, string? DisplayVersion, string? Build) ReadWindows()
    {
        string? name = Safe(() => Registry.GetValue(WindowsKey, "ProductName", null) as string);
        string? display = Safe(() => Registry.GetValue(WindowsKey, "DisplayVersion", null) as string);
        string? build = Safe(() => Registry.GetValue(WindowsKey, "CurrentBuild", null) as string);
        object? ubr = Safe(() => Registry.GetValue(WindowsKey, "UBR", null));
        if (build is null && OperatingSystem.IsWindows())
        {
            build = Environment.OSVersion.Version.Build.ToString(System.Globalization.CultureInfo.InvariantCulture);
        }

        if (name is not null && int.TryParse(build, out int number) && number >= 22000 && name.StartsWith("Windows 10", StringComparison.Ordinal))
        {
            name = "Windows 11" + name["Windows 10".Length..];
        }

        return (name, display, ubr is int revision && build is not null ? $"{build}.{revision}" : build);
    }

    public static string? ReadCpu() => Safe(() => (Registry.GetValue(CpuKey, "ProcessorNameString", null) as string)?.Trim());

    /// <summary>Controles conectados agora: XInput (posições 1–4) e PlayStation por HID.</summary>
    public static IReadOnlyList<string> ReadControllers()
    {
        var list = new List<string>();
        try
        {
            list.AddRange(XInputGamepad.ConnectedSlots().Select(slot => $"Xbox/XInput (posição {slot + 1})"));
        }
        catch (Exception ex) when (ex is DllNotFoundException or EntryPointNotFoundException)
        {
            list.Add("XInput indisponível neste Windows");
        }

        list.AddRange(SonyHidGamepad.ListConnected().Select(model => model switch
        {
            SonyController.DualSense => "PlayStation 5 DualSense (HID)",
            _ => "PlayStation 4 DualShock 4 (HID)",
        }));
        return list;
    }

    /// <summary>
    /// Monitores ligados à área de trabalho: resolução e Hz do modo atual (<c>EnumDisplaySettings</c>), escala efetiva
    /// do monitor (<c>GetDpiForMonitor</c>) e a placa de vídeo. Valores em pixels físicos (processo PerMonitorV2).
    /// </summary>
    public static unsafe IReadOnlyList<DisplayInfo> ReadDisplays()
    {
        var displays = new List<DisplayInfo>();
        try
        {
            var device = new DisplayDevice { Size = (uint)sizeof(DisplayDevice) };
            for (uint i = 0; EnumDisplayDevicesW(null, i, &device, 0); i++, device = new DisplayDevice { Size = (uint)sizeof(DisplayDevice) })
            {
                if ((device.StateFlags & AttachedToDesktop) == 0)
                {
                    continue;
                }

                string name = new((char*)device.DeviceName);
                string adapter = new((char*)device.DeviceString);
                var mode = new DevMode { Size = (ushort)sizeof(DevMode) };
                fixed (char* deviceName = name)
                {
                    if (!EnumDisplaySettingsW(deviceName, CurrentSettings, &mode))
                    {
                        continue;
                    }
                }

                int? scale = null;
                var center = new Point { X = mode.PositionX + ((int)mode.PelsWidth / 2), Y = mode.PositionY + ((int)mode.PelsHeight / 2) };
                IntPtr monitor = MonitorFromPoint(center, MonitorDefaultToNull);
                if (monitor != IntPtr.Zero && GetDpiForMonitor(monitor, EffectiveDpi, out uint dpiX, out _) == 0 && dpiX > 0)
                {
                    scale = (int)Math.Round(dpiX * 100d / 96);
                }

                int? hz = mode.DisplayFrequency > 1 ? (int)mode.DisplayFrequency : null; // 0 e 1 = "padrão do hardware"
                displays.Add(new DisplayInfo(name, string.IsNullOrWhiteSpace(adapter) ? null : adapter.Trim(), (int)mode.PelsWidth, (int)mode.PelsHeight, scale, hz, (device.StateFlags & PrimaryDevice) != 0));
            }
        }
        catch (Exception ex) when (ex is DllNotFoundException or EntryPointNotFoundException)
        {
            // Windows sem shcore/user32 completos (Server Core): o relatório diz "nenhum monitor lido".
        }

        return [.. displays.OrderByDescending(d => d.IsPrimary)];
    }

    private static T? Safe<T>(Func<T?> read)
    {
        try
        {
            return read();
        }
        catch (Exception ex) when (ex is SecurityException or UnauthorizedAccessException or System.IO.IOException)
        {
            return default;
        }
    }

    private const uint AttachedToDesktop = 0x1;
    private const uint PrimaryDevice = 0x4;
    private const int CurrentSettings = -1;
    private const uint MonitorDefaultToNull = 0;
    private const int EffectiveDpi = 0;

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static unsafe partial bool EnumDisplayDevicesW(char* device, uint index, DisplayDevice* displayDevice, uint flags);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static unsafe partial bool EnumDisplaySettingsW(char* deviceName, int modeNumber, DevMode* devMode);

    [LibraryImport("user32.dll")]
    private static partial IntPtr MonitorFromPoint(Point point, uint flags);

    [LibraryImport("shcore.dll")]
    private static partial int GetDpiForMonitor(IntPtr monitor, int dpiType, out uint dpiX, out uint dpiY);

    [StructLayout(LayoutKind.Sequential)]
    private struct Point
    {
        public int X;
        public int Y;
    }

    /// <summary>DISPLAY_DEVICEW (840 bytes).</summary>
    [StructLayout(LayoutKind.Sequential)]
    private unsafe struct DisplayDevice
    {
        public uint Size;
        public fixed ushort DeviceName[32];
        public fixed ushort DeviceString[128];
        public uint StateFlags;
        public fixed ushort DeviceId[128];
        public fixed ushort DeviceKey[128];
    }

    /// <summary>DEVMODEW (220 bytes) com a união no formato de monitor (posição, orientação).</summary>
    [StructLayout(LayoutKind.Sequential)]
    private unsafe struct DevMode
    {
        public fixed ushort DeviceName[32];
        public ushort SpecVersion;
        public ushort DriverVersion;
        public ushort Size;
        public ushort DriverExtra;
        public uint Fields;
        public int PositionX;
        public int PositionY;
        public uint DisplayOrientation;
        public uint DisplayFixedOutput;
        public short Color;
        public short Duplex;
        public short YResolution;
        public short TTOption;
        public short Collate;
        public fixed ushort FormName[32];
        public ushort LogPixels;
        public uint BitsPerPel;
        public uint PelsWidth;
        public uint PelsHeight;
        public uint DisplayFlags;
        public uint DisplayFrequency;
        public uint IcmMethod;
        public uint IcmIntent;
        public uint MediaType;
        public uint DitherType;
        public uint Reserved1;
        public uint Reserved2;
        public uint PanningWidth;
        public uint PanningHeight;
    }
}
