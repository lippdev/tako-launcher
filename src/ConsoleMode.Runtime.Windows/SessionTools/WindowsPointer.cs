// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/ControllerMouse.cs e src/ConsoleMode/Services/CursorHider.cs
using System.Diagnostics;
using System.Runtime.InteropServices;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;
using static ConsoleMode.Runtime.Windows.SessionTools.SessionToolsNative;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>Amostra de um controle que o XInput não vê (ex.: PlayStation por HID), entregue pela UI.</summary>
public readonly record struct PointerPadSample(double LeftX, double LeftY, bool A);

/// <summary>
/// Ponteiro durante a sessão, pela Central de Controle.
/// <para>
/// Esconder: o Windows não tem "esconder o ponteiro em todos os apps", então os cursores do sistema são trocados por
/// um vazio (<c>SetSystemCursor</c>) e recarregados do esquema do usuário depois (<c>SPI_SETCURSORS</c>). Um arquivo
/// marcador cobre uma queda no meio: a próxima abertura devolve os cursores (<see cref="RecoverAfterCrash"/>).
/// </para>
/// <para>
/// Controlar com o analógico, para jogos e emuladores cujos menus não aceitam controle: o esquerdo move, o direito rola,
/// A clica (duas vezes = botão direito, segurar = arrastar). Lê o XInput, que funciona com o jogo em foco, numa thread
/// própria para o ponteiro ficar suave com a UI ocupada; pausa enquanto a Central está aberta.
/// </para>
/// </summary>
public sealed partial class WindowsPointer(string markerFile, Func<PointerPadSample?> otherPads, ILogger<WindowsPointer> logger) : IPointerService, IDisposable
{
    // OCR_NORMAL, IBEAM, WAIT, CROSS, UP, SIZENWSE, SIZENESW, SIZEWE, SIZENS, SIZEALL, NO, HAND, APPSTARTING, HELP, PIN, PERSON.
    private static readonly uint[] SystemCursors =
        [32512, 32513, 32514, 32515, 32516, 32642, 32643, 32644, 32645, 32646, 32648, 32649, 32650, 32651, 32671, 32672];

    private static readonly TimeSpan SampleInterval = TimeSpan.FromMilliseconds(8);

    private const uint SpiSetCursors = 0x0057;
    private const int SmCxCursor = 13;
    private const int SmCyCursor = 14;
    private const ushort XInputA = 0x1000;
    private const uint MouseMove = 0x0001;
    private const uint MouseLeftDown = 0x0002;
    private const uint MouseLeftUp = 0x0004;
    private const uint MouseRightDown = 0x0008;
    private const uint MouseRightUp = 0x0010;
    private const uint MouseWheel = 0x0800;
    private const uint MouseHWheel = 0x1000;
    private const int WheelDelta = 120;

    private readonly object _gate = new();
    private CancellationTokenSource? _mouse;
    private Thread? _thread;
    private volatile bool _paused;
    private bool _xinputMissing;

    public bool IsCursorHidden { get; private set; }

    public bool IsControllerMouseOn => _mouse is not null;

    public bool IsControllerMousePaused
    {
        get => _paused;
        set => _paused = value;
    }

    public void SetCursorHidden(bool hidden)
    {
        lock (_gate)
        {
            if (hidden == IsCursorHidden)
            {
                return;
            }

            IsCursorHidden = hidden;
            if (hidden)
            {
                HideCursors();
            }
            else
            {
                RestoreCursors();
            }
        }
    }

    public void SetControllerMouse(bool enabled, int screenHeight)
    {
        lock (_gate)
        {
            if (enabled && _mouse is null)
            {
                var cancel = new CancellationTokenSource();
                _mouse = cancel;
                _thread = new Thread(() => Run(screenHeight, cancel.Token)) { IsBackground = true, Name = "Tako ponteiro pelo controle" };
                _thread.Start();
                LogControllerMouse(logger, true);
            }
            else if (!enabled && _mouse is { } cancel)
            {
                _mouse = null;
                cancel.Cancel();
                _thread?.Join(500);
                _thread = null;
                cancel.Dispose();
                LogControllerMouse(logger, false);
            }
        }
    }

    public void Reset()
    {
        SetControllerMouse(false, 0);
        SetCursorHidden(false);
        _paused = false;
    }

    /// <summary>Na abertura: uma execução que caiu com o ponteiro escondido deixou o marcador.</summary>
    public void RecoverAfterCrash()
    {
        try
        {
            if (File.Exists(markerFile))
            {
                RestoreCursors();
                LogCursorRecovered(logger);
            }
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogCursorFailed(logger, ex);
        }
    }

    public void Dispose() => Reset();

    private static void Send(uint flags, int dx = 0, int dy = 0, int data = 0)
    {
        var input = new Input
        {
            Type = 0, // INPUT_MOUSE
            Mouse = new MouseInput { Dx = dx, Dy = dy, MouseData = unchecked((uint)data), Flags = flags },
        };

        // Falha em silêncio contra janela de administrador (UIPI): nada a fazer daqui.
        unsafe
        {
            _ = SendInput(1, &input, sizeof(Input));
        }
    }

    /// <summary>Um cursor do tamanho do sistema cuja máscara AND mantém a tela e a XOR não acrescenta nada.</summary>
    private static unsafe nint CreateBlankCursor()
    {
        int width = Math.Max(GetSystemMetrics(SmCxCursor), 32);
        int height = Math.Max(GetSystemMetrics(SmCyCursor), 32);
        int size = (width + 15) / 16 * 2 * height; // linhas alinhadas em WORD
        byte[] and = new byte[size];
        Array.Fill(and, (byte)0xFF);
        byte[] xor = new byte[size];
        fixed (byte* a = and)
        fixed (byte* x = xor)
        {
            return CreateCursor(GetModuleHandle(null), 0, 0, width, height, a, x);
        }
    }

    private void HideCursors()
    {
        WriteMarker(true);
        nint blank = CreateBlankCursor();
        if (blank == 0)
        {
            LogCursorWin32(logger, Marshal.GetLastPInvokeError());
            return;
        }

        // SetSystemCursor fica dono de cada handle: cada posição recebe a sua cópia.
        foreach (uint id in SystemCursors)
        {
            SetSystemCursor(CopyIcon(blank), id);
        }

        DestroyCursor(blank);
        LogCursorHidden(logger, true);
    }

    private void RestoreCursors()
    {
        // Recarrega o esquema do próprio usuário (Configurações › Ponteiro do mouse), seja qual for.
        if (!SystemParametersInfo(SpiSetCursors, 0, 0, 0))
        {
            LogCursorWin32(logger, Marshal.GetLastPInvokeError());
        }

        WriteMarker(false);
        LogCursorHidden(logger, false);
    }

    private void WriteMarker(bool hidden)
    {
        try
        {
            if (hidden)
            {
                Directory.CreateDirectory(Path.GetDirectoryName(markerFile)!);
                File.WriteAllText(markerFile, string.Empty);
            }
            else if (File.Exists(markerFile))
            {
                File.Delete(markerFile);
            }
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogCursorFailed(logger, ex);
        }
    }

    private void Run(int screenHeight, CancellationToken cancel)
    {
        var pointer = new PointerMotion();
        var wheelY = new WheelMotion();
        var wheelX = new WheelMotion();
        var gesture = new ClickGesture();
        var clock = Stopwatch.StartNew();
        var last = clock.Elapsed;
        bool wasPaused = true;
        bool leftDown = false;
        try
        {
            while (!cancel.IsCancellationRequested)
            {
                Thread.Sleep(SampleInterval);
                var now = clock.Elapsed;

                // Uma parada (suspensão, depurador) não pode jogar o ponteiro para o outro lado da tela.
                double seconds = Math.Min((now - last).TotalSeconds, 0.05);
                last = now;
                if (_paused)
                {
                    if (!wasPaused)
                    {
                        if (gesture.Reset())
                        {
                            Send(MouseLeftUp);
                        }

                        leftDown = false;
                        pointer.Reset();
                        wheelX.Reset();
                        wheelY.Reset();
                    }

                    wasPaused = true;
                    continue;
                }

                if (wasPaused)
                {
                    // Voltando (ou começando): o A que fechou a Central provavelmente ainda está pressionado.
                    gesture.Reset();
                    wasPaused = false;
                }

                var pad = Read();
                var (dx, dy) = pointer.Step(pad.LeftX, pad.LeftY, seconds, screenHeight);
                if (dx != 0 || dy != 0)
                {
                    Send(MouseMove, dx, dy);
                }

                int notchesY = wheelY.Step(pad.RightY, seconds);
                if (notchesY != 0)
                {
                    Send(MouseWheel, data: notchesY * WheelDelta);
                }

                int notchesX = wheelX.Step(pad.RightX, seconds);
                if (notchesX != 0)
                {
                    Send(MouseHWheel, data: notchesX * WheelDelta);
                }

                switch (gesture.Update(pad.A, now))
                {
                    case ClickEvent.LeftClick:
                        Send(MouseLeftDown);
                        Send(MouseLeftUp);
                        break;
                    case ClickEvent.RightClick:
                        Send(MouseRightDown);
                        Send(MouseRightUp);
                        break;
                    case ClickEvent.LeftDown:
                        Send(MouseLeftDown);
                        leftDown = true;
                        break;
                    case ClickEvent.LeftUp:
                        Send(MouseLeftUp);
                        leftDown = false;
                        break;
                }
            }
        }
        catch (Exception ex) when (ex is ExternalException or DllNotFoundException or EntryPointNotFoundException or InvalidOperationException)
        {
            LogControllerMouseFailed(logger, ex);
        }
        finally
        {
            // Nunca deixa o botão esquerdo preso quando o modo termina no meio de um arraste.
            if (leftDown)
            {
                Send(MouseLeftUp);
            }
        }
    }

    /// <summary>Todos os controles de uma vez: vence o analógico mais empurrado; A conta em qualquer controle.</summary>
    private unsafe (double LeftX, double LeftY, double RightX, double RightY, bool A) Read()
    {
        double lx = 0, ly = 0, rx = 0, ry = 0;
        bool a = false;

        static void Take(double x, double y, ref double bestX, ref double bestY)
        {
            if ((x * x) + (y * y) > (bestX * bestX) + (bestY * bestY))
            {
                bestX = x;
                bestY = y;
            }
        }

        if (!_xinputMissing)
        {
            try
            {
                for (uint i = 0; i < 4; i++)
                {
                    XInputState state;
                    if (XInputGetState(i, &state) != 0)
                    {
                        continue;
                    }

                    var g = state.Gamepad;
                    Take(ControllerMouseMath.FromXInput(g.ThumbLX), ControllerMouseMath.FromXInput(g.ThumbLY), ref lx, ref ly);
                    Take(ControllerMouseMath.FromXInput(g.ThumbRX), ControllerMouseMath.FromXInput(g.ThumbRY), ref rx, ref ry);
                    a |= (g.Buttons & XInputA) != 0;
                }
            }
            catch (DllNotFoundException)
            {
                _xinputMissing = true;
            }
        }

        if (otherPads() is { } other)
        {
            Take(other.LeftX, other.LeftY, ref lx, ref ly);
            a |= other.A;
        }

        return (lx, ly, rx, ry, a);
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Ponteiro pelo controle: {On}")]
    private static partial void LogControllerMouse(ILogger logger, bool on);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Ponteiro pelo controle parou")]
    private static partial void LogControllerMouseFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Information, Message = "Ponteiro escondido: {Hidden}")]
    private static partial void LogCursorHidden(ILogger logger, bool hidden);

    [LoggerMessage(Level = LogLevel.Information, Message = "Ponteiro restaurado depois de um encerramento inesperado")]
    private static partial void LogCursorRecovered(ILogger logger);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Cursores do sistema: erro Win32 {Error}")]
    private static partial void LogCursorWin32(ILogger logger, int error);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Marcador do ponteiro escondido não pôde ser gravado")]
    private static partial void LogCursorFailed(ILogger logger, Exception exception);
}
