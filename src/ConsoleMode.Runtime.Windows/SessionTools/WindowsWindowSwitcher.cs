// Adaptado de lippdev/consolemode: src/ConsoleMode/Native/WindowSwitcher.cs e src/ConsoleMode/Native/WindowIcons.cs
using System.Runtime.InteropServices;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;
using static ConsoleMode.Runtime.Windows.SessionTools.SessionToolsNative;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Alternador de janelas da Central de Controle no Windows: lista o que o Alt+Tab listaria, da frente para trás, traz
/// uma janela para a frente e pede para outra fechar. Só mexe em janelas: a sessão de jogo não termina porque o foco
/// saiu do jogo. Ícones: o do executável no tamanho pedido ou, sem ele, o que a janela mostra na barra de tarefas.
/// </summary>
public sealed unsafe partial class WindowsWindowSwitcher(ILogger<WindowsWindowSwitcher> logger) : IWindowSwitcher
{
    public IReadOnlyList<OpenWindow> List()
    {
        var handles = new List<nint>();
        var gc = GCHandle.Alloc(handles);
        try
        {
            _ = EnumWindows(&Collect, GCHandle.ToIntPtr(gc));
        }
        finally
        {
            gc.Free();
        }

        uint own = (uint)Environment.ProcessId;
        var result = new List<OpenWindow>();
        foreach (nint hwnd in handles)
        {
            try
            {
                if (Describe(hwnd, own) is { } window)
                {
                    result.Add(window);
                }
            }
            catch (Exception ex) when (ex is ExternalException or InvalidOperationException or ArgumentException)
            {
                LogWindowSkipped(logger, ex);
            }
        }

        return result;
    }

    public IconBitmap? LoadIcon(OpenWindow window, int size)
    {
        ArgumentNullException.ThrowIfNull(window);
        try
        {
            return FromExe(window.ExePath, size) ?? FromWindow(window.Handle);
        }
        catch (Exception ex) when (ex is ExternalException or ArgumentException or IOException)
        {
            LogIconFailed(logger, ex, window.ProcessName);
            return null;
        }
    }

    public bool Activate(nint handle)
    {
        if (handle == 0 || !IsWindow(handle))
        {
            return false;
        }

        if (IsIconic(handle))
        {
            ShowWindow(handle, SwRestore);
        }

        // O Windows recusa SetForegroundWindow de um processo em segundo plano: entra na fila de entrada de quem está
        // em primeiro plano antes de pedir.
        uint ourThread = GetCurrentThreadId();
        nint current = GetForegroundWindow();
        uint theirThread = current == 0 ? 0 : GetWindowThreadProcessId(current, null);
        bool attached = theirThread != 0 && theirThread != ourThread && AttachThreadInput(ourThread, theirThread, true);
        try
        {
            BringWindowToTop(handle);
            return SetForegroundWindow(handle);
        }
        finally
        {
            if (attached)
            {
                AttachThreadInput(ourThread, theirThread, false);
            }
        }
    }

    public bool RequestClose(nint handle) => handle != 0 && IsWindow(handle) && PostMessage(handle, WmClose, 0, 0);

    [UnmanagedCallersOnly]
    private static int Collect(nint hwnd, nint state)
    {
        if (IsWindowVisible(hwnd) && GCHandle.FromIntPtr(state).Target is List<nint> list)
        {
            list.Add(hwnd);
        }

        return 1;
    }

    private static OpenWindow? Describe(nint hwnd, uint ownProcessId)
    {
        uint pid;
        _ = GetWindowThreadProcessId(hwnd, &pid);
        char* title = stackalloc char[256];
        int titleLength = GetWindowText(hwnd, title, 256);
        char* className = stackalloc char[128];
        int classLength = GetClassName(hwnd, className, 128);
        long style = GetWindowLongPtr(hwnd, GwlExStyle);
        int cloaked = 0;
        _ = DwmGetWindowAttribute(hwnd, DwmaCloaked, &cloaked, sizeof(int));

        var facts = new WindowFacts(
            new string(title, 0, Math.Max(0, titleLength)),
            new string(className, 0, Math.Max(0, classLength)),
            Visible: true,
            Cloaked: cloaked != 0,
            HasOwner: GetWindow(hwnd, GwOwner) != 0,
            IsToolWindow: (style & WsExToolWindow) != 0,
            IsAppWindow: (style & WsExAppWindow) != 0,
            IsOwnProcess: pid == ownProcessId);
        if (!AltTabRules.ShouldList(facts))
        {
            return null;
        }

        string? exe = ExePathOf(pid);
        return new OpenWindow(hwnd, facts.Title, AltTabRules.ProcessDisplayName(exe is null ? null : Path.GetFileName(exe)), exe);
    }

    private static string? ExePathOf(uint pid)
    {
        nint process = OpenProcess(ProcessQueryLimitedInformation, false, pid);
        if (process == 0)
        {
            return null;
        }

        try
        {
            const int Capacity = 1024;
            char* buffer = stackalloc char[Capacity];
            int size = Capacity;
            return QueryFullProcessImageName(process, 0, buffer, &size) ? new string(buffer, 0, size) : null;
        }
        finally
        {
            CloseHandle(process);
        }
    }

    private static IconBitmap? FromExe(string? path, int size)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            return null;
        }

        // Escolhe o tamanho mais próximo guardado no exe e escala: um ícone pequeno não fica perdido num quadro grande.
        nint icon = 0;
        uint count = PrivateExtractIcons(path, 0, size, size, &icon, null, 1, 0);
        if (count == 0 || count == uint.MaxValue || icon == 0)
        {
            return null;
        }

        try
        {
            return ReadIcon(icon);
        }
        finally
        {
            DestroyIcon(icon);
        }
    }

    private static IconBitmap? FromWindow(nint hwnd)
    {
        foreach (var (message, classIndex) in new[] { (IconBig, GclpHIcon), (IconSmall2, GclpHIconSm) })
        {
            nint result = 0;
            nint icon = SendMessageTimeout(hwnd, WmGetIcon, message, 0, SmtoAbortIfHung, 150, &result) != 0 ? result : 0;
            if (icon == 0)
            {
                icon = GetClassLongPtr(hwnd, classIndex);
            }

            // Pertence à janela: só lido, nunca destruído.
            if (icon != 0 && ReadIcon(icon) is { } read)
            {
                return read;
            }
        }

        return null;
    }

    private static IconBitmap? ReadIcon(nint icon)
    {
        IconInfo info;
        if (!GetIconInfo(icon, &info))
        {
            return null;
        }

        try
        {
            if (info.Color == 0)
            {
                return null; // monocromático: não vale mostrar
            }

            var color = ReadBitmap(info.Color);
            if (color is null)
            {
                return null;
            }

            var mask = info.Mask == 0 ? null : ReadBitmap(info.Mask);
            bool sameSize = mask is not null && mask.Width == color.Width && mask.Height == color.Height;
            IconPixels.Premultiply(color.Pixels, sameSize ? mask!.Pixels : null);
            return IconPixels.IsBlank(color.Pixels) ? null : color;
        }
        finally
        {
            if (info.Color != 0)
            {
                DeleteObject(info.Color);
            }

            if (info.Mask != 0)
            {
                DeleteObject(info.Mask);
            }
        }
    }

    private static IconBitmap? ReadBitmap(nint bitmap)
    {
        Bitmap facts;
        if (GetObject(bitmap, sizeof(Bitmap), &facts) == 0)
        {
            return null;
        }

        int width = facts.Width;
        int height = Math.Abs(facts.Height);
        if (width <= 0 || height <= 0 || width > 1024 || height > 1024)
        {
            return null;
        }

        // Altura negativa: linhas de cima primeiro.
        var header = new BitmapInfoHeader { Size = sizeof(BitmapInfoHeader), Width = width, Height = -height, Planes = 1, BitCount = 32 };
        byte[] pixels = new byte[width * height * 4];
        nint screen = GetDC(0);
        try
        {
            fixed (byte* p = pixels)
            {
                return GetDIBits(screen, bitmap, 0, (uint)height, p, &header, 0) == height ? new IconBitmap(width, height, pixels) : null;
            }
        }
        finally
        {
            _ = ReleaseDC(0, screen);
        }
    }

    [LoggerMessage(Level = LogLevel.Debug, Message = "Janela pulada no alternador")]
    private static partial void LogWindowSkipped(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Debug, Message = "Ícone de {Process} não pôde ser lido")]
    private static partial void LogIconFailed(ILogger logger, Exception exception, string process);
}
