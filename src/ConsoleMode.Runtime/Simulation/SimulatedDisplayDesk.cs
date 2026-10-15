using ConsoleMode.Runtime.Display;

namespace ConsoleMode.Runtime.Simulation;

/// <summary>
/// Mesa de monitores em memória (ativos e desligados), sem tocar no sistema: testes, teste de fumaça e CI. Atende ao
/// controle da "Tela de jogo" e à leitura/restauração do ADR-0013 sobre o mesmo estado; registra cada chamada em
/// <see cref="Calls"/>.
/// </summary>
public sealed class SimulatedDisplayDesk : IDisplayControlService, IDisplayConfigurationService
{
    private readonly object _gate = new();
    private readonly List<Screen> _screens;

    public SimulatedDisplayDesk(IEnumerable<SimulatedScreen>? screens = null)
    {
        _screens = [.. (screens ?? Default).Select(s => new Screen(s))];
    }

    /// <summary>Monitor principal 2560×1440 ligado e uma TV 4K com HDR desligada.</summary>
    public static IReadOnlyList<SimulatedScreen> Default { get; } =
    [
        new("SIM\\DESK", "Monitor da mesa", IsActive: true, new ScreenBounds(0, 0, 2560, 1440), 144, HdrStatus.Unsupported, [new(2560, 1440, 144), new(1920, 1080, 60)]),
        new("SIM\\TV", "TV da sala", IsActive: false, new ScreenBounds(0, 0, 3840, 2160), 60, HdrStatus.Off, [new(3840, 2160, 120), new(3840, 2160, 60), new(1920, 1080, 60)]),
    ];

    /// <summary>Monitores que não ligam quando pedido (TV que não acorda).</summary>
    public ISet<string> NeverWake { get; } = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

    /// <summary>Próximas chamadas falham como falharia a plataforma.</summary>
    public bool FailNext { get; set; }

    /// <summary>Alterações feitas, na ordem (ex.: "enable SIM\TV", "primary SIM\TV").</summary>
    public IList<string> Calls { get; } = [];

    public IReadOnlyList<ConnectedDisplay> ListDisplays()
    {
        lock (_gate)
        {
            ThrowIfFailing();
            return [.. _screens.Select(s => s.ToConnected())];
        }
    }

    public IReadOnlyList<DisplayMode> ListModes(string displayId)
    {
        lock (_gate)
        {
            ThrowIfFailing();
            return Find(displayId) is { IsActive: true } s ? DisplayMode.Normalize(s.Modes) : [];
        }
    }

    public void Enable(IReadOnlyCollection<string> displayIds)
    {
        ArgumentNullException.ThrowIfNull(displayIds);
        lock (_gate)
        {
            ThrowIfFailing();
            foreach (string id in displayIds)
            {
                var screen = Require(id);
                Calls.Add($"enable {id}");
                if (screen.IsActive || NeverWake.Contains(id))
                {
                    continue;
                }

                int right = _screens.Where(s => s.IsActive).Select(s => s.X + s.Width).DefaultIfEmpty(0).Max();
                screen.IsActive = true;
                screen.X = right;
                screen.Y = 0;
            }
        }
    }

    public void Disable(IReadOnlyCollection<string> displayIds)
    {
        ArgumentNullException.ThrowIfNull(displayIds);
        lock (_gate)
        {
            ThrowIfFailing();
            if (_screens.Where(s => s.IsActive).All(s => displayIds.Contains(s.Id, StringComparer.OrdinalIgnoreCase)))
            {
                throw new RuntimeEnvironmentException("Desconectar todas as telas deixaria o PC sem imagem");
            }

            foreach (string id in displayIds)
            {
                Require(id).IsActive = false;
                Calls.Add($"disable {id}");
            }

            EnsurePrimary();
        }
    }

    public void SetPrimary(string displayId)
    {
        lock (_gate)
        {
            ThrowIfFailing();
            var target = Require(displayId);
            if (!target.IsActive)
            {
                throw new RuntimeEnvironmentException($"Tela desligada não pode ser a principal: {displayId}");
            }

            int dx = target.X;
            int dy = target.Y;
            foreach (var s in _screens.Where(s => s.IsActive))
            {
                s.X -= dx;
                s.Y -= dy;
            }

            Calls.Add($"primary {displayId}");
        }
    }

    public void SetMode(string displayId, DisplayMode mode)
    {
        ArgumentNullException.ThrowIfNull(mode);
        lock (_gate)
        {
            ThrowIfFailing();
            var s = Require(displayId);
            if (!s.IsActive || !s.Modes.Contains(mode))
            {
                throw new RuntimeEnvironmentException($"Modo não aceito: {mode} em {displayId}");
            }

            (s.Width, s.Height, s.Hz) = (mode.Width, mode.Height, mode.RefreshHz);
            Calls.Add($"mode {displayId} {mode.Key}");
        }
    }

    public void SetHdr(string displayId, bool enabled)
    {
        lock (_gate)
        {
            ThrowIfFailing();
            var s = Require(displayId);
            if (!s.IsActive || s.Hdr is HdrStatus.Unsupported)
            {
                throw new RuntimeEnvironmentException($"HDR indisponível em {displayId}");
            }

            s.Hdr = enabled ? HdrStatus.On : HdrStatus.Off;
            Calls.Add($"hdr {displayId} {(enabled ? "on" : "off")}");
        }
    }

    public DisplayTopology Capture()
    {
        lock (_gate)
        {
            ThrowIfFailing();
            return new DisplayTopology([.. _screens.Where(s => s.IsActive).Select(s => s.ToMonitorState())]);
        }
    }

    public void Apply(DisplayTopology target, IReadOnlyCollection<string> changedMonitorIds)
    {
        ArgumentNullException.ThrowIfNull(target);
        lock (_gate)
        {
            ThrowIfFailing();
            foreach (var s in _screens)
            {
                var wanted = target.Find(s.Id);
                s.IsActive = wanted is not null;
                if (wanted is not null)
                {
                    (s.X, s.Y, s.Width, s.Height) = (wanted.X, wanted.Y, wanted.Width, wanted.Height);
                    s.Hz = (int)Math.Round(wanted.RefreshRate.Hz);
                }
            }

            Calls.Add("restore");
        }
    }

    private Screen? Find(string id) => _screens.FirstOrDefault(s => string.Equals(s.Id, id, StringComparison.OrdinalIgnoreCase));

    private Screen Require(string id) => Find(id) ?? throw new RuntimeEnvironmentException($"Tela não conectada: {id}");

    private void EnsurePrimary()
    {
        var active = _screens.Where(s => s.IsActive).ToList();
        if (active.Count > 0 && !active.Any(s => s.X == 0 && s.Y == 0))
        {
            var first = active[0];
            int dx = first.X;
            int dy = first.Y;
            active.ForEach(s => (s.X, s.Y) = (s.X - dx, s.Y - dy));
        }
    }

    private void ThrowIfFailing()
    {
        if (FailNext)
        {
            throw new RuntimeEnvironmentException("Falha simulada de monitores");
        }
    }

    private sealed class Screen(SimulatedScreen spec)
    {
        public string Id { get; } = spec.Id;

        public string Name { get; } = spec.Name;

        public IReadOnlyList<DisplayMode> Modes { get; } = spec.Modes;

        public bool IsActive { get; set; } = spec.IsActive;

        public int X { get; set; } = spec.Bounds.X;

        public int Y { get; set; } = spec.Bounds.Y;

        public int Width { get; set; } = spec.Bounds.Width;

        public int Height { get; set; } = spec.Bounds.Height;

        public int Hz { get; set; } = spec.RefreshHz;

        public HdrStatus Hdr { get; set; } = spec.Hdr;

        public ConnectedDisplay ToConnected() => IsActive
            ? new ConnectedDisplay(Id, Name, true, X == 0 && Y == 0, new ScreenBounds(X, Y, Width, Height), new DisplayMode(Width, Height, Hz), Hdr)
            : new ConnectedDisplay(Id, Name, false, false, null, null, Hdr == HdrStatus.Unsupported ? HdrStatus.Unsupported : HdrStatus.Unknown);

        public MonitorState ToMonitorState() =>
            new(Id, Name, X == 0 && Y == 0, X, Y, Width, Height, RefreshRate.FromHz((uint)Hz), DisplayRotation.None);
    }
}

/// <summary>Monitor da mesa simulada.</summary>
public sealed record SimulatedScreen(
    string Id,
    string Name,
    bool IsActive,
    ScreenBounds Bounds,
    int RefreshHz,
    HdrStatus Hdr,
    IReadOnlyList<DisplayMode> Modes);

/// <summary>Cortina preta em memória: guarda as telas cobertas.</summary>
public sealed class SimulatedScreenCurtain : IScreenCurtain
{
    public IReadOnlyList<ScreenBounds> Covered { get; private set; } = [];

    public void Show(IReadOnlyList<ScreenBounds> screens) => Covered = [.. screens];

    public void Close() => Covered = [];
}

/// <summary>Atalho de emergência em memória: <see cref="PressAsync"/> faz o papel do teclado.</summary>
public sealed class SimulatedEmergencyHotkey : IEmergencyHotkey
{
    private Action? _pressed;

    public string Keys => "Ctrl+Alt+Shift+D";

    public bool IsStarted => _pressed is not null;

    public bool Start(Action pressed)
    {
        _pressed = pressed;
        return true;
    }

    /// <summary>Aperta o atalho numa thread que não é a do chamador, como o Windows faria.</summary>
    public Task PressAsync() => Task.Run(() => _pressed?.Invoke());

    public void Dispose() => _pressed = null;
}
