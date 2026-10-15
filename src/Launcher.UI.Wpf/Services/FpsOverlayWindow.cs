using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Interop;
using System.Windows.Media;
using ConsoleMode.Runtime.SessionTools;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Contador de FPS do Tako por cima do jogo, sem injeção (ADR-0017): uma janela transparente, sempre no topo, que não
/// recebe clique nem foco e não aparece no Alt+Tab (WS_EX_LAYERED | TRANSPARENT | TOOLWINDOW | NOACTIVATE). Fica no
/// canto escolhido do monitor do jogo, em pixels físicos, e o WPF escala o texto pelo DPI daquele monitor (2K/4K).
/// Em tela cheia exclusiva o Windows não mostra janelas por cima do jogo: lá o FPS fica só na Central de Controle.
/// </summary>
public sealed partial class FpsOverlayWindow : Window
{
    private const double BaseFontSize = 15;

    private static readonly Brush LabelBrush = Frozen(Color.FromRgb(0x9A, 0xA4, 0xB2));
    private static readonly Brush AccentBrush = Frozen(Color.FromRgb(0x4C, 0xC2, 0xFF));
    private static readonly Brush ValueBrush = Frozen(Colors.White);
    private static readonly Brush CardBrush = Frozen(Color.FromArgb(0xC0, 0x10, 0x14, 0x18));

    private readonly Border _card;
    private readonly StackPanel _lines;
    private nint _gameWindow;
    private string _position = FpsOverlay.TopLeft;

    public FpsOverlayWindow()
    {
        WindowStyle = WindowStyle.None;
        AllowsTransparency = true;
        Background = Brushes.Transparent;
        ResizeMode = ResizeMode.NoResize;
        ShowInTaskbar = false;
        ShowActivated = false;
        Focusable = false;
        Topmost = true;
        SizeToContent = SizeToContent.WidthAndHeight;
        Title = "Contador de FPS do Tako";
        _lines = new StackPanel();
        _card = new Border { Child = _lines, CornerRadius = new CornerRadius(8), Padding = new Thickness(8, 6, 8, 6) };
        Content = _card;
        TextOptions.SetTextFormattingMode(this, TextFormattingMode.Display);
        SizeChanged += (_, _) => Reposition();
        DpiChanged += (_, _) => Reposition();
    }

    protected override void OnSourceInitialized(EventArgs e)
    {
        base.OnSourceInitialized(e);
        var handle = new WindowInteropHelper(this).Handle;
        nint style = Native.GetWindowLongPtr(handle, Native.GwlExStyle);
        Native.SetWindowLongPtr(handle, Native.GwlExStyle, style | Native.ExLayered | Native.ExTransparent | Native.ExToolWindow | Native.ExNoActivate);
    }

    /// <summary>A janela não recebe clique nem foco e fica fora do Alt+Tab (o que o teste de fumaça confere).</summary>
    public bool IsClickThroughToolWindow
    {
        get
        {
            var handle = new WindowInteropHelper(this).Handle;
            nint wanted = Native.ExLayered | Native.ExTransparent | Native.ExToolWindow | Native.ExNoActivate;
            return handle != 0 && (Native.GetWindowLongPtr(handle, Native.GwlExStyle) & wanted) == wanted;
        }
    }

    /// <summary>Redesenha com os números de agora e leva a janela para o canto do monitor do jogo.</summary>
    public void Update(IReadOnlyList<IReadOnlyList<OverlayRun>> rows, FpsOverlayLayout layout, nint gameWindow)
    {
        ArgumentNullException.ThrowIfNull(rows);
        ArgumentNullException.ThrowIfNull(layout);
        _gameWindow = gameWindow;
        _position = FpsOverlay.NormalizePosition(layout.Position);
        double size = BaseFontSize * FpsOverlay.Scale(layout.Size);
        _card.Background = layout.Card ? CardBrush : Brushes.Transparent;
        _card.CornerRadius = new CornerRadius(8 * FpsOverlay.Scale(layout.Size));
        _lines.Children.Clear();
        foreach (var row in rows)
        {
            var line = new TextBlock { FontSize = size, FontWeight = FontWeights.SemiBold };
            if (!layout.Card)
            {
                // Sem cartão, uma sombra deixa o texto legível em cenas claras.
                line.Effect = new System.Windows.Media.Effects.DropShadowEffect { BlurRadius = 3, ShadowDepth = 1, Opacity = 0.9 };
            }

            foreach (var run in row)
            {
                line.Inlines.Add(Styled(run, size));
            }

            _lines.Children.Add(line);
        }

        if (!IsVisible)
        {
            Show();
        }

        Reposition();
    }

    /// <summary>Mantém por cima de um jogo em janela sem borda que também pede "sempre no topo".</summary>
    public void KeepOnTop()
    {
        var handle = new WindowInteropHelper(this).Handle;
        if (handle != 0)
        {
            Native.SetWindowPos(handle, Native.HwndTopmost, 0, 0, 0, 0, Native.SwpNoMove | Native.SwpNoSize | Native.SwpNoActivate);
        }
    }

    private static Run Styled(OverlayRun run, double size)
    {
        var text = new Run(run.Text);
        switch (run.Kind)
        {
            case OverlayRunKind.Fps:
                text.Foreground = ValueBrush;
                text.FontSize = size * 1.3;
                text.FontWeight = FontWeights.Bold;
                break;
            case OverlayRunKind.Value:
                text.Foreground = ValueBrush;
                break;
            case OverlayRunKind.Accent:
                text.Foreground = AccentBrush;
                text.FontSize = size * 0.75;
                break;
            default:
                text.Foreground = LabelBrush;
                text.FontSize = size * 0.75;
                break;
        }

        return text;
    }

    /// <summary>Canto do monitor onde está a janela do jogo (ou o principal), em pixels físicos.</summary>
    private void Reposition()
    {
        var handle = new WindowInteropHelper(this).Handle;
        if (handle == 0)
        {
            return;
        }

        nint monitor = Native.MonitorFromWindow(_gameWindow != 0 ? _gameWindow : handle, Native.MonitorDefaultToPrimary);
        var info = new Native.MonitorInfo { Size = Marshal.SizeOf<Native.MonitorInfo>() };
        if (monitor == 0 || !Native.GetMonitorInfo(monitor, ref info) || !Native.GetWindowRect(handle, out var window))
        {
            return;
        }

        double scale = Native.GetDpiForMonitor(monitor, 0, out uint dpiX, out _) == 0 && dpiX > 0 ? dpiX / 96.0 : 1;
        var area = new PixelRect(info.Monitor.Left, info.Monitor.Top, info.Monitor.Right, info.Monitor.Bottom);
        var (x, y) = OverlayPlacement.Place(area, window.Right - window.Left, window.Bottom - window.Top, _position, scale);
        Native.SetWindowPos(handle, Native.HwndTopmost, x, y, 0, 0, Native.SwpNoSize | Native.SwpNoActivate);
    }

    private static SolidColorBrush Frozen(Color color)
    {
        var brush = new SolidColorBrush(color);
        brush.Freeze();
        return brush;
    }

    private static partial class Native
    {
        public const int GwlExStyle = -20;
        public const nint ExLayered = 0x00080000;
        public const nint ExTransparent = 0x00000020;
        public const nint ExToolWindow = 0x00000080;
        public const nint ExNoActivate = 0x08000000;
        public const uint MonitorDefaultToPrimary = 1;
        public const uint SwpNoSize = 0x0001;
        public const uint SwpNoMove = 0x0002;
        public const uint SwpNoActivate = 0x0010;
        public static readonly nint HwndTopmost = -1;

        [LibraryImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
        public static partial nint GetWindowLongPtr(nint hwnd, int index);

        [LibraryImport("user32.dll", EntryPoint = "SetWindowLongPtrW")]
        public static partial nint SetWindowLongPtr(nint hwnd, int index, nint value);

        [LibraryImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static partial bool SetWindowPos(nint hwnd, nint after, int x, int y, int width, int height, uint flags);

        [LibraryImport("user32.dll")]
        public static partial nint MonitorFromWindow(nint hwnd, uint flags);

        [LibraryImport("user32.dll", EntryPoint = "GetMonitorInfoW")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static partial bool GetMonitorInfo(nint monitor, ref MonitorInfo info);

        [LibraryImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static partial bool GetWindowRect(nint hwnd, out Rect rect);

        [LibraryImport("shcore.dll")]
        public static partial int GetDpiForMonitor(nint monitor, int type, out uint dpiX, out uint dpiY);

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
