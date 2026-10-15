// Adaptado de lippdev/consolemode: src/ConsoleMode/Native/NativeDisplays.cs, src/ConsoleMode/Native/CcdHelper.cs e src/ConsoleMode/Services/DisplayBackend.cs
using ConsoleMode.Runtime.Display;
using static ConsoleMode.Runtime.Windows.DisplayNative;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Comandos de monitores da "Tela de jogo" pela API do próprio Windows, como no Console Mode: CCD
/// (<c>QueryDisplayConfig</c>/<c>SetDisplayConfig</c>) para ligar, desconectar, principal e HDR, e
/// <c>ChangeDisplaySettingsEx</c> para resolução e taxa. Os monitores são identificados pelo caminho do dispositivo, o
/// mesmo <see cref="MonitorState.Id"/> que <see cref="WindowsDisplayConfiguration"/> grava.
/// </summary>
public sealed unsafe class WindowsDisplayControl : IDisplayControlService
{
    // Alvo (adaptador + id) → fonte que ele tinha enquanto ativo, para religar com o mesmo \\.\DISPLAYn.
    private static readonly Dictionary<(long Adapter, uint Target), uint> LastSource = [];
    private static readonly object Gate = new();

    public IReadOnlyList<ConnectedDisplay> ListDisplays() =>
        [.. Connected().Select(m => m.IsActive
            ? new ConnectedDisplay(m.Id, m.Name, true, m.Bounds is { X: 0, Y: 0 }, m.Bounds, m.Mode, Ccd.GetHdr(m.Adapter, m.TargetId))
            : new ConnectedDisplay(m.Id, m.Name, false, false, null, null, HdrStatus.Unknown))];

    public IReadOnlyList<DisplayMode> ListModes(string displayId)
    {
        if (Connected().FirstOrDefault(m => m.Is(displayId)) is not { IsActive: true, GdiName: { } gdi })
        {
            return [];
        }

        var modes = new List<DisplayMode>();
        DevMode mode = default;
        mode.Size = (ushort)sizeof(DevMode);
        for (int i = 0; EnumDisplaySettingsEx(gdi, i, &mode, 0); i++)
        {
            // Taxa 0 ou 1 é "padrão do hardware": não serve para escolher.
            if (mode.DisplayFrequency > 1 && mode.BitsPerPel is 0 or >= 24)
            {
                modes.Add(new DisplayMode((int)mode.PelsWidth, (int)mode.PelsHeight, (int)mode.DisplayFrequency));
            }
        }

        return DisplayMode.Normalize(modes);
    }

    public void Enable(IReadOnlyCollection<string> displayIds)
    {
        ArgumentNullException.ThrowIfNull(displayIds);
        var wanted = Connected().Where(m => !m.IsActive && displayIds.Contains(m.Id, StringComparer.OrdinalIgnoreCase)).ToList();
        if (wanted.Count == 0)
        {
            return;
        }

        var (paths, modes) = Ccd.Query(QdcAllPaths);
        var selected = paths.Where(p => (p.Flags & PathActive) != 0).ToList();
        foreach (var m in wanted)
        {
            int index = Array.FindIndex(paths, p =>
                p.TargetInfo.TargetAvailable != 0 && p.TargetInfo.Id == m.TargetId && p.SourceInfo.Id == m.SourceId
                && Ccd.Key(p.TargetInfo.AdapterId) == Ccd.Key(m.Adapter));
            if (index < 0)
            {
                continue;
            }

            var path = paths[index];
            path.Flags |= PathActive;
            path.SourceInfo.ModeInfoIdx = PathModeIdxInvalid;
            path.TargetInfo.ModeInfoIdx = PathModeIdxInvalid;
            selected.Add(path);
        }

        int code = Ccd.Set([.. selected], modes);
        if (code != ErrorSuccess)
        {
            // Último recurso do Console Mode (EnsureActive): estender a área de trabalho para todos os conectados.
            int extend = Ccd.ExtendAll();
            if (extend != ErrorSuccess)
            {
                throw new RuntimeEnvironmentException($"SetDisplayConfig não ligou {string.Join(", ", wanted.Select(w => w.Name))} ({code}; estender: {extend})");
            }
        }
    }

    public void Disable(IReadOnlyCollection<string> displayIds)
    {
        ArgumentNullException.ThrowIfNull(displayIds);
        var (paths, modes) = Ccd.Query(QdcOnlyActivePaths);
        int remaining = 0;
        bool changed = false;
        for (int i = 0; i < paths.Length; i++)
        {
            if (displayIds.Contains(IdOf(paths[i]), StringComparer.OrdinalIgnoreCase))
            {
                paths[i].Flags = 0;
                changed = true;
            }
            else
            {
                remaining++;
            }
        }

        if (!changed)
        {
            return;
        }

        if (remaining == 0)
        {
            throw new RuntimeEnvironmentException("Desconectar todas as telas deixaria o PC sem imagem");
        }

        int code = Ccd.Set(paths, modes);
        if (code != ErrorSuccess)
        {
            throw new RuntimeEnvironmentException($"SetDisplayConfig não desconectou as telas ({code})");
        }
    }

    public void SetPrimary(string displayId)
    {
        var (paths, modes) = Ccd.Query(QdcOnlyActivePaths);
        int pathIndex = Array.FindIndex(paths, p => string.Equals(IdOf(p), displayId, StringComparison.OrdinalIgnoreCase));
        if (pathIndex < 0 || paths[pathIndex].SourceInfo.ModeInfoIdx >= modes.Length)
        {
            throw new RuntimeEnvironmentException($"Tela desligada não pode ser a principal: {displayId}");
        }

        var origin = modes[paths[pathIndex].SourceInfo.ModeInfoIdx].SourceMode;
        if (origin.X == 0 && origin.Y == 0)
        {
            return;
        }

        // A principal é a que está em (0,0): desloca todas as fontes para a escolhida cair na origem.
        for (int i = 0; i < modes.Length; i++)
        {
            if (modes[i].InfoType == ModeInfoTypeSource)
            {
                modes[i].SourceMode.X -= origin.X;
                modes[i].SourceMode.Y -= origin.Y;
            }
        }

        int code = Ccd.Set(paths, modes);
        if (code != ErrorSuccess)
        {
            throw new RuntimeEnvironmentException($"SetDisplayConfig não trocou a tela principal ({code})");
        }
    }

    public void SetMode(string displayId, DisplayMode mode)
    {
        ArgumentNullException.ThrowIfNull(mode);
        if (Connected().FirstOrDefault(m => m.Is(displayId)) is not { IsActive: true, GdiName: { } gdi, Bounds: { } bounds })
        {
            throw new RuntimeEnvironmentException($"Tela desligada: {displayId}");
        }

        DevMode dm = default;
        dm.Size = (ushort)sizeof(DevMode);
        if (!EnumDisplaySettingsEx(gdi, EnumCurrentSettings, &dm, 0))
        {
            throw new RuntimeEnvironmentException($"EnumDisplaySettingsEx falhou para {gdi}");
        }

        dm.PelsWidth = (uint)mode.Width;
        dm.PelsHeight = (uint)mode.Height;
        dm.DisplayFrequency = (uint)mode.RefreshHz;
        dm.PositionX = bounds.X;
        dm.PositionY = bounds.Y;
        dm.Fields = DmPelsWidth | DmPelsHeight | DmDisplayFrequency | DmPosition;
        int queued = ChangeDisplaySettingsEx(gdi, &dm, 0, CdsUpdateRegistry | CdsNoReset, 0);
        int applied = queued is DispChangeSuccessful or DispChangeRestart ? ChangeDisplaySettingsEx(null, null, 0, 0, 0) : queued;
        if (applied is not (DispChangeSuccessful or DispChangeRestart))
        {
            throw new RuntimeEnvironmentException($"ChangeDisplaySettingsEx recusou {mode} em {gdi} ({applied})");
        }

        if (bounds is { X: 0, Y: 0 })
        {
            SetPrimary(displayId);
        }
    }

    public void SetHdr(string displayId, bool enabled)
    {
        if (Connected().FirstOrDefault(m => m.Is(displayId)) is not { IsActive: true } monitor)
        {
            throw new RuntimeEnvironmentException($"Tela desligada: {displayId}");
        }

        int code = Ccd.SetHdr(monitor.Adapter, monitor.TargetId, enabled);
        if (code != ErrorSuccess)
        {
            throw new RuntimeEnvironmentException($"HDR {(enabled ? "ligar" : "desligar")} falhou em {monitor.Name} ({code})");
        }
    }

    /// <summary>
    /// Monitores conectados e a fonte de vídeo de cada um (a atual, ou a que ele usaria se fosse ligado), como o
    /// <c>NativeDisplays.ListMonitors</c> do Console Mode.
    /// </summary>
    private static List<Monitor> Connected()
    {
        var (paths, modes) = Ccd.Query(QdcAllPaths);
        var targets = new List<(long Adapter, uint Target, Luid AdapterId)>();
        var candidates = new Dictionary<(long, uint), List<uint>>();
        var active = new Dictionary<(long, uint), int>();
        for (int i = 0; i < paths.Length; i++)
        {
            ref var p = ref paths[i];
            if (p.TargetInfo.TargetAvailable == 0)
            {
                continue;
            }

            var key = (Ccd.Key(p.TargetInfo.AdapterId), p.TargetInfo.Id);
            if (!candidates.TryGetValue(key, out var list))
            {
                candidates[key] = list = [];
                targets.Add((key.Item1, key.Item2, p.TargetInfo.AdapterId));
            }

            // As fontes são por adaptador; a deste caminho fica no mesmo adaptador do alvo.
            if (!list.Contains(p.SourceInfo.Id))
            {
                list.Add(p.SourceInfo.Id);
            }

            if ((p.Flags & PathActive) != 0)
            {
                active.TryAdd(key, i);
            }
        }

        Dictionary<(long, uint), (long Adapter, uint Source)> assigned;
        lock (Gate)
        {
            foreach (var (key, index) in active)
            {
                LastSource[key] = paths[index].SourceInfo.Id;
            }

            // A fonte é identificada também pelo adaptador: fontes de adaptadores diferentes nunca colidem.
            assigned = DisplayIdentity.AssignSources<(long, uint), (long Adapter, uint Source)>(targets.Select(t =>
            {
                var key = (t.Adapter, t.Target);
                (long, uint)? activeSource = active.TryGetValue(key, out int idx) ? (t.Adapter, paths[idx].SourceInfo.Id) : null;
                (long, uint)? preferred = LastSource.TryGetValue(key, out uint last) ? (t.Adapter, last) : null;
                IReadOnlyList<(long, uint)> list = [.. candidates[key].Select(s => (t.Adapter, s))];
                return (key, activeSource, preferred, list);
            }));
        }

        var result = new List<Monitor>();
        foreach (var t in targets)
        {
            var key = (t.Adapter, t.Target);
            if (!assigned.TryGetValue(key, out var source))
            {
                continue;
            }

            var (friendly, devicePath) = Ccd.TargetName(t.AdapterId, t.Target);
            string id = Ccd.MonitorId(t.AdapterId, t.Target, devicePath);
            string? gdi = Ccd.SourceGdiName(t.AdapterId, source.Source);
            ScreenBounds? bounds = null;
            DisplayMode? mode = null;
            bool isActive = active.TryGetValue(key, out int pathIndex);
            if (isActive)
            {
                uint modeIdx = paths[pathIndex].SourceInfo.ModeInfoIdx;
                if (modeIdx < modes.Length && modes[modeIdx].InfoType == ModeInfoTypeSource)
                {
                    var sm = modes[modeIdx].SourceMode;
                    bounds = new ScreenBounds(sm.X, sm.Y, (int)sm.Width, (int)sm.Height);
                    var rate = paths[pathIndex].TargetInfo.RefreshRate;
                    int hz = rate.Denominator == 0 ? 0 : (int)Math.Round((double)rate.Numerator / rate.Denominator);
                    mode = new DisplayMode((int)sm.Width, (int)sm.Height, hz);
                }
            }

            if (result.Any(m => m.Is(id)))
            {
                continue; // tela clonada: um monitor só
            }

            result.Add(new Monitor(id, friendly ?? gdi ?? id, gdi, t.AdapterId, source.Source, t.Target, isActive, bounds, mode));
        }

        return result;
    }

    private static string IdOf(PathInfo path) =>
        Ccd.MonitorId(path.TargetInfo.AdapterId, path.TargetInfo.Id, Ccd.TargetName(path.TargetInfo.AdapterId, path.TargetInfo.Id).DevicePath);

    private sealed record Monitor(
        string Id,
        string Name,
        string? GdiName,
        Luid Adapter,
        uint SourceId,
        uint TargetId,
        bool IsActive,
        ScreenBounds? Bounds,
        DisplayMode? Mode)
    {
        public bool Is(string id) => string.Equals(Id, id, StringComparison.OrdinalIgnoreCase);
    }
}
