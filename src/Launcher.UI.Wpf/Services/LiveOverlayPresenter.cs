using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Central de Controle "ao vivo" (como a sobreposição da Steam): a própria janela do Tako fica transparente, sempre no
/// topo e do tamanho do monitor do jogo, mostrando só a Central e um escurecimento leve; o jogo continua visível e
/// rodando atrás. A transparência é a da composição do DWM (moldura estendida a toda a janela e fundo transparente),
/// sem <c>AllowsTransparency</c>: a janela continua acelerada por hardware e o modo volta ao normal sem recriá-la.
/// Em tela cheia exclusiva uma janela por cima tira o jogo da tela: lá a MainWindow não usa este modo
/// (<see cref="IsExclusiveFullscreen"/>).
/// </summary>
internal sealed partial class LiveOverlayPresenter(Window window, FrameworkElement stage, IReadOnlyList<UIElement> hiddenLayers)
{
    private Brush? _windowBackground;
    private Brush? _stageBackground;
    private Color? _composition;
    private WindowState _state;
    private Native.Rect? _home;

    public bool IsActive { get; private set; }

    /// <summary>
    /// O jogo em primeiro plano está em tela cheia exclusiva (Direct3D): o Windows informa isso ao shell para silenciar
    /// avisos. Fora do Windows, ou se a consulta falhar, considera que não.
    /// </summary>
    public static bool IsExclusiveFullscreen()
    {
        const int RunningD3dFullScreen = 3;
        try
        {
            return Native.SHQueryUserNotificationState(out int state) == 0 && state == RunningD3dFullScreen;
        }
        catch (Exception ex) when (ex is DllNotFoundException or EntryPointNotFoundException)
        {
            return false;
        }
    }

    /// <summary>Janela em primeiro plano agora (o jogo, quando o atalho chega com o Tako atrás).</summary>
    public static nint ForegroundWindow() => Native.GetForegroundWindow();

    /// <summary>Fica transparente e sempre no topo, cobrindo o monitor de <paramref name="gameWindow"/>.</summary>
    public void Enter(nint gameWindow)
    {
        var handle = new WindowInteropHelper(window).Handle;
        if (IsActive || handle == 0)
        {
            return;
        }

        IsActive = true;
        foreach (var layer in hiddenLayers)
        {
            layer.Opacity = 0;
        }

        stage.ClearValue(UIElement.OpacityProperty);
        _windowBackground = window.Background;
        window.Background = Brushes.Transparent;
        if (stage is System.Windows.Controls.Panel panel)
        {
            _stageBackground = panel.Background;
            panel.Background = Brushes.Transparent;
        }

        if (HwndSource.FromHwnd(handle)?.CompositionTarget is { } target)
        {
            _composition = target.BackgroundColor;
            target.BackgroundColor = Colors.Transparent;
        }

        var sheet = new Native.Margins { Left = -1, Right = -1, Top = -1, Bottom = -1 };
        _ = Native.DwmExtendFrameIntoClientArea(handle, ref sheet);

        // Jogo em outro monitor (ex.: a TV): a janela vai até ele enquanto a Central está aberta.
        nint own = Native.MonitorFromWindow(handle, Native.MonitorDefaultToNearest);
        nint game = gameWindow != 0 ? Native.MonitorFromWindow(gameWindow, Native.MonitorDefaultToNearest) : own;
        _home = null;
        if (game != 0 && game != own && MonitorBounds(own) is { } home && MonitorBounds(game) is { } gameBounds)
        {
            _home = home;
            _state = window.WindowState;
            window.WindowState = WindowState.Normal;
            Native.SetWindowPos(handle, Native.HwndTopmost, gameBounds.Left, gameBounds.Top, gameBounds.Right - gameBounds.Left, gameBounds.Bottom - gameBounds.Top, Native.SwpNoActivate);
        }

        window.Topmost = true;
    }

    /// <summary>A Central fechou: some de cima do jogo na hora (o jogo volta ao primeiro plano em seguida).</summary>
    public void Leave()
    {
        if (!IsActive)
        {
            return;
        }

        stage.Opacity = 0;
        window.Topmost = false;
    }

    /// <summary>Depois que o jogo (ou outra janela) voltou à frente: a janela do Tako volta a ser a tela cheia normal.</summary>
    public void Restore()
    {
        var handle = new WindowInteropHelper(window).Handle;
        if (!IsActive)
        {
            return;
        }

        IsActive = false;
        window.Topmost = false;
        if (handle != 0)
        {
            var none = default(Native.Margins);
            _ = Native.DwmExtendFrameIntoClientArea(handle, ref none);
            if (_composition is { } color && HwndSource.FromHwnd(handle)?.CompositionTarget is { } target)
            {
                target.BackgroundColor = color;
            }

            if (_home is { } home)
            {
                Native.SetWindowPos(handle, Native.HwndNoTopmost, home.Left, home.Top, home.Right - home.Left, home.Bottom - home.Top, Native.SwpNoActivate);
                window.WindowState = _state;
            }
        }

        window.Background = _windowBackground;
        if (stage is System.Windows.Controls.Panel panel)
        {
            panel.Background = _stageBackground;
        }

        foreach (var layer in hiddenLayers)
        {
            layer.ClearValue(UIElement.OpacityProperty);
        }

        stage.ClearValue(UIElement.OpacityProperty);
        _home = null;
    }

    private static Native.Rect? MonitorBounds(nint monitor)
    {
        var info = new Native.MonitorInfo { Size = Marshal.SizeOf<Native.MonitorInfo>() };
        return monitor != 0 && Native.GetMonitorInfo(monitor, ref info) ? info.Monitor : null;
    }

    private static partial class Native
    {
        public const uint MonitorDefaultToNearest = 2;
        public const uint SwpNoActivate = 0x0010;
        public static readonly nint HwndTopmost = -1;
        public static readonly nint HwndNoTopmost = -2;

        [LibraryImport("shell32.dll")]
        public static partial int SHQueryUserNotificationState(out int state);

        [LibraryImport("user32.dll")]
        public static partial nint GetForegroundWindow();

        [LibraryImport("dwmapi.dll")]
        public static partial int DwmExtendFrameIntoClientArea(nint hwnd, ref Margins margins);

        [LibraryImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static partial bool SetWindowPos(nint hwnd, nint after, int x, int y, int width, int height, uint flags);

        [LibraryImport("user32.dll")]
        public static partial nint MonitorFromWindow(nint hwnd, uint flags);

        [LibraryImport("user32.dll", EntryPoint = "GetMonitorInfoW")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static partial bool GetMonitorInfo(nint monitor, ref MonitorInfo info);

        [StructLayout(LayoutKind.Sequential)]
        public struct Margins
        {
            public int Left;
            public int Right;
            public int Top;
            public int Bottom;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct Rect
        {
            public int Left;
            public int Top;
            public int Right;
            public int Bottom;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct MonitorInfo
        {
            public int Size;
            public Rect Monitor;
            public Rect Work;
            public uint Flags;
        }
    }
}
