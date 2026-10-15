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

    /// <summary>Abre uma janela simulada (ex.: títulos longos no teste de fumaça); entra no fim da lista.</summary>
    public void Add(OpenWindow window)
    {
        lock (_gate)
        {
            _windows.Add(window);
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

/// <summary>Contador de FPS simulado: "mede" um FPS fixo enquanto há processos do jogo.</summary>
public sealed class SimulatedFpsCounter(bool available = false, double fps = 60) : IFpsCounter
{
    private Func<IReadOnlyCollection<int>>? _game;

    public bool IsAvailable { get; } = available;

    public FpsCounterStatus Status { get; private set; } = FpsCounterStatus.Idle;

    public FrameStats? Current => Status == FpsCounterStatus.Running && _game?.Invoke() is { Count: > 0 } pids
        ? new FrameStats(fps, 1000 / fps, fps * 0.9, "DXGI", pids.First())
        : null;

    public int Starts { get; private set; }

    public bool StartCapture(Func<IReadOnlyCollection<int>> gameProcesses)
    {
        if (!IsAvailable)
        {
            Status = FpsCounterStatus.Unavailable;
            return false;
        }

        _game = gameProcesses;
        Status = FpsCounterStatus.Running;
        Starts++;
        return true;
    }

    public void StopCapture()
    {
        _game = null;
        Status = IsAvailable ? FpsCounterStatus.Idle : FpsCounterStatus.Unavailable;
    }

    public void RecoverAfterCrash()
    {
    }
}

/// <summary>Permissão do contador simulada: o "UAC" responde o que o teste mandar.</summary>
public sealed class SimulatedPerfLogAccess(PerfLogStatus permission = PerfLogStatus.NotGranted) : IPerfLogAccess
{
    public PerfLogStatus Permission { get; set; } = permission;

    /// <summary>Resposta do próximo UAC.</summary>
    public PerfLogChange NextAnswer { get; set; } = PerfLogChange.Done;

    /// <summary>Depois de conceder, o token só tem o grupo no próximo login.</summary>
    public PerfLogStatus AfterGrant { get; set; } = PerfLogStatus.NeedsSignOut;

    public int Prompts { get; private set; }

    public PerfLogStatus Check() => Permission;

    public Task<PerfLogChange> GrantAsync()
    {
        Prompts++;
        if (NextAnswer == PerfLogChange.Done)
        {
            Permission = AfterGrant;
        }

        return Task.FromResult(NextAnswer);
    }

    public Task<PerfLogChange> RevokeAsync()
    {
        Prompts++;
        if (NextAnswer == PerfLogChange.Done)
        {
            Permission = PerfLogStatus.NotGranted;
        }

        return Task.FromResult(NextAnswer);
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
