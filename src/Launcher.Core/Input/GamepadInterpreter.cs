namespace Launcher.Core.Input;

/// <summary>Comandos do launcher, independentes do dispositivo (controle, teclado, mouse).</summary>
public enum AppCommand
{
    Up,
    Down,
    Left,
    Right,
    Accept,
    Back,
    Favorite,
    Search,
    PreviousSection,
    NextSection,
    Menu,
    Guide,
}

[Flags]
public enum GamepadButtons
{
    None = 0,
    DPadUp = 1 << 0,
    DPadDown = 1 << 1,
    DPadLeft = 1 << 2,
    DPadRight = 1 << 3,
    Start = 1 << 4,
    View = 1 << 5,
    LeftShoulder = 1 << 8,
    RightShoulder = 1 << 9,
    Guide = 1 << 10,
    A = 1 << 12,
    B = 1 << 13,
    X = 1 << 14,
    Y = 1 << 15,
}

/// <summary>Leitura do controle num instante. Eixos do analógico esquerdo em -1..1 (Y positivo = para cima).</summary>
public readonly record struct GamepadSnapshot(GamepadButtons Buttons, double LeftX, double LeftY);

/// <summary>
/// Converte leituras do controle em comandos: dispara na borda de pressão e repete direções mantidas
/// (380 ms de atraso, depois a cada 110 ms), como no protótipo.
/// </summary>
public sealed class GamepadInterpreter
{
    public static readonly TimeSpan RepeatDelay = TimeSpan.FromMilliseconds(380);
    public static readonly TimeSpan RepeatInterval = TimeSpan.FromMilliseconds(110);
    private const double DeadZone = 0.5;

    private static readonly (GamepadButtons Button, AppCommand Command)[] ButtonMap =
    [
        (GamepadButtons.A, AppCommand.Accept),
        (GamepadButtons.B, AppCommand.Back),
        (GamepadButtons.X, AppCommand.Favorite),
        (GamepadButtons.Y, AppCommand.Search),
        (GamepadButtons.LeftShoulder, AppCommand.PreviousSection),
        (GamepadButtons.RightShoulder, AppCommand.NextSection),
        (GamepadButtons.Start, AppCommand.Menu),
        (GamepadButtons.Guide, AppCommand.Guide),
        // View funciona como Guia quando o sistema captura o botão Xbox.
        (GamepadButtons.View, AppCommand.Guide),
    ];

    private readonly Dictionary<AppCommand, TimeSpan> _held = [];
    private bool _suppressHeld;

    /// <summary>
    /// Controle desconectado, trocado ou janela sem foco (MVP-031): esquece o estado e ignora o que ainda estiver
    /// pressionado até ser solto. Assim uma direção mantida antes da queda não volta "andando sozinha", e um A que
    /// fechou o jogo não aciona nada ao voltar para o launcher.
    /// </summary>
    public void Reset()
    {
        _held.Clear();
        _suppressHeld = true;
    }

    public IReadOnlyList<AppCommand> Update(GamepadSnapshot snapshot, TimeSpan now)
    {
        var pressed = new HashSet<AppCommand>();
        foreach (var (button, command) in ButtonMap)
        {
            if (snapshot.Buttons.HasFlag(button))
            {
                pressed.Add(command);
            }
        }

        if (snapshot.Buttons.HasFlag(GamepadButtons.DPadUp) || snapshot.LeftY > DeadZone)
        {
            pressed.Add(AppCommand.Up);
        }

        if (snapshot.Buttons.HasFlag(GamepadButtons.DPadDown) || snapshot.LeftY < -DeadZone)
        {
            pressed.Add(AppCommand.Down);
        }

        if (snapshot.Buttons.HasFlag(GamepadButtons.DPadLeft) || snapshot.LeftX < -DeadZone)
        {
            pressed.Add(AppCommand.Left);
        }

        if (snapshot.Buttons.HasFlag(GamepadButtons.DPadRight) || snapshot.LeftX > DeadZone)
        {
            pressed.Add(AppCommand.Right);
        }

        var fired = new List<AppCommand>();
        if (_suppressHeld)
        {
            // Primeira leitura depois do Reset: o que já está pressionado só volta a valer depois de solto.
            _suppressHeld = false;
            foreach (var command in pressed)
            {
                _held[command] = TimeSpan.MaxValue;
            }

            return fired;
        }

        foreach (var command in pressed)
        {
            if (!_held.TryGetValue(command, out var next))
            {
                _held[command] = now + RepeatDelay;
                fired.Add(command);
            }
            else if (IsDirection(command) && now >= next)
            {
                _held[command] = now + RepeatInterval;
                fired.Add(command);
            }
        }

        foreach (var released in _held.Keys.Where(k => !pressed.Contains(k)).ToList())
        {
            _held.Remove(released);
        }

        return fired;
    }

    private static bool IsDirection(AppCommand command) =>
        command is AppCommand.Up or AppCommand.Down or AppCommand.Left or AppCommand.Right;
}
