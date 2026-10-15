using ConsoleMode.Runtime.SessionTools;

namespace ConsoleMode.Runtime.Simulation;

/// <summary>Janelas em memória para testes e o teste de fumaça da UI: nada no Windows é ativado ou fechado.</summary>
public sealed class SimulatedWindowSwitcher : IWindowSwitcher
{
    private readonly object _gate = new();
    private readonly List<OpenWindow> _windows =
    [
        new(101, "Discord", "Discord", null),
        new(102, "Spotify Premium", "Spotify", null),
        new(103, "Steam", "Steam", null),
    ];

    public nint LastActivated { get; private set; }

    public IReadOnlyList<OpenWindow> List()
    {
        lock (_gate)
        {
            return [.. _windows];
        }
    }

    public IconBitmap? LoadIcon(OpenWindow window, int size) => null;

    public bool Activate(nint handle)
    {
        lock (_gate)
        {
            LastActivated = handle;
            return _windows.Any(w => w.Handle == handle);
        }
    }

    public bool RequestClose(nint handle)
    {
        lock (_gate)
        {
            return _windows.RemoveAll(w => w.Handle == handle) > 0;
        }
    }
}

/// <summary>RTSS simulado: instalado ou não, guarda o último estilo e limite pedidos.</summary>
public sealed class SimulatedFpsOverlay(bool installed = false) : IFpsOverlayService
{
    public bool IsInstalled { get; } = installed;

    public bool CanLimitFps => IsInstalled;

    public string? Style { get; private set; }

    public int Limit { get; private set; }

    public bool ApplyOverlay(string style, FpsOverlayLayout layout)
    {
        if (!IsInstalled)
        {
            return false;
        }

        Style = style;
        return true;
    }

    public bool SetFpsLimit(int fps)
    {
        if (!IsInstalled)
        {
            return false;
        }

        Limit = fps;
        return true;
    }

    public void Restore()
    {
        Style = null;
        Limit = 0;
    }
}

/// <summary>Ponteiro simulado: só guarda o estado.</summary>
public sealed class SimulatedPointer : IPointerService
{
    public bool IsCursorHidden { get; private set; }

    public bool IsControllerMouseOn { get; private set; }

    public bool IsControllerMousePaused { get; set; }

    public void SetCursorHidden(bool hidden) => IsCursorHidden = hidden;

    public void SetControllerMouse(bool enabled, int screenHeight) => IsControllerMouseOn = enabled;

    public void Reset()
    {
        IsCursorHidden = false;
        IsControllerMouseOn = false;
        IsControllerMousePaused = false;
    }

    public void RecoverAfterCrash()
    {
    }
}

/// <summary>ControlFS simulado (instalado ou não); conta as aberturas.</summary>
public sealed class SimulatedControlFs(bool installed = false) : IControlFsService
{
    public bool IsInstalled { get; } = installed;

    public int Opened { get; private set; }

    public bool Open()
    {
        if (!IsInstalled)
        {
            return false;
        }

        Opened++;
        return true;
    }
}

/// <summary>Leituras fixas de um PC de jogos médio.</summary>
public sealed class SimulatedHardwareMonitor : IHardwareMonitor
{
    public HardwareSample Sample() => new()
    {
        CpuUsage = 38,
        CpuClockMhz = 4200,
        GpuUsage = 92,
        GpuClockMhz = 1900,
        GpuTempC = 67,
        VramUsedMb = 5300,
        VramTotalMb = 8192,
        RamUsedMb = 9800,
        RamTotalMb = 16384,
    };
}
