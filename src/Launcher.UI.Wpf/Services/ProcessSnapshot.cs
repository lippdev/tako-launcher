using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using Launcher.Core.Session;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Lista de processos do Windows para o acompanhamento de jogos (MVP-026): id e pai pelo Toolhelp, executável e início
/// pelo próprio processo. Acesso negado (sistema, outro usuário, anti-cheat) vira caminho nulo, sem erro. Caminhos ficam
/// em cache por id, então cada leitura só consulta os processos novos.
/// </summary>
internal sealed partial class ProcessSnapshot
{
    private const uint SnapProcess = 0x2;
    private const uint QueryLimitedInformation = 0x1000;
    private const int ShowRestore = 9;
    private static readonly nint InvalidHandle = -1;

    private readonly Dictionary<int, (string? Path, DateTimeOffset? Started)> _cache = [];

    public unsafe IReadOnlyList<ProcessEntry> Take()
    {
        nint snapshot = CreateToolhelp32Snapshot(SnapProcess, 0);
        if (snapshot == InvalidHandle)
        {
            return [];
        }

        var entries = new List<ProcessEntry>(400);
        var seen = new HashSet<int>();
        try
        {
            var entry = new ProcessEntry32 { Size = (uint)sizeof(ProcessEntry32) };
            for (bool ok = Process32FirstW(snapshot, &entry); ok; ok = Process32NextW(snapshot, &entry))
            {
                int id = (int)entry.ProcessId;
                seen.Add(id);
                if (!_cache.TryGetValue(id, out var info))
                {
                    info = Query(id);
                    _cache[id] = info;
                }

                entries.Add(new ProcessEntry(id, (int)entry.ParentProcessId, info.Path, info.Started));
            }
        }
        finally
        {
            CloseHandle(snapshot);
        }

        foreach (int gone in _cache.Keys.Where(id => !seen.Contains(id)).ToList())
        {
            _cache.Remove(gone);
        }

        return entries;
    }

    /// <summary>Traz para frente a janela principal de um dos processos do jogo ("Voltar ao jogo").</summary>
    public static bool TryBringToFront(IEnumerable<int> processIds)
    {
        foreach (int id in processIds)
        {
            try
            {
                using var process = Process.GetProcessById(id);
                nint window = process.MainWindowHandle;
                if (window == 0)
                {
                    continue;
                }

                if (IsIconic(window))
                {
                    ShowWindow(window, ShowRestore);
                }

                return SetForegroundWindow(window);
            }
            catch (Exception ex) when (ex is ArgumentException or InvalidOperationException or Win32Exception)
            {
                // O processo fechou ou não pode ser lido: tenta o próximo.
            }
        }

        return false;
    }

    private static unsafe (string? Path, DateTimeOffset? Started) Query(int id)
    {
        nint handle = OpenProcess(QueryLimitedInformation, false, (uint)id);
        if (handle == 0)
        {
            return (null, null);
        }

        try
        {
            const int Capacity = 1024;
            char* buffer = stackalloc char[Capacity];
            uint size = Capacity;
            string? path = QueryFullProcessImageNameW(handle, 0, buffer, ref size) ? new string(buffer, 0, (int)size) : null;
            DateTimeOffset? started = GetProcessTimes(handle, out long created, out _, out _, out _) ? DateTimeOffset.FromFileTime(created) : null;
            return (path, started);
        }
        finally
        {
            CloseHandle(handle);
        }
    }

    [LibraryImport("kernel32.dll", SetLastError = true)]
    private static partial nint CreateToolhelp32Snapshot(uint flags, uint processId);

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static unsafe partial bool Process32FirstW(nint snapshot, ProcessEntry32* entry);

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static unsafe partial bool Process32NextW(nint snapshot, ProcessEntry32* entry);

    [LibraryImport("kernel32.dll", SetLastError = true)]
    private static partial nint OpenProcess(uint access, [MarshalAs(UnmanagedType.Bool)] bool inheritHandle, uint processId);

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static unsafe partial bool QueryFullProcessImageNameW(nint process, uint flags, char* name, ref uint size);

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool GetProcessTimes(nint process, out long creation, out long exit, out long kernel, out long user);

    [LibraryImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool CloseHandle(nint handle);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool IsIconic(nint window);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool ShowWindow(nint window, int command);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetForegroundWindow(nint window);

    [StructLayout(LayoutKind.Sequential)]
    private unsafe struct ProcessEntry32
    {
        public uint Size;
        public uint Usage;
        public uint ProcessId;
        public nint DefaultHeapId;
        public uint ModuleId;
        public uint Threads;
        public uint ParentProcessId;
        public int PriorityClassBase;
        public uint Flags;
        public fixed char ExeFile[260];
    }
}
