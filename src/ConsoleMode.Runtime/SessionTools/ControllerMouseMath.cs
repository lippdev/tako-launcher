// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/ControllerMouseMath.cs
namespace ConsoleMode.Runtime.SessionTools;

/// <summary>Regras puras do ponteiro pelo controle, separadas para os testes.</summary>
public static class ControllerMouseMath
{
    /// <summary>Zona morta radial do analógico esquerdo (ponteiro); analógicos gastos descansam um pouco fora do centro.</summary>
    public const double MoveDeadZone = 0.18;

    /// <summary>O direito (rolagem) pede um empurrão mais claro, para mirar com o esquerdo nunca rolar sem querer.</summary>
    public const double ScrollDeadZone = 0.3;

    /// <summary>No máximo, o ponteiro cruza umas tantas alturas de tela por segundo: uma TV 4K parece uma 1080p.</summary>
    public const double ScreensPerSecond = 1.1;

    public const double MinNotchesPerSecond = 3;

    public const double MaxNotchesPerSecond = 16;

    /// <summary>Analógico do XInput (-32768..32767, Y para cima) para -1..1.</summary>
    public static double FromXInput(short value) => Math.Max(-1, value / 32767.0);

    /// <summary>Byte de analógico HID da Sony (0..255, 128 = centro, Y para baixo) para -1..1 com Y para cima, como o XInput.</summary>
    public static double FromHidByte(byte value, bool invert) => Math.Clamp((value - 128) / 127.0, -1, 1) * (invert ? -1 : 1);

    /// <summary>
    /// Zona morta radial e curva quadrática: inclinação pequena mira com precisão, inclinação total percorre a tela.
    /// Entrada e saída em -1..1; a direção é mantida.
    /// </summary>
    public static (double X, double Y) Curve(double x, double y, double deadZone)
    {
        double magnitude = Math.Sqrt((x * x) + (y * y));
        if (magnitude <= deadZone)
        {
            return (0, 0);
        }

        double scaled = Math.Min(1, (magnitude - deadZone) / (1 - deadZone));
        double factor = scaled * scaled / magnitude;
        return (x * factor, y * factor);
    }
}

/// <summary>Analógico esquerdo em pixels do ponteiro, guardando as frações entre amostras para o movimento lento não travar.</summary>
public sealed class PointerMotion
{
    private double _restX;
    private double _restY;

    /// <param name="x">X do analógico, -1..1.</param>
    /// <param name="y">Y do analógico, -1..1, para cima positivo (o Y da tela cresce para baixo).</param>
    /// <param name="seconds">Tempo desde a amostra anterior.</param>
    /// <param name="screenHeight">Altura da tela do jogo em pixels.</param>
    public (int Dx, int Dy) Step(double x, double y, double seconds, int screenHeight)
    {
        var (vx, vy) = ControllerMouseMath.Curve(x, y, ControllerMouseMath.MoveDeadZone);
        if (vx == 0 && vy == 0)
        {
            Reset();
            return (0, 0);
        }

        double speed = Math.Max(screenHeight, 720) * ControllerMouseMath.ScreensPerSecond;
        _restX += vx * speed * seconds;
        _restY -= vy * speed * seconds;
        int dx = (int)Math.Truncate(_restX);
        int dy = (int)Math.Truncate(_restY);
        _restX -= dx;
        _restY -= dy;
        return (dx, dy);
    }

    public void Reset() => _restX = _restY = 0;
}

/// <summary>
/// Um eixo do analógico direito em "dentes" inteiros da roda (positivo = para cima / direita). Só dentes inteiros,
/// porque alguns programas ignoram frações; o primeiro sai assim que o analógico deixa a zona morta.
/// </summary>
public sealed class WheelMotion
{
    private double _pending;
    private bool _moving;

    public int Step(double axis, double seconds)
    {
        double magnitude = Math.Abs(axis);
        if (magnitude <= ControllerMouseMath.ScrollDeadZone)
        {
            Reset();
            return 0;
        }

        double scaled = Math.Min(1, (magnitude - ControllerMouseMath.ScrollDeadZone) / (1 - ControllerMouseMath.ScrollDeadZone));
        double rate = ControllerMouseMath.MinNotchesPerSecond
                      + ((ControllerMouseMath.MaxNotchesPerSecond - ControllerMouseMath.MinNotchesPerSecond) * scaled * scaled);
        if (!_moving)
        {
            _moving = true;
            _pending = 1;
        }
        else
        {
            _pending += rate * seconds;
        }

        int notches = (int)Math.Truncate(_pending);
        _pending -= notches;
        return notches * Math.Sign(axis);
    }

    public void Reset()
    {
        _pending = 0;
        _moving = false;
    }
}

public enum ClickEvent
{
    None,

    /// <summary>A tocado uma vez: botão esquerdo desce e sobe.</summary>
    LeftClick,

    /// <summary>A tocado duas vezes: botão direito desce e sobe.</summary>
    RightClick,

    /// <summary>A segurado: botão esquerdo desce, para arrastar.</summary>
    LeftDown,

    /// <summary>A solto depois de segurar.</summary>
    LeftUp,
}

/// <summary>
/// O botão A como mouse: toque é clique esquerdo, dois toques rápidos clique direito, segurar arrasta com o esquerdo.
/// O clique simples espera <see cref="DoubleTapWindow"/> depois de soltar, para saber que não é o início de um clique direito.
/// </summary>
public sealed class ClickGesture
{
    public static readonly TimeSpan HoldForDrag = TimeSpan.FromMilliseconds(350);

    public static readonly TimeSpan DoubleTapWindow = TimeSpan.FromMilliseconds(250);

    private Phase _phase = Phase.WaitRelease;
    private TimeSpan _since;

    private enum Phase
    {
        WaitRelease,
        Idle,
        FirstPress,
        WaitSecondTap,
        Dragging,
    }

    /// <summary>Uma amostra do botão A em <paramref name="now"/> (qualquer relógio monotônico).</summary>
    public ClickEvent Update(bool held, TimeSpan now)
    {
        switch (_phase)
        {
            case Phase.WaitRelease:
                if (!held)
                {
                    _phase = Phase.Idle;
                }

                return ClickEvent.None;

            case Phase.Idle:
                if (held)
                {
                    Enter(Phase.FirstPress, now);
                }

                return ClickEvent.None;

            case Phase.FirstPress:
                if (!held)
                {
                    Enter(Phase.WaitSecondTap, now);
                    return ClickEvent.None;
                }

                if (now - _since < HoldForDrag)
                {
                    return ClickEvent.None;
                }

                _phase = Phase.Dragging;
                return ClickEvent.LeftDown;

            case Phase.WaitSecondTap:
                if (held)
                {
                    _phase = Phase.WaitRelease;
                    return ClickEvent.RightClick;
                }

                if (now - _since < DoubleTapWindow)
                {
                    return ClickEvent.None;
                }

                _phase = Phase.Idle;
                return ClickEvent.LeftClick;

            case Phase.Dragging:
                if (held)
                {
                    return ClickEvent.None;
                }

                _phase = Phase.Idle;
                return ClickEvent.LeftUp;
        }

        return ClickEvent.None;
    }

    /// <summary>
    /// Recomeça ignorando o A até ele ser solto (o toque que fechou a Central não pode clicar).
    /// Verdadeiro quando o botão esquerdo ficou pressionado de um arraste e quem chamou precisa soltá-lo.
    /// </summary>
    public bool Reset()
    {
        bool wasDragging = _phase == Phase.Dragging;
        _phase = Phase.WaitRelease;
        return wasDragging;
    }

    private void Enter(Phase phase, TimeSpan now)
    {
        _phase = phase;
        _since = now;
    }
}
