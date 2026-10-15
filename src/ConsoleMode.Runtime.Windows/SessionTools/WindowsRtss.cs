// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/RtssService.cs
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO.MemoryMappedFiles;
using System.Runtime.InteropServices;
using System.Security;
using System.Text;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;
using Microsoft.Win32;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Contador e limite de FPS pelo RivaTuner Statistics Server, quando instalado (o Tako não o distribui).
/// <para>
/// Contador: texto num espaço de OSD próprio na memória compartilhada <c>RTSSSharedMemoryV2</c> (como o
/// <c>UpdateOSD</c> da amostra do SDK). Estilos com CPU/GPU/memória são reescritos a cada segundo; enquanto um estilo
/// do Tako está ligado, o OSD do MSI Afterburner fica escondido.
/// </para>
/// <para>
/// Limite: propriedade <c>FramerateLimit</c> do perfil global, pela <c>RTSSHooks64.dll</c> do próprio RTSS (a API de
/// perfis do SDK, a mesma que o rtss-cli usa no Console Mode). O valor do usuário é guardado em arquivo na primeira
/// mudança e volta em <see cref="Restore"/>, inclusive na abertura seguinte a uma queda.
/// </para>
/// </summary>
public sealed partial class WindowsRtss(string limitBackupFile, ILogger<WindowsRtss> logger) : IFpsOverlayService
{
    // RTSSSharedMemoryV2 (SDK do RTSS, RTSSSharedMemory.h): deslocamentos do cabeçalho e formato das entradas de OSD.
    private const string SharedMemoryName = "RTSSSharedMemoryV2";
    private const uint Signature = 0x52545353; // 'RTSS'
    private const int OffVersion = 4;
    private const int OffOsdEntrySize = 20;
    private const int OffOsdArrOffset = 24;
    private const int OffOsdArrSize = 28;
    private const int OffOsdFrame = 32;
    private const int OffBusy = 36;
    private const int EntryOsd = 0;
    private const int EntryOwner = 256;
    private const int EntryOsdEx = 512;
    private const int OsdSize = 256;
    private const int OwnerSize = 256;
    private const int OsdExSize = 4096;
    private const int EntryOsdEx2 = EntryOsdEx + OsdExSize + 262144; // depois do szOSDEx e do buffer de objetos de 256 KB
    private const string AfterburnerOwner = "MSIAfterburner";
    private const string LimitProperty = "FramerateLimit";

    private readonly object _slotLock = new();
    private readonly object _limitLock = new();
    private readonly Lazy<string?> _folder = new(FindFolder);
    private int _overlayGeneration;
    private volatile bool _hideAfterburner;
    private Thread? _afterburnerHider;
    private RtssProfiles? _profiles;
    private bool _profilesTried;

    public bool IsInstalled => _folder.Value is not null;

    public bool CanLimitFps => Profiles() is not null;

    public bool ApplyOverlay(string style, FpsOverlayLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        if (!IsInstalled)
        {
            return false;
        }

        int generation = StopTicker();
        string text = RtssOverlay.Text(style, layout);
        if (!ShowOverlay(text, RtssOverlay.HidesOthers(style)))
        {
            return false;
        }

        if (text.Length > 0 && RtssOverlay.NeedsHardware(style, layout))
        {
            new Thread(() => TickOverlay(generation, style, layout)) { IsBackground = true, Name = "Tako contador RTSS" }.Start();
        }

        return true;
    }

    public bool SetFpsLimit(int fps)
    {
        lock (_limitLock)
        {
            if (Profiles() is not { } profiles)
            {
                return false;
            }

            try
            {
                // Só a primeira mudança guarda o valor do usuário; as seguintes já têm a cópia.
                if (!File.Exists(limitBackupFile))
                {
                    int original = profiles.GetLimit() ?? 0;
                    Directory.CreateDirectory(Path.GetDirectoryName(limitBackupFile)!);
                    File.WriteAllText(limitBackupFile, original.ToString(CultureInfo.InvariantCulture));
                }

                profiles.SetLimit(Math.Max(0, fps));
                LogLimitSet(logger, fps);
                return true;
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                LogRtssFailed(logger, ex, "limite de FPS");
                return false;
            }
        }
    }

    public void Restore()
    {
        StopTicker();
        _hideAfterburner = false;
        lock (_slotLock)
        {
            WriteOverlaySlot(null);
        }

        lock (_limitLock)
        {
            if (!File.Exists(limitBackupFile))
            {
                return;
            }

            try
            {
                if (Profiles() is { } profiles
                    && int.TryParse(File.ReadAllText(limitBackupFile).Trim(), NumberStyles.Integer, CultureInfo.InvariantCulture, out int original))
                {
                    profiles.SetLimit(Math.Max(0, original));
                    LogLimitRestored(logger, original);
                }

                File.Delete(limitBackupFile);
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                LogRtssFailed(logger, ex, "restaurar o limite de FPS");
            }
        }
    }

    private static string? FindFolder()
    {
        string? registry = null;
        try
        {
            using var key = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry32).OpenSubKey(@"SOFTWARE\Unwinder\RTSS");
            registry = key?.GetValue("InstallDir") as string;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or SecurityException)
        {
            // Sem acesso ao registro: tenta as pastas padrão.
        }

        var folders = RtssLocator.Folders(
            registry,
            Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
            Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles));
        return RtssLocator.Resolve(folders, File.Exists);
    }

    private static bool IsRunning()
    {
        foreach (string name in new[] { "RTSS", "RTSSHooksLoader64", "RTSSHooksLoader" })
        {
            var processes = Process.GetProcessesByName(name);
            bool found = processes.Length > 0;
            foreach (var process in processes)
            {
                process.Dispose();
            }

            if (found)
            {
                return true;
            }
        }

        return false;
    }

    private static unsafe bool IsOwner(byte* entry, byte[] owner) =>
        new ReadOnlySpan<byte>(entry + EntryOwner, owner.Length + 1).SequenceEqual([.. owner, (byte)0]);

    private RtssProfiles? Profiles()
    {
        lock (_limitLock)
        {
            if (!_profilesTried && _folder.Value is { } folder)
            {
                _profilesTried = true;
                _profiles = RtssProfiles.TryOpen(Path.Combine(folder, RtssLocator.HooksLibrary));
            }

            return _profiles;
        }
    }

    private bool EnsureRunning()
    {
        if (IsRunning())
        {
            return true;
        }

        if (_folder.Value is not { } folder)
        {
            return false;
        }

        try
        {
            // O RTSS pede administrador: o Windows pode perguntar uma vez.
            Process.Start(new ProcessStartInfo { FileName = Path.Combine(folder, RtssLocator.ExeName), WindowStyle = ProcessWindowStyle.Hidden, UseShellExecute = true })?.Dispose();
            Thread.Sleep(1500);
            return IsRunning();
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
        {
            LogRtssFailed(logger, ex, "abrir o RTSS");
            return false;
        }
    }

    /// <summary>Termina o atualizador em andamento (a próxima escrita dele vê uma geração mais nova) e devolve a nova geração.</summary>
    private int StopTicker()
    {
        lock (_slotLock)
        {
            return ++_overlayGeneration;
        }
    }

    private void TickOverlay(int generation, string style, FpsOverlayLayout layout)
    {
        try
        {
            using var monitor = new WindowsHardwareMonitor();
            while (Volatile.Read(ref _overlayGeneration) == generation)
            {
                Thread.Sleep(1000);
                string text = RtssOverlay.Text(style, layout, monitor.Sample());

                // Conferido sob a trava para uma sessão que acabou de terminar não ganhar o contador de volta.
                lock (_slotLock)
                {
                    if (_overlayGeneration != generation)
                    {
                        return;
                    }

                    WriteOverlaySlot(text);
                }
            }
        }
        catch (Exception ex) when (ex is ExternalException or InvalidOperationException or UnauthorizedAccessException)
        {
            LogRtssFailed(logger, ex, "atualizar CPU/GPU do contador");
        }
    }

    private bool ShowOverlay(string text, bool hideOthers)
    {
        if (string.IsNullOrEmpty(text))
        {
            lock (_slotLock)
            {
                WriteOverlaySlot(null);
            }

            // Desligado não abre o RTSS só para esconder algo que não está lá.
            if (hideOthers && IsRunning())
            {
                HideAfterburner();
            }
            else
            {
                _hideAfterburner = false;
            }

            return true;
        }

        if (!EnsureRunning())
        {
            return false;
        }

        // Logo depois de o RTSS abrir a memória compartilhada pode ainda não existir.
        for (int attempt = 0; attempt < 10; attempt++)
        {
            bool written;
            lock (_slotLock)
            {
                written = WriteOverlaySlot(text);
            }

            if (written)
            {
                if (hideOthers)
                {
                    HideAfterburner();
                }
                else
                {
                    _hideAfterburner = false;
                }

                return true;
            }

            Thread.Sleep(300);
        }

        return false;
    }

    private void HideAfterburner()
    {
        _hideAfterburner = true;
        if (_afterburnerHider is { IsAlive: true })
        {
            return;
        }

        _afterburnerHider = new Thread(HideAfterburnerLoop) { IsBackground = true, Name = "Tako RTSS Afterburner", Priority = ThreadPriority.AboveNormal };
        _afterburnerHider.Start();
    }

    /// <summary>
    /// O RTSS desenha todo espaço e não tem como esconder um, e o atalho do Afterburner desliga o OSD inteiro (o do Tako
    /// também). Então, com o contador do Tako ligado, o texto do Afterburner é apagado logo depois de cada reescrita
    /// (uma por período de leitura, ~1 s): no pior caso aparece por um quadro. Quando o Tako para, ele volta sozinho.
    /// </summary>
    private unsafe void HideAfterburnerLoop()
    {
        try
        {
            using var map = MemoryMappedFile.OpenExisting(SharedMemoryName, MemoryMappedFileRights.ReadWrite);
            using var view = map.CreateViewAccessor(0, 0, MemoryMappedFileAccess.ReadWrite);
            byte* mem = null;
            view.SafeMemoryMappedViewHandle.AcquirePointer(ref mem);
            try
            {
                mem += view.PointerOffset;
                byte[] afterburner = Encoding.ASCII.GetBytes(AfterburnerOwner);
                uint version = *(uint*)(mem + OffVersion);
                uint entrySize = *(uint*)(mem + OffOsdEntrySize);
                byte* slot = mem + *(uint*)(mem + OffOsdArrOffset); // o Afterburner fica no espaço 0
                bool hasEx2 = version >= 0x00020014 && entrySize > EntryOsdEx2;
                while (_hideAfterburner && *(uint*)mem == Signature)
                {
                    if (IsOwner(slot, afterburner) && (slot[EntryOsd] != 0 || slot[EntryOsdEx] != 0 || (hasEx2 && slot[EntryOsdEx2] != 0)))
                    {
                        slot[EntryOsd] = 0;
                        slot[EntryOsdEx] = 0;
                        if (hasEx2)
                        {
                            slot[EntryOsdEx2] = 0;
                        }

                        Interlocked.Increment(ref *(int*)(mem + OffOsdFrame));
                    }

                    Thread.Sleep(1);
                }
            }
            finally
            {
                view.SafeMemoryMappedViewHandle.ReleasePointer();
            }
        }
        catch (FileNotFoundException)
        {
            // RTSS fechado: nada a esconder.
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogRtssFailed(logger, ex, "esconder o OSD do Afterburner");
        }
    }

    /// <summary>Nulo = libera o espaço do Tako. Espelha UpdateOSD/ReleaseOSD da amostra do SDK. Chamar sob <see cref="_slotLock"/>.</summary>
    private unsafe bool WriteOverlaySlot(string? text)
    {
        try
        {
            using var map = MemoryMappedFile.OpenExisting(SharedMemoryName, MemoryMappedFileRights.ReadWrite);
            using var view = map.CreateViewAccessor(0, 0, MemoryMappedFileAccess.ReadWrite);
            byte* mem = null;
            view.SafeMemoryMappedViewHandle.AcquirePointer(ref mem);
            try
            {
                mem += view.PointerOffset;
                uint version = *(uint*)(mem + OffVersion);
                if (*(uint*)mem != Signature || version < 0x00020000)
                {
                    return false;
                }

                uint entrySize = *(uint*)(mem + OffOsdEntrySize);
                uint arrOffset = *(uint*)(mem + OffOsdArrOffset);
                uint arrSize = *(uint*)(mem + OffOsdArrSize);
                byte[] owner = Encoding.ASCII.GetBytes(RtssOverlay.Owner);
                byte* entries = mem + arrOffset;

                if (text is null)
                {
                    // O espaço 0 é do cliente principal (Afterburner); terceiros começam no 1.
                    for (uint i = 1; i < arrSize; i++)
                    {
                        byte* entry = entries + (i * entrySize);
                        if (IsOwner(entry, owner))
                        {
                            new Span<byte>(entry, (int)entrySize).Clear();
                            Interlocked.Increment(ref *(int*)(mem + OffOsdFrame));
                        }
                    }

                    return true;
                }

                // O espaço que o Tako já tem, senão o primeiro livre.
                byte* slot = null;
                for (uint i = 1; i < arrSize && slot is null; i++)
                {
                    if (IsOwner(entries + (i * entrySize), owner))
                    {
                        slot = entries + (i * entrySize);
                    }
                }

                for (uint i = 1; i < arrSize && slot is null; i++)
                {
                    byte* entry = entries + (i * entrySize);
                    if (entry[EntryOwner] != 0)
                    {
                        continue;
                    }

                    owner.CopyTo(new Span<byte>(entry + EntryOwner, OwnerSize));
                    slot = entry;
                }

                if (slot is null)
                {
                    return false;
                }

                byte[] bytes = Encoding.Latin1.GetBytes(text); // a fonte do RTSS tem ° (0xB0)
                bool extended = version >= 0x00020007;
                var target = new Span<byte>(slot + (extended ? EntryOsdEx : EntryOsd), extended ? OsdExSize : OsdSize);

                // v2.14+: o bit 0 de dwBusy trava o OSD enquanto o renderizador lê.
                bool lockable = version >= 0x0002000e;
                ref int busy = ref *(int*)(mem + OffBusy);
                for (int spin = 0; lockable && (Interlocked.Or(ref busy, 1) & 1) != 0; spin++)
                {
                    if (spin > 200)
                    {
                        return false;
                    }

                    Thread.Sleep(1);
                }

                try
                {
                    target.Clear();
                    bytes.AsSpan(0, Math.Min(bytes.Length, target.Length - 1)).CopyTo(target);
                }
                finally
                {
                    if (lockable)
                    {
                        Interlocked.And(ref busy, ~1);
                    }
                }

                Interlocked.Increment(ref *(int*)(mem + OffOsdFrame));
                return true;
            }
            finally
            {
                view.SafeMemoryMappedViewHandle.ReleasePointer();
            }
        }
        catch (FileNotFoundException)
        {
            return false; // RTSS fechado: nada na tela para mostrar ou limpar
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogRtssFailed(logger, ex, "atualizar o contador de FPS");
            return false;
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "RTSS: limite de FPS {Fps} (0 = sem limite)")]
    private static partial void LogLimitSet(ILogger logger, int fps);

    [LoggerMessage(Level = LogLevel.Information, Message = "RTSS: limite de FPS do usuário restaurado ({Fps})")]
    private static partial void LogLimitRestored(ILogger logger, int fps);

    [LoggerMessage(Level = LogLevel.Warning, Message = "RTSS: não foi possível {Action}")]
    private static partial void LogRtssFailed(ILogger logger, Exception exception, string action);

    /// <summary>
    /// API de perfis do SDK do RTSS exportada pela <c>RTSSHooks64.dll</c>: <c>LoadProfile</c>, <c>SaveProfile</c>,
    /// <c>GetProfileProperty</c>, <c>SetProfileProperty</c> e <c>UpdateProfiles</c>. Perfil "" é o global.
    /// A biblioteca fica carregada até o fim do processo.
    /// </summary>
    private sealed unsafe class RtssProfiles
    {
        private static readonly byte[] Global = [0];
        private static readonly byte[] Limit = [.. Encoding.ASCII.GetBytes(LimitProperty), 0];

        private readonly delegate* unmanaged[Cdecl]<byte*, void> _load;
        private readonly delegate* unmanaged[Cdecl]<byte*, void> _save;
        private readonly delegate* unmanaged[Cdecl]<byte*, byte*, uint, int> _get;
        private readonly delegate* unmanaged[Cdecl]<byte*, byte*, uint, int> _set;
        private readonly delegate* unmanaged[Cdecl]<void> _update;

        private RtssProfiles(nint load, nint save, nint get, nint set, nint update)
        {
            _load = (delegate* unmanaged[Cdecl]<byte*, void>)load;
            _save = (delegate* unmanaged[Cdecl]<byte*, void>)save;
            _get = (delegate* unmanaged[Cdecl]<byte*, byte*, uint, int>)get;
            _set = (delegate* unmanaged[Cdecl]<byte*, byte*, uint, int>)set;
            _update = (delegate* unmanaged[Cdecl]<void>)update;
        }

        public static RtssProfiles? TryOpen(string path)
        {
            if (!File.Exists(path) || !NativeLibrary.TryLoad(path, out nint library))
            {
                return null;
            }

            return NativeLibrary.TryGetExport(library, "LoadProfile", out nint load)
                && NativeLibrary.TryGetExport(library, "SaveProfile", out nint save)
                && NativeLibrary.TryGetExport(library, "GetProfileProperty", out nint get)
                && NativeLibrary.TryGetExport(library, "SetProfileProperty", out nint set)
                && NativeLibrary.TryGetExport(library, "UpdateProfiles", out nint update)
                ? new RtssProfiles(load, save, get, set, update)
                : null;
        }

        public int? GetLimit()
        {
            int value = 0;
            fixed (byte* profile = Global)
            fixed (byte* name = Limit)
            {
                _load(profile);
                return _get(name, (byte*)&value, sizeof(int)) != 0 ? value : null;
            }
        }

        public void SetLimit(int fps)
        {
            fixed (byte* profile = Global)
            fixed (byte* name = Limit)
            {
                _load(profile);
                _set(name, (byte*)&fps, sizeof(int));
                _save(profile);
                _update();
            }
        }
    }
}
