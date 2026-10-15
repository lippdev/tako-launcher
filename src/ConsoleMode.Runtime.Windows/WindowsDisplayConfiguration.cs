using System.Buffers.Binary;
using System.Globalization;
using System.Runtime.InteropServices;
using ConsoleMode.Runtime.Display;
using static ConsoleMode.Runtime.Windows.DisplayNative;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Monitores no Windows pela API CCD. A leitura guarda, além dos campos descritivos, a configuração ativa completa
/// (caminhos e modos do <c>QueryDisplayConfig</c>) em <see cref="DisplayTopology.PlatformData"/>; a restauração devolve
/// essa configuração ao <c>SetDisplayConfig</c> (topologia, principal, resolução, taxa, rotação e posição de uma vez).
/// Se isso falhar (ex.: depois de reiniciar o PC, quando os identificadores dos adaptadores mudam), restaura monitor a
/// monitor pelo GDI os monitores alterados que ainda estão ativos.
/// </summary>
public sealed unsafe class WindowsDisplayConfiguration : IDisplayConfigurationService
{
    private const string PlatformPrefix = "ccd1:";

    public DisplayTopology Capture()
    {
        var (paths, modes) = Query();
        var monitors = new List<MonitorState>();
        foreach (var path in paths)
        {
            if ((path.Flags & PathActive) == 0 || path.SourceInfo.ModeInfoIdx == PathModeIdxInvalid || path.SourceInfo.ModeInfoIdx >= modes.Length)
            {
                continue;
            }

            var mode = modes[path.SourceInfo.ModeInfoIdx];
            if (mode.InfoType != ModeInfoTypeSource)
            {
                continue;
            }

            var target = TargetName(path.TargetInfo.AdapterId, path.TargetInfo.Id);
            string id = target.DevicePath ?? string.Create(CultureInfo.InvariantCulture, $"{DescribeLuid(path.TargetInfo.AdapterId)}:{path.TargetInfo.Id}");
            if (monitors.Any(m => string.Equals(m.Id, id, StringComparison.OrdinalIgnoreCase)))
            {
                continue;
            }

            var source = mode.SourceMode;
            monitors.Add(new MonitorState(
                id,
                target.FriendlyName ?? SourceGdiName(path.SourceInfo.AdapterId, path.SourceInfo.Id) ?? id,
                IsPrimary: source.X == 0 && source.Y == 0,
                source.X,
                source.Y,
                (int)source.Width,
                (int)source.Height,
                new RefreshRate(path.TargetInfo.RefreshRate.Numerator, path.TargetInfo.RefreshRate.Denominator),
                FromCcdRotation(path.TargetInfo.Rotation)));
        }

        return new DisplayTopology(monitors, Encode(paths, modes));
    }

    public void Apply(DisplayTopology target, IReadOnlyCollection<string> changedMonitorIds)
    {
        ArgumentNullException.ThrowIfNull(target);
        ArgumentNullException.ThrowIfNull(changedMonitorIds);
        int ccdError = ErrorSuccess;
        if (Decode(target.PlatformData) is (var paths, var modes))
        {
            fixed (PathInfo* p = paths)
            fixed (ModeInfo* m = modes)
            {
                ccdError = SetDisplayConfig((uint)paths.Length, p, (uint)modes.Length, m, SdcApply | SdcUseSuppliedDisplayConfig | SdcSaveToDatabase | SdcAllowChanges);
            }

            if (ccdError == ErrorSuccess)
            {
                return;
            }
        }

        try
        {
            ApplyPerMonitor(target, changedMonitorIds);
        }
        catch (RuntimeEnvironmentException ex) when (ccdError != ErrorSuccess)
        {
            throw new RuntimeEnvironmentException($"SetDisplayConfig falhou ({ccdError}); restauração monitor a monitor: {ex.Message}", ex);
        }
    }

    /// <summary>Caminho GDI: resolução, taxa, rotação, posição e principal dos monitores alterados que estão ativos.</summary>
    private static void ApplyPerMonitor(DisplayTopology target, IReadOnlyCollection<string> changedMonitorIds)
    {
        var gdiNames = CurrentGdiNames();
        var inactive = new List<string>();
        int pending = 0;
        foreach (var monitor in target.Monitors.Where(m => changedMonitorIds.Contains(m.Id, StringComparer.OrdinalIgnoreCase)))
        {
            if (!gdiNames.TryGetValue(monitor.Id, out string? gdi))
            {
                inactive.Add(monitor.Name);
                continue;
            }

            DevMode mode = default;
            mode.Size = (ushort)sizeof(DevMode);
            if (!EnumDisplaySettingsEx(gdi, EnumCurrentSettings, &mode, 0))
            {
                throw new RuntimeEnvironmentException($"EnumDisplaySettingsEx falhou para {monitor.Name}");
            }

            mode.PelsWidth = (uint)monitor.Width;
            mode.PelsHeight = (uint)monitor.Height;
            // O GDI usa Hz inteiros, arredondados para baixo (59,94 Hz aparece como 59).
            mode.DisplayFrequency = (uint)Math.Floor(monitor.RefreshRate.Hz + 0.01);
            mode.PositionX = monitor.X;
            mode.PositionY = monitor.Y;
            mode.DisplayOrientation = monitor.Rotation switch
            {
                DisplayRotation.Rotate90 => 1u,
                DisplayRotation.Rotate180 => 2u,
                DisplayRotation.Rotate270 => 3u,
                _ => 0u,
            };
            mode.Fields = DmPelsWidth | DmPelsHeight | DmDisplayFrequency | DmPosition | DmDisplayOrientation;
            uint flags = CdsUpdateRegistry | CdsNoReset | (monitor.IsPrimary ? CdsSetPrimary : 0);
            int result = ChangeDisplaySettingsEx(gdi, &mode, 0, flags, 0);
            if (result is not (DispChangeSuccessful or DispChangeRestart))
            {
                throw new RuntimeEnvironmentException($"ChangeDisplaySettingsEx recusou {monitor} ({result})");
            }

            pending++;
        }

        if (pending > 0)
        {
            int applied = ChangeDisplaySettingsEx(null, null, 0, 0, 0);
            if (applied is not (DispChangeSuccessful or DispChangeRestart))
            {
                throw new RuntimeEnvironmentException($"ChangeDisplaySettingsEx não aplicou as mudanças ({applied})");
            }
        }

        if (inactive.Count > 0)
        {
            throw new RuntimeEnvironmentException($"Monitor(es) inativo(s) não podem ser religados sem a configuração original: {string.Join(", ", inactive)}");
        }
    }

    private static Dictionary<string, string> CurrentGdiNames()
    {
        var (paths, _) = Query();
        var names = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var path in paths.Where(p => (p.Flags & PathActive) != 0))
        {
            var target = TargetName(path.TargetInfo.AdapterId, path.TargetInfo.Id);
            if (target.DevicePath is { } id && SourceGdiName(path.SourceInfo.AdapterId, path.SourceInfo.Id) is { } gdi)
            {
                names.TryAdd(id, gdi);
            }
        }

        return names;
    }

    private static (PathInfo[] Paths, ModeInfo[] Modes) Query()
    {
        // A configuração pode mudar entre a consulta do tamanho e a leitura: tenta de novo algumas vezes.
        for (int attempt = 0; attempt < 5; attempt++)
        {
            uint pathCount;
            uint modeCount;
            int sizes = GetDisplayConfigBufferSizes(QdcOnlyActivePaths, &pathCount, &modeCount);
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
                result = QueryDisplayConfig(QdcOnlyActivePaths, &pathCount, p, &modeCount, m, 0);
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

    private static (string? FriendlyName, string? DevicePath) TargetName(Luid adapter, uint targetId)
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

    private static string? SourceGdiName(Luid adapter, uint sourceId)
    {
        SourceDeviceName request = default;
        request.Header.Type = DeviceInfoGetSourceName;
        request.Header.Size = (uint)sizeof(SourceDeviceName);
        request.Header.AdapterId = adapter;
        request.Header.Id = sourceId;
        return DisplayConfigGetDeviceInfo(&request.Header) == ErrorSuccess ? NullIfEmpty(new string(request.ViewGdiDeviceName, 0, 32)) : null;
    }

    private static string? NullIfEmpty(string text)
    {
        int end = text.IndexOf('\0', StringComparison.Ordinal);
        string value = end >= 0 ? text[..end] : text;
        return value.Length == 0 ? null : value;
    }

    private static DisplayRotation FromCcdRotation(uint rotation) => rotation switch
    {
        Rotation90 => DisplayRotation.Rotate90,
        Rotation180 => DisplayRotation.Rotate180,
        Rotation270 => DisplayRotation.Rotate270,
        _ => DisplayRotation.None,
    };

    // Formato opaco: "ccd1:" + base64(contagem de caminhos, contagem de modos, caminhos, modos), little-endian.
    private static string Encode(PathInfo[] paths, ModeInfo[] modes)
    {
        var pathBytes = MemoryMarshal.AsBytes(paths.AsSpan());
        var modeBytes = MemoryMarshal.AsBytes(modes.AsSpan());
        var buffer = new byte[8 + pathBytes.Length + modeBytes.Length];
        BinaryPrimitives.WriteInt32LittleEndian(buffer, paths.Length);
        BinaryPrimitives.WriteInt32LittleEndian(buffer.AsSpan(4), modes.Length);
        pathBytes.CopyTo(buffer.AsSpan(8));
        modeBytes.CopyTo(buffer.AsSpan(8 + pathBytes.Length));
        return PlatformPrefix + Convert.ToBase64String(buffer);
    }

    private static (PathInfo[] Paths, ModeInfo[] Modes)? Decode(string? data)
    {
        if (data is null || !data.StartsWith(PlatformPrefix, StringComparison.Ordinal))
        {
            return null;
        }

        byte[] buffer;
        try
        {
            buffer = Convert.FromBase64String(data[PlatformPrefix.Length..]);
        }
        catch (FormatException)
        {
            return null;
        }

        if (buffer.Length < 8)
        {
            return null;
        }

        int pathCount = BinaryPrimitives.ReadInt32LittleEndian(buffer);
        int modeCount = BinaryPrimitives.ReadInt32LittleEndian(buffer.AsSpan(4));
        if (pathCount <= 0 || modeCount < 0 || pathCount > 64 || modeCount > 256
            || buffer.Length != 8 + (pathCount * sizeof(PathInfo)) + (modeCount * sizeof(ModeInfo)))
        {
            return null;
        }

        var paths = MemoryMarshal.Cast<byte, PathInfo>(buffer.AsSpan(8, pathCount * sizeof(PathInfo))).ToArray();
        var modes = MemoryMarshal.Cast<byte, ModeInfo>(buffer.AsSpan(8 + (pathCount * sizeof(PathInfo)))).ToArray();
        return (paths, modes);
    }

    private static string DescribeLuid(Luid luid) => string.Create(CultureInfo.InvariantCulture, $"{luid.HighPart:X8}{luid.LowPart:X8}");
}
