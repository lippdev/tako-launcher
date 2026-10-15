using System.Runtime.InteropServices;
using ConsoleMode.Runtime.Display;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Atalho global <c>Ctrl+Alt+Shift+D</c> por <c>RegisterHotKey</c>, numa thread própria com fila de mensagens (como a
/// cortina preta): funciona com o jogo em primeiro plano e mesmo se a thread da UI estiver ocupada. A ação roda nessa
/// thread.
/// </summary>
public sealed unsafe partial class WindowsEmergencyHotkey : IEmergencyHotkey
{
    private const int HotkeyId = 0x7A4B; // qualquer id da thread
    private const uint ModAlt = 0x0001;
    private const uint ModControl = 0x0002;
    private const uint ModShift = 0x0004;
    private const uint ModNoRepeat = 0x4000;
    private const uint VirtualKeyD = 0x44;
    private const uint WmHotkey = 0x0312;
    private const uint WmQuit = 0x0012;

    private readonly object _gate = new();
    private Thread? _thread;
    private uint _threadId;

    public string Keys => "Ctrl+Alt+Shift+D";

    public bool Start(Action pressed)
    {
        ArgumentNullException.ThrowIfNull(pressed);
        lock (_gate)
        {
            if (_thread is not null)
            {
                return true;
            }

            bool registered = false;
            using var ready = new ManualResetEventSlim();
            _thread = new Thread(() => Run(pressed, ready, ok => registered = ok)) { IsBackground = true, Name = "Tako: atalho de emergência" };
            _thread.Start();
            ready.Wait(TimeSpan.FromSeconds(5));
            if (!registered)
            {
                _thread = null;
            }

            return registered;
        }
    }

    public void Dispose()
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

    private void Run(Action pressed, ManualResetEventSlim ready, Action<bool> started)
    {
        bool ok;
        try
        {
            _threadId = GetCurrentThreadId();
            ok = RegisterHotKey(0, HotkeyId, ModControl | ModAlt | ModShift | ModNoRepeat, VirtualKeyD);
            started(ok);
        }
        finally
        {
            ready.Set();
        }

        if (!ok)
        {
            return;
        }

        Msg msg;
        while (GetMessageW(&msg, 0, 0, 0) > 0)
        {
            if (msg.Message == WmHotkey && msg.WParam == HotkeyId)
            {
                pressed();
            }
        }

        UnregisterHotKey(0, HotkeyId);
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
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool RegisterHotKey(nint hwnd, int id, uint modifiers, uint virtualKey);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool UnregisterHotKey(nint hwnd, int id);

    [LibraryImport("user32.dll")]
    private static partial int GetMessageW(Msg* msg, nint hwnd, uint filterMin, uint filterMax);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool PostThreadMessageW(uint threadId, uint msg, nint wParam, nint lParam);

    [LibraryImport("kernel32.dll")]
    private static partial uint GetCurrentThreadId();
}
