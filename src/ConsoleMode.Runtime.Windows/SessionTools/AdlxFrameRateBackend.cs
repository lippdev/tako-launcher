using System.Runtime.InteropServices;
using ConsoleMode.Runtime.SessionTools;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Limite de FPS da AMD pelo ADLX (<c>amdadlx64.dll</c>, instalada com o driver Adrenalin): FRTC (Frame Rate Target
/// Control) quando a placa tem, senão Radeon Chill com mínimo e máximo iguais ao limite. Os dois são globais da placa
/// (o ADLX não expõe perfis por jogo), por isso o valor de antes sempre volta no fim da sessão.
/// <para>
/// O ADLX é C++, mas cada interface é uma tabela de funções com ordem fixa (a mesma que os headers do SDK descrevem para
/// C, <c>IADLX...Vtbl</c>); as chamadas aqui usam essa ordem, sem DLL nativa nossa. Índices conferidos nos headers do
/// ADLX SDK 1.x/2.0 (<c>ISystem.h</c>, <c>I3DSettings.h</c>, <c>ICollections.h</c>); nada do SDK é redistribuído.
/// </para>
/// </summary>
public sealed unsafe class AdlxFrameRateBackend : IDriverFrameRateBackend
{
    private const string Library = "amdadlx64.dll";

    // ADLX_RESULT: ADLX_OK, ADLX_ALREADY_ENABLED e ADLX_ALREADY_INITIALIZED contam como sucesso.
    private const int AlreadyInitialized = 2;

    // IADLXSystem (não é IADLXInterface: começa direto nos métodos).
    private const int SystemGetGpus = 1;
    private const int SystemGet3DSettingsServices = 7;

    // IADLXInterface: Acquire, Release, QueryInterface.
    private const int Release = 1;

    // IADLXGPUList: métodos de IADLXList e depois At_GPUList.
    private const int ListSize = 3;
    private const int ListAtGpu = 11;

    // IADLXGPU.
    private const int GpuName = 7;

    // IADLX3DSettingsServices.
    private const int ServicesGetChill = 4;
    private const int ServicesGetFrtc = 9;

    // IADLX3DFrameRateTargetControl e IADLX3DChill começam iguais: IsSupported, IsEnabled, GetFPSRange.
    private const int FeatureIsSupported = 3;
    private const int FeatureIsEnabled = 4;
    private const int FeatureGetRange = 5;
    private const int FrtcGetFps = 6;
    private const int FrtcSetEnabled = 7;
    private const int FrtcSetFps = 8;
    private const int ChillGetMin = 6;
    private const int ChillGetMax = 7;
    private const int ChillSetEnabled = 8;
    private const int ChillSetMin = 9;
    private const int ChillSetMax = 10;

    private readonly object _gate = new();
    private readonly Action<string>? _log;
    private readonly Dictionary<string, (DriverProbe Probe, AmdLimitMethod Method)> _probes = new(StringComparer.Ordinal);
    private nint _system;
    private nint _services;
    private bool _tried;

    public AdlxFrameRateBackend(Action<string>? log = null) => _log = log;

    public GpuVendor Vendor => GpuVendor.Amd;

    public DriverProbe Probe(GpuAdapter adapter)
    {
        ArgumentNullException.ThrowIfNull(adapter);
        lock (_gate)
        {
            return ProbeLocked(adapter).Probe;
        }
    }

    public int? Read(GpuAdapter adapter, string? exeName)
    {
        ArgumentNullException.ThrowIfNull(adapter);
        lock (_gate)
        {
            return CurrentState(adapter) is { } state ? (state.Enabled ? state.MaxFps : 0) : null;
        }
    }

    public string? Capture(GpuAdapter adapter, string? exeName)
    {
        ArgumentNullException.ThrowIfNull(adapter);
        lock (_gate)
        {
            return CurrentState(adapter)?.Serialize();
        }
    }

    public bool Apply(GpuAdapter adapter, string? exeName, int fps)
    {
        ArgumentNullException.ThrowIfNull(adapter);
        lock (_gate)
        {
            var (probe, method) = ProbeLocked(adapter);
            if (probe.Availability != FrameRateLimitAvailability.Available)
            {
                return false;
            }

            return WithFeature(adapter, method, feature =>
            {
                if (method == AmdLimitMethod.Frtc)
                {
                    return Ok(Call(feature, FrtcSetFps, fps)) && Ok(CallBool(feature, FrtcSetEnabled, true));
                }

                // Chill com mínimo = máximo: o jogo fica preso no limite (o Chill não baixa mais o FPS parado).
                int currentMin = ReadInt(feature, ChillGetMin) ?? probe.MinFps;
                bool ok = SetChill(feature, currentMin, fps, fps);
                return ok && Ok(CallBool(feature, ChillSetEnabled, true));
            });
        }
    }

    public bool Restore(GpuAdapter adapter, string? exeName, string state)
    {
        ArgumentNullException.ThrowIfNull(adapter);
        if (AmdLimitState.Parse(state) is not { } before)
        {
            return false;
        }

        lock (_gate)
        {
            return WithFeature(adapter, before.Method, feature =>
            {
                if (before.Method == AmdLimitMethod.Frtc)
                {
                    bool value = before.MaxFps <= 0 || Ok(Call(feature, FrtcSetFps, before.MaxFps));
                    return Ok(CallBool(feature, FrtcSetEnabled, before.Enabled)) && value;
                }

                int currentMin = ReadInt(feature, ChillGetMin) ?? before.MinFps;
                bool values = before.MaxFps <= 0 || SetChill(feature, currentMin, before.MinFps, before.MaxFps);
                return Ok(CallBool(feature, ChillSetEnabled, before.Enabled)) && values;
            });
        }
    }

    private static bool Ok(int result) => result is >= 0 and <= AlreadyInitialized;

    private static nint* Vtable(nint obj) => *(nint**)obj;

    private static int Call(nint obj, int index, int value) => ((delegate* unmanaged<nint, int, int>)Vtable(obj)[index])(obj, value);

    private static int CallBool(nint obj, int index, bool value) => ((delegate* unmanaged<nint, byte, int>)Vtable(obj)[index])(obj, value ? (byte)1 : (byte)0);

    private static int? ReadInt(nint obj, int index)
    {
        int value = 0;
        return Ok(((delegate* unmanaged<nint, int*, int>)Vtable(obj)[index])(obj, &value)) ? value : null;
    }

    private static bool? ReadBool(nint obj, int index)
    {
        byte value = 0;
        return Ok(((delegate* unmanaged<nint, byte*, int>)Vtable(obj)[index])(obj, &value)) ? value != 0 : null;
    }

    private static (int Min, int Max)? ReadRange(nint obj)
    {
        int* range = stackalloc int[3]; // ADLX_IntRange: minValue, maxValue, step
        return Ok(((delegate* unmanaged<nint, int*, int>)Vtable(obj)[FeatureGetRange])(obj, range)) ? (range[0], range[1]) : null;
    }

    private static void ReleaseObject(nint obj)
    {
        if (obj != 0)
        {
            _ = ((delegate* unmanaged<nint, int>)Vtable(obj)[Release])(obj);
        }
    }

    /// <summary>Troca mínimo e máximo do Chill sem nunca deixar o máximo abaixo do mínimo em vigor.</summary>
    private static bool SetChill(nint chill, int currentMin, int newMin, int newMax)
    {
        if (AmdLimitState.ChillSetsMaxFirst(currentMin, newMax))
        {
            bool max = Ok(Call(chill, ChillSetMax, newMax));
            _ = Call(chill, ChillSetMin, newMin); // alguns drivers pedem mínimo < máximo: o máximo já limita
            return max;
        }

        _ = Call(chill, ChillSetMin, newMin);
        return Ok(Call(chill, ChillSetMax, newMax));
    }

    private (DriverProbe Probe, AmdLimitMethod Method) ProbeLocked(GpuAdapter adapter)
    {
        if (_probes.TryGetValue(adapter.Name, out var cached))
        {
            return cached;
        }

        var result = (DriverProbe.Missing, AmdLimitMethod.Frtc);
        if (Open())
        {
            result = (DriverProbe.Missing with { Availability = FrameRateLimitAvailability.Unsupported }, AmdLimitMethod.Frtc);
            foreach (var (method, name) in new[] { (AmdLimitMethod.Frtc, "FRTC"), (AmdLimitMethod.Chill, "Radeon Chill") })
            {
                var probe = WithFeature(adapter, method, feature =>
                    ReadBool(feature, FeatureIsSupported) == true && ReadRange(feature) is { } range
                        ? new DriverProbe(FrameRateLimitAvailability.Available, Math.Max(1, range.Min), Math.Max(range.Min, range.Max), PerApplication: false, name)
                        : null);
                if (probe is not null)
                {
                    result = (probe, method);
                    break;
                }
            }
        }

        _probes[adapter.Name] = result;
        _log?.Invoke($"ADLX: {adapter.Name}: {result.Item1.Availability} {result.Item1.Method} {result.Item1.MinFps}–{result.Item1.MaxFps}");
        return result;
    }

    private AmdLimitState? CurrentState(GpuAdapter adapter)
    {
        var (probe, method) = ProbeLocked(adapter);
        if (probe.Availability != FrameRateLimitAvailability.Available)
        {
            return null;
        }

        return WithFeature(adapter, method, feature =>
        {
            bool? enabled = ReadBool(feature, FeatureIsEnabled);
            if (enabled is null)
            {
                return null;
            }

            return method == AmdLimitMethod.Frtc
                ? ReadInt(feature, FrtcGetFps) is { } fps ? new AmdLimitState(method, enabled.Value, 0, fps) : null
                : ReadInt(feature, ChillGetMin) is { } min && ReadInt(feature, ChillGetMax) is { } max ? new AmdLimitState(method, enabled.Value, min, max) : null;
        });
    }

    private bool Open()
    {
        if (_tried)
        {
            return _services != 0;
        }

        _tried = true;
        if (!NativeLibrary.TryLoad(Library, typeof(AdlxFrameRateBackend).Assembly, DllImportSearchPath.System32, out nint library)
            || !NativeLibrary.TryGetExport(library, "ADLXQueryFullVersion", out nint queryVersion)
            || !NativeLibrary.TryGetExport(library, "ADLXInitialize", out nint initialize))
        {
            return false; // sem driver AMD (ou Adrenalin antigo, sem ADLX)
        }

        // Inicializa pedindo a versão que o próprio driver tem: as interfaces usadas existem desde o ADLX 1.0.
        ulong version = 0;
        if (!Ok(((delegate* unmanaged[Cdecl]<ulong*, int>)queryVersion)(&version)) || version == 0)
        {
            version = 1UL << 48;
        }

        nint system = 0;
        int result = ((delegate* unmanaged[Cdecl]<ulong, nint*, int>)initialize)(version, &system);
        if (!Ok(result) || system == 0)
        {
            _log?.Invoke($"ADLX: ADLXInitialize falhou ({result})");
            return false;
        }

        nint services = 0;
        result = ((delegate* unmanaged<nint, nint*, int>)Vtable(system)[SystemGet3DSettingsServices])(system, &services);
        if (!Ok(result) || services == 0)
        {
            _log?.Invoke($"ADLX: serviços de 3D indisponíveis ({result})");
            return false;
        }

        // Sistema e serviços ficam abertos até o fim do processo, como a biblioteca do RTSS ficava.
        _system = system;
        _services = services;
        return true;
    }

    /// <summary>A GPU do ADLX que corresponde à placa (pelo nome); o chamador libera. Zero = não achou.</summary>
    private nint FindGpu(GpuAdapter adapter)
    {
        nint list = 0;
        if (!Ok(((delegate* unmanaged<nint, nint*, int>)Vtable(_system)[SystemGetGpus])(_system, &list)) || list == 0)
        {
            return 0;
        }

        try
        {
            uint count = ((delegate* unmanaged<nint, uint>)Vtable(list)[ListSize])(list);
            var gpus = new List<nint>();
            var names = new List<string?>();
            for (uint i = 0; i < count; i++)
            {
                nint gpu = 0;
                if (Ok(((delegate* unmanaged<nint, uint, nint*, int>)Vtable(list)[ListAtGpu])(list, i, &gpu)) && gpu != 0)
                {
                    byte* name = null;
                    _ = ((delegate* unmanaged<nint, byte**, int>)Vtable(gpu)[GpuName])(gpu, &name);
                    gpus.Add(gpu);
                    names.Add(name == null ? null : Marshal.PtrToStringUTF8((nint)name));
                }
            }

            int index = GpuSelection.MatchByName(names, adapter.Name);
            for (int i = 0; i < gpus.Count; i++)
            {
                if (i != index)
                {
                    ReleaseObject(gpus[i]);
                }
            }

            return index >= 0 ? gpus[index] : 0;
        }
        finally
        {
            ReleaseObject(list);
        }
    }

    /// <summary>Abre o FRTC ou o Chill da placa, roda <paramref name="work"/> e libera tudo. Default = não abriu.</summary>
    private T? WithFeature<T>(GpuAdapter adapter, AmdLimitMethod method, Func<nint, T?> work)
    {
        if (!Open())
        {
            return default;
        }

        nint gpu = FindGpu(adapter);
        if (gpu == 0)
        {
            return default;
        }

        nint feature = 0;
        try
        {
            int slot = method == AmdLimitMethod.Frtc ? ServicesGetFrtc : ServicesGetChill;
            int result = ((delegate* unmanaged<nint, nint, nint*, int>)Vtable(_services)[slot])(_services, gpu, &feature);
            return Ok(result) && feature != 0 ? work(feature) : default;
        }
        finally
        {
            ReleaseObject(feature);
            ReleaseObject(gpu);
        }
    }
}
