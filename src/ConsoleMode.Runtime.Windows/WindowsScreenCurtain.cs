// Adaptado de lippdev/consolemode: src/ConsoleMode/Native/BlackCurtain.cs
using System.Runtime.InteropServices;
using ConsoleMode.Runtime.Display;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Cortina preta: uma janela preta, sem borda e sempre por cima, em cada tela que deve ficar sem imagem. As janelas
/// vivem numa thread própria com fila de mensagens, então funcionam chamadas de qualquer thread (inclusive fora da UI)
/// e nunca roubam o foco do Tako ou do jogo.
/// </summary>
public sealed unsafe partial class WindowsScreenCurtain : IScreenCurtain, IDisposable
{
    private const string ClassName = "TakoGameScreenCurtain";
    private const uint WsPopup = 0x80000000;
    private const uint WsVisible = 0x10000000;
    private const uint WsExTopmost = 0x00000008;
    private const uint WsExToolWindow = 0x00000080;
    private const uint WsExNoActivate = 0x08000000;
    private const int SwShowNoActivate = 4;
    private const uint WmQuit = 0x0012;
    private const int BlackBrush = 4;
    private const int ErrorClassAlreadyExists = 1410;

    private readonly object _gate = new();
    private Thread? _thread;
    private uint _threadId;

    public void Show(IReadOnlyList<ScreenBounds> screens)
    {
        ArgumentNullException.ThrowIfNull(screens);
        Close();
        if (screens.Count == 0)
        {
            return;
        }

        lock (_gate)
        {
            using var ready = new ManualResetEventSlim();
            var copy = screens.ToArray();
            _thread = new Thread(() => Run(copy, ready)) { IsBackground = true, Name = "Tako: cortina preta" };
            _thread.Start();
            ready.Wait(TimeSpan.FromSeconds(5));
        }
    }

    public void Close()
    {
        lock (_gate)
        {
            if (_thread is null)
            {
                return;
            }

            if (_threadId != 0)
            {
                PostThreadMessageW(_threadId, WmQuit, 0, 0);
            }

            _thread.Join(TimeSpan.FromSeconds(2));
            _thread = null;
            _threadId = 0;
        }
    }

    public void Dispose() => Close();

    private void Run(ScreenBounds[] screens, ManualResetEventSlim ready)
    {
        var windows = new List<nint>();
        try
        {
            _threadId = GetCurrentThreadId();
            nint instance = GetModuleHandleW(null);
            EnsureClass(instance);
            foreach (var s in screens)
            {
                nint hwnd = CreateWindowExW(WsExTopmost | WsExToolWindow | WsExNoActivate, ClassName, string.Empty, WsPopup | WsVisible, s.X, s.Y, s.Width, s.Height, 0, 0, instance, 0);
                if (hwnd != 0)
                {
                    ShowWindow(hwnd, SwShowNoActivate);
                    windows.Add(hwnd);
                }
            }
        }
        finally
        {
            ready.Set();
        }

        Msg msg;
        while (GetMessageW(&msg, 0, 0, 0) > 0)
        {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        foreach (nint hwnd in windows)
        {
            DestroyWindow(hwnd);
        }
    }

    private static void EnsureClass(nint instance)
    {
        fixed (char* name = ClassName)
        {
            var wc = new WndClassEx
            {
                Size = (uint)sizeof(WndClassEx),
                WndProc = &WndProc,
                Instance = instance,
                Background = GetStockObject(BlackBrush),
                ClassName = name,
            };
            if (RegisterClassExW(&wc) == 0 && Marshal.GetLastPInvokeError() != ErrorClassAlreadyExists)
            {
                throw new RuntimeEnvironmentException($"RegisterClassEx falhou ({Marshal.GetLastPInvokeError()})");
            }
        }
    }

    [UnmanagedCallersOnly]
    private static nint WndProc(nint hwnd, uint msg, nint wParam, nint lParam) => DefWindowProcW(hwnd, msg, wParam, lParam);

    [StructLayout(LayoutKind.Sequential)]
    private struct WndClassEx
    {
        public uint Size;
        public uint Style;
        public delegate* unmanaged<nint, uint, nint, nint, nint> WndProc;
        public int ClassExtra;
        public int WindowExtra;
        public nint Instance;
        public nint Icon;
        public nint Cursor;
        public nint Background;
        public char* MenuName;
        public char* ClassName;
        public nint IconSmall;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Msg
    {
        public nint Hwnd;
        public uint Message;
        public nint WParam;
        public nint LParam;
        public uint Time;
        public int X;
        public int Y;
        public uint Private;
    }

    [LibraryImport("user32.dll", SetLastError = true)]
    private static partial ushort RegisterClassExW(WndClassEx* windowClass);

    [LibraryImport("user32.dll", StringMarshalling = StringMarshalling.Utf16)]
    private static partial nint CreateWindowExW(uint exStyle, string className, string windowName, uint style, int x, int y, int width, int height, nint parent, nint menu, nint instance, nint param);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool ShowWindow(nint hwnd, int command);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool DestroyWindow(nint hwnd);

    [LibraryImport("user32.dll")]
    private static partial nint DefWindowProcW(nint hwnd, uint msg, nint wParam, nint lParam);

    [LibraryImport("user32.dll")]
    private static partial int GetMessageW(Msg* msg, nint hwnd, uint filterMin, uint filterMax);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool TranslateMessage(Msg* msg);

    [LibraryImport("user32.dll")]
    private static partial nint DispatchMessageW(Msg* msg);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool PostThreadMessageW(uint threadId, uint msg, nint wParam, nint lParam);

    [LibraryImport("kernel32.dll")]
    private static partial uint GetCurrentThreadId();

    [LibraryImport("kernel32.dll", StringMarshalling = StringMarshalling.Utf16)]
    private static partial nint GetModuleHandleW(string? moduleName);

    [LibraryImport("gdi32.dll")]
    private static partial nint GetStockObject(int index);
}

/// <summary>Posiciona uma janela numa tela, em pixels físicos (o Tako é PerMonitorV2).</summary>
public static partial class WindowPlacement
{
    private const uint SwpNoZOrder = 0x0004;
    private const uint SwpNoActivate = 0x0010;

    public static bool MoveTo(nint hwnd, ScreenBounds bounds) =>
        hwnd != 0 && SetWindowPos(hwnd, 0, bounds.X, bounds.Y, bounds.Width, bounds.Height, SwpNoZOrder | SwpNoActivate);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetWindowPos(nint hwnd, nint insertAfter, int x, int y, int width, int height, uint flags);
}
