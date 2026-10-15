using System.Text.Json;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>O que o driver de um fabricante oferece numa placa.</summary>
public sealed record DriverProbe(FrameRateLimitAvailability Availability, int MinFps, int MaxFps, bool PerApplication, string Method)
{
    public static DriverProbe Missing { get; } = new(FrameRateLimitAvailability.NoDriver, 0, 0, false, string.Empty);
}

/// <summary>
/// Limite de FPS pelo driver de um fabricante (NVAPI, ADLX, IGCL). Implementações nunca lançam: falha vira falso/nulo.
/// <para>
/// <paramref name="exeName"/> é o nome do arquivo do jogo em minúsculas (perfil por aplicativo) ou nulo (global). O estado
/// guardado por <see cref="Capture"/> é um texto do próprio backend (ex.: <see cref="NvidiaLimitState.Serialize"/>).
/// </para>
/// </summary>
public interface IDriverFrameRateBackend
{
    GpuVendor Vendor { get; }

    DriverProbe Probe(GpuAdapter adapter);

    /// <summary>Limite em vigor (0 = sem limite); nulo quando não deu para ler.</summary>
    int? Read(GpuAdapter adapter, string? exeName);

    /// <summary>Estado de antes, para <see cref="Restore"/>; nulo quando não deu para ler (aí nada é mudado).</summary>
    string? Capture(GpuAdapter adapter, string? exeName);

    bool Apply(GpuAdapter adapter, string? exeName, int fps);

    bool Restore(GpuAdapter adapter, string? exeName, string state);
}

/// <summary>Uma mudança feita pelo Tako e ainda não desfeita (fica em arquivo até voltar, inclusive após uma queda).</summary>
public sealed record FrameRateLimitBackup(GpuVendor Vendor, string AdapterName, string? Exe, string State);

/// <summary>Arquivo JSON com as mudanças a desfazer. Arquivo ausente ou ilegível = nada a desfazer.</summary>
public sealed class FrameRateLimitBackupStore(string path)
{
    private static readonly JsonSerializerOptions Json = new() { WriteIndented = true };

    public string Path { get; } = path;

    public List<FrameRateLimitBackup> Load()
    {
        try
        {
            return File.Exists(Path) ? JsonSerializer.Deserialize<List<FrameRateLimitBackup>>(File.ReadAllText(Path), Json) ?? [] : [];
        }
        catch (JsonException)
        {
            return [];
        }
    }

    /// <summary>Grava (ou apaga, quando vazio). Lança <see cref="IOException"/>/<see cref="UnauthorizedAccessException"/>.</summary>
    public void Save(IReadOnlyList<FrameRateLimitBackup> backups)
    {
        ArgumentNullException.ThrowIfNull(backups);
        if (backups.Count == 0)
        {
            File.Delete(Path);
            return;
        }

        string? folder = System.IO.Path.GetDirectoryName(Path);
        if (!string.IsNullOrEmpty(folder))
        {
            Directory.CreateDirectory(folder);
        }

        string temp = Path + ".tmp";
        File.WriteAllText(temp, JsonSerializer.Serialize(backups, Json));
        File.Move(temp, Path, overwrite: true);
    }
}

/// <summary>
/// Escolhe a placa que roda o jogo (<see cref="GpuSelection"/>), usa o backend do fabricante dela e guarda o "antes" em
/// <see cref="FrameRateLimitBackupStore"/> antes de qualquer escrita, como o RTSS fazia. Puro: placas, preferência do
/// Windows e backends entram pelo construtor. Seguro entre threads.
/// </summary>
public sealed class DriverFrameRateLimiter : IFrameRateLimiter
{
    private readonly Func<IReadOnlyList<GpuAdapter>> _adapters;
    private readonly Func<string?, GpuPreference> _preference;
    private readonly IReadOnlyList<IDriverFrameRateBackend> _backends;
    private readonly FrameRateLimitBackupStore _store;
    private readonly Action<string>? _log;
    private readonly object _gate = new();
    private IReadOnlyList<GpuAdapter>? _adapterCache;

    public DriverFrameRateLimiter(
        Func<IReadOnlyList<GpuAdapter>> adapters,
        Func<string?, GpuPreference> preference,
        IReadOnlyList<IDriverFrameRateBackend> backends,
        FrameRateLimitBackupStore store,
        Action<string>? log = null)
    {
        _adapters = adapters ?? throw new ArgumentNullException(nameof(adapters));
        _preference = preference ?? throw new ArgumentNullException(nameof(preference));
        _backends = backends ?? throw new ArgumentNullException(nameof(backends));
        _store = store ?? throw new ArgumentNullException(nameof(store));
        _log = log;
    }

    public FrameRateLimitCapabilities Capabilities
    {
        get
        {
            lock (_gate)
            {
                return Describe(Target(null));
            }
        }
    }

    public int? ReadCurrent(string? gameExePath)
    {
        lock (_gate)
        {
            var target = Target(gameExePath);
            return target.Backend is { } backend && target.Adapter is { } adapter && target.Probe.Availability == FrameRateLimitAvailability.Available
                ? backend.Read(adapter, ScopeFor(target.Probe, gameExePath))
                : null;
        }
    }

    public bool Apply(string? gameExePath, int fps)
    {
        lock (_gate)
        {
            var target = Target(gameExePath);
            if (target.Backend is not { } backend || target.Adapter is not { } adapter || target.Probe.Availability != FrameRateLimitAvailability.Available)
            {
                _log?.Invoke($"limite de FPS indisponível ({Describe(target).Explanation})");
                return false;
            }

            string? exe = ScopeFor(target.Probe, gameExePath);
            if (target.Probe.PerApplication && exe is null)
            {
                _log?.Invoke("limite de FPS por aplicativo sem o exe do jogo: aguardando o processo");
                return false;
            }

            var backups = _store.Load();
            int index = backups.FindIndex(b => Matches(b, backend.Vendor, adapter.Name, exe));

            // "Sem limite" devolve o valor do usuário; sem mudança do Tako, não há o que fazer.
            if (fps <= 0)
            {
                if (index < 0)
                {
                    return true;
                }

                bool restored = backend.Restore(adapter, exe, backups[index].State);
                backups.RemoveAt(index);
                return TrySave(backups) && restored;
            }

            if (index < 0)
            {
                string? before = backend.Capture(adapter, exe);
                if (before is null)
                {
                    _log?.Invoke("não foi possível ler o limite atual do driver; nada foi mudado");
                    return false;
                }

                backups.Add(new FrameRateLimitBackup(backend.Vendor, adapter.Name, exe, before));
                if (!TrySave(backups))
                {
                    return false; // sem o "antes" em disco, uma queda deixaria o limite para sempre
                }
            }

            return backend.Apply(adapter, exe, Describe(target).Clamp(fps));
        }
    }

    public void Restore()
    {
        lock (_gate)
        {
            var backups = _store.Load();
            if (backups.Count == 0)
            {
                return;
            }

            var adapters = Adapters();
            foreach (var backup in backups)
            {
                var backend = _backends.FirstOrDefault(b => b.Vendor == backup.Vendor);
                var adapter = adapters.FirstOrDefault(a => a.Vendor == backup.Vendor && a.Name == backup.AdapterName)
                    ?? adapters.FirstOrDefault(a => a.Vendor == backup.Vendor);
                if (backend is null || adapter is null)
                {
                    _log?.Invoke($"placa {GpuVendors.Name(backup.Vendor)} não encontrada: limite de antes não pôde voltar");
                    continue;
                }

                if (!backend.Restore(adapter, backup.Exe, backup.State))
                {
                    _log?.Invoke($"o driver {GpuVendors.Name(backup.Vendor)} recusou devolver o limite de antes");
                }
            }

            // Mesmo com recusa o registro sai: tentar de novo a cada abertura não muda o resultado.
            TrySave([]);
        }
    }

    private static bool Matches(FrameRateLimitBackup backup, GpuVendor vendor, string adapterName, string? exe) =>
        backup.Vendor == vendor && backup.AdapterName == adapterName && string.Equals(backup.Exe, exe, StringComparison.OrdinalIgnoreCase);

    private static string? ScopeFor(DriverProbe probe, string? gameExePath) => probe.PerApplication ? GameExecutables.FileName(gameExePath) : null;

    private static FrameRateLimitCapabilities Describe((GpuAdapter? Adapter, IDriverFrameRateBackend? Backend, DriverProbe Probe) target) =>
        target.Adapter is not { } adapter
            ? FrameRateLimitCapabilities.None
            : new FrameRateLimitCapabilities(adapter.Vendor, adapter.Name, target.Probe.Availability, target.Probe.MinFps, target.Probe.MaxFps, target.Probe.PerApplication, target.Probe.Method);

    private (GpuAdapter? Adapter, IDriverFrameRateBackend? Backend, DriverProbe Probe) Target(string? gameExePath)
    {
        var adapter = GpuSelection.Choose(Adapters(), gameExePath is null ? GpuPreference.Default : _preference(gameExePath));
        if (adapter is null)
        {
            return (null, null, DriverProbe.Missing with { Availability = FrameRateLimitAvailability.NoGpu });
        }

        var backend = _backends.FirstOrDefault(b => b.Vendor == adapter.Vendor);
        if (backend is null)
        {
            return (adapter, null, DriverProbe.Missing with { Availability = FrameRateLimitAvailability.Unsupported });
        }

        // Os backends guardam o resultado da primeira abertura do driver: as leituras seguintes são baratas.
        return (adapter, backend, backend.Probe(adapter));
    }

    private IReadOnlyList<GpuAdapter> Adapters() => _adapterCache ??= _adapters();

    private bool TrySave(IReadOnlyList<FrameRateLimitBackup> backups)
    {
        try
        {
            _store.Save(backups);
            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            _log?.Invoke($"não foi possível gravar {_store.Path}: {ex.Message}");
            return false;
        }
    }
}
