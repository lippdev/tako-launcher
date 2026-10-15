using System.Runtime.InteropServices;
using System.Text;
using ConsoleMode.Runtime.SessionTools;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Limite de FPS da Intel pela Intel Graphics Control Library (<c>ControlLib.dll</c>, instalada com o driver Arc/Iris Xe):
/// recurso 3D <c>CTL_3D_FEATURE_FRAME_LIMIT</c>, por aplicativo quando o driver diz que aceita (<c>PerAppSupport</c>),
/// senão global da placa.
/// <para>
/// As estruturas abaixo foram escritas à mão a partir da documentação pública do IGCL (o header da Intel tem licença
/// própria e não é copiado nem distribuído). Com mais de uma placa Intel (integrada + Arc), o limite vai para todas: o
/// IGCL não diz qual placa do DXGI é cada uma sem estruturas maiores, e o valor de antes volta em todas.
/// </para>
/// </summary>
public sealed unsafe class IgclFrameRateBackend : IDriverFrameRateBackend
{
    private const string Library = "ControlLib.dll";
    private const uint ApiVersion = (1u << 16) | 1u; // CTL_MAKE_VERSION(1, 1)
    private const int Success = 0;
    private const int InsufficientPermissions = 0x40000006;
    private const int ValueTypeInt32 = 2; // CTL_PROPERTY_VALUE_TYPE_INT32

    private readonly object _gate = new();
    private readonly Action<string>? _log;
    private Api? _api;
    private nint[] _devices = [];
    private DriverProbe? _probe;
    private bool _tried;
    private bool _needsAdmin;

    public IgclFrameRateBackend(Action<string>? log = null) => _log = log;

    public GpuVendor Vendor => GpuVendor.Intel;

    public DriverProbe Probe(GpuAdapter adapter)
    {
        lock (_gate)
        {
            var probe = ProbeLocked();
            return _needsAdmin ? probe with { Availability = FrameRateLimitAvailability.NeedsAdmin } : probe;
        }
    }

    public int? Read(GpuAdapter adapter, string? exeName)
    {
        lock (_gate)
        {
            return Get(Scope(exeName)) is { } state ? (state.Enabled ? state.Value : 0) : null;
        }
    }

    public string? Capture(GpuAdapter adapter, string? exeName)
    {
        lock (_gate)
        {
            return Get(Scope(exeName))?.Serialize();
        }
    }

    public bool Apply(GpuAdapter adapter, string? exeName, int fps)
    {
        lock (_gate)
        {
            return ProbeLocked().Availability == FrameRateLimitAvailability.Available && Set(Scope(exeName), fps > 0, fps);
        }
    }

    public bool Restore(GpuAdapter adapter, string? exeName, string state)
    {
        if (IntelLimitState.Parse(state) is not { } before)
        {
            return false;
        }

        lock (_gate)
        {
            return Open() is not null && Set(before.PerApplication ? exeName : null, before.Enabled, before.Value);
        }
    }

    private string? Scope(string? exeName) => ProbeLocked().PerApplication ? exeName : null;

    private DriverProbe ProbeLocked()
    {
        if (_probe is { } cached)
        {
            return cached;
        }

        var probe = DriverProbe.Missing;
        if (Open() is { } api)
        {
            probe = DriverProbe.Missing with { Availability = FrameRateLimitAvailability.Unsupported };
            foreach (nint device in _devices)
            {
                if (api.FrameLimitCaps(device) is { } caps)
                {
                    probe = new DriverProbe(FrameRateLimitAvailability.Available, Math.Max(1, caps.Min), Math.Max(caps.Min, caps.Max), caps.PerApp, "Frame Limiter");
                    break;
                }
            }
        }

        _probe = probe;
        _log?.Invoke($"IGCL: {_devices.Length} placa(s) Intel; limite {probe.Availability} {probe.MinFps}–{probe.MaxFps}{(probe.PerApplication ? " por jogo" : " global")}");
        return probe;
    }

    private IntelLimitState? Get(string? exeName)
    {
        if (Open() is not { } api || _devices.Length == 0)
        {
            return null;
        }

        var feature = NewFeature(exeName, set: false, enabled: false, value: 0, out byte[]? name);
        fixed (byte* namePtr = name)
        {
            feature.ApplicationName = (nint)namePtr;
            int result = api.GetSet3DFeature(_devices[0], &feature);
            return result == Success ? new IntelLimitState(exeName is not null, feature.Enable != 0, feature.Value) : null;
        }
    }

    private bool Set(string? exeName, bool enabled, int value)
    {
        if (Open() is not { } api || _devices.Length == 0 || _needsAdmin)
        {
            return false;
        }

        bool all = true;
        foreach (nint device in _devices)
        {
            var feature = NewFeature(exeName, set: true, enabled, value, out byte[]? name);
            fixed (byte* namePtr = name)
            {
                feature.ApplicationName = (nint)namePtr;
                int result = api.GetSet3DFeature(device, &feature);
                if (result == InsufficientPermissions)
                {
                    _needsAdmin = true;
                    _log?.Invoke("IGCL: mudar o limite exige administrador; limite indisponível");
                }

                all &= result == Success;
            }
        }

        return all;
    }

    private static GetSet3DFeature NewFeature(string? exeName, bool set, bool enabled, int value, out byte[]? name)
    {
        // Só o nome do exe, sem pasta; vazio = configuração global da placa.
        name = exeName is null ? null : [.. Encoding.ASCII.GetBytes(exeName.Length > 120 ? exeName[..120] : exeName), 0];
        return new GetSet3DFeature
        {
            Size = (uint)sizeof(GetSet3DFeature),
            FeatureType = IntelLimitState.FrameLimitFeature,
            ApplicationNameLength = (sbyte)(name is null ? 0 : name.Length - 1),
            Set = set ? (byte)1 : (byte)0,
            ValueType = ValueTypeInt32,
            Enable = enabled ? (byte)1 : (byte)0,
            Value = value,
        };
    }

    private Api? Open()
    {
        if (_tried)
        {
            return _api;
        }

        _tried = true;
        if (!NativeLibrary.TryLoad(Library, typeof(IgclFrameRateBackend).Assembly, DllImportSearchPath.System32, out nint library))
        {
            return null; // sem driver Intel com IGCL
        }

        _api = Api.TryCreate(library, _log, out _devices);
        return _api;
    }

    /// <summary><c>ctl_init_args_t</c>.</summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct InitArgs
    {
        public uint Size;
        public byte Version;
        public uint AppVersion;
        public uint Flags;
        public uint SupportedVersion;
        public Guid ApplicationUid;
    }

    /// <summary><c>ctl_3d_feature_getset_t</c> com o valor como <c>ctl_property_int_t</c> (Enable + Value).</summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct GetSet3DFeature
    {
        public uint Size;
        public byte Version;
        public int FeatureType;
        public nint ApplicationName;
        public sbyte ApplicationNameLength;
        public byte Set;
        public int ValueType;
        public byte Enable; // união ctl_property_t: 8 bytes, alinhada em 4
        public int Value;
        public int CustomValueSize;
        public nint CustomValue;
    }

    /// <summary><c>ctl_3d_feature_caps_t</c>.</summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct FeatureCaps
    {
        public uint Size;
        public byte Version;
        public uint NumSupportedFeatures;
        public FeatureDetails* FeatureDetails;
    }

    /// <summary><c>ctl_3d_feature_details_t</c> com a união <c>ctl_property_info_t</c> (24 bytes, alinhada em 8).</summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct FeatureDetails
    {
        public int FeatureType;
        public int ValueType;
        public PropertyInfo Value;
        public int CustomValueSize;
        public nint CustomValue;
        public byte PerAppSupport;
        public long ConflictingFeatures;
        public short FeatureMiscSupport;
        public short Reserved;
        public short Reserved1;
        public short Reserved2;
    }

    /// <summary>União <c>ctl_property_info_t</c> lida como <c>ctl_property_info_int_t</c>.</summary>
    [StructLayout(LayoutKind.Explicit, Size = 24)]
    private struct PropertyInfo
    {
        [FieldOffset(0)]
        public ulong Alignment; // ctl_property_info_enum_t começa com uint64_t

        [FieldOffset(0)]
        public byte DefaultEnable;

        [FieldOffset(4)]
        public int Min;

        [FieldOffset(8)]
        public int Max;

        [FieldOffset(12)]
        public int Step;

        [FieldOffset(16)]
        public int Default;
    }

    private sealed class Api
    {
        private readonly delegate* unmanaged[Cdecl]<nint, FeatureCaps*, int> _caps;
        private readonly delegate* unmanaged[Cdecl]<nint, GetSet3DFeature*, int> _getSet;

        private Api(nint caps, nint getSet)
        {
            _caps = (delegate* unmanaged[Cdecl]<nint, FeatureCaps*, int>)caps;
            _getSet = (delegate* unmanaged[Cdecl]<nint, GetSet3DFeature*, int>)getSet;
        }

        public static Api? TryCreate(nint library, Action<string>? log, out nint[] devices)
        {
            devices = [];
            if (!NativeLibrary.TryGetExport(library, "ctlInit", out nint init)
                || !NativeLibrary.TryGetExport(library, "ctlEnumerateDevices", out nint enumerate)
                || !NativeLibrary.TryGetExport(library, "ctlGetSupported3DCapabilities", out nint caps)
                || !NativeLibrary.TryGetExport(library, "ctlGetSet3DFeature", out nint getSet))
            {
                log?.Invoke("IGCL: ControlLib sem as funções de 3D");
                return null;
            }

            var args = new InitArgs { Size = (uint)sizeof(InitArgs), AppVersion = ApiVersion };
            nint handle = 0;
            int result = ((delegate* unmanaged[Cdecl]<InitArgs*, nint*, int>)init)(&args, &handle);
            if (result != Success || handle == 0)
            {
                log?.Invoke($"IGCL: ctlInit falhou (0x{result:X})");
                return null;
            }

            // A biblioteca fica aberta até o fim do processo (ctlClose não é chamado), como a do RTSS.
            var enumerateFn = (delegate* unmanaged[Cdecl]<nint, uint*, nint*, int>)enumerate;
            uint count = 0;
            if (enumerateFn(handle, &count, null) != Success || count == 0)
            {
                return null;
            }

            var found = new nint[count];
            fixed (nint* list = found)
            {
                if (enumerateFn(handle, &count, list) != Success)
                {
                    return null;
                }
            }

            devices = found[..(int)Math.Min(count, (uint)found.Length)];
            return new Api(caps, getSet);
        }

        public int GetSet3DFeature(nint device, GetSet3DFeature* feature) => _getSet(device, feature);

        /// <summary>Faixa e suporte por aplicativo do limitador; nulo se a placa não tem.</summary>
        public (int Min, int Max, bool PerApp)? FrameLimitCaps(nint device)
        {
            var caps = new FeatureCaps { Size = (uint)sizeof(FeatureCaps) };
            if (_caps(device, &caps) != Success || caps.NumSupportedFeatures == 0 || caps.NumSupportedFeatures > 256)
            {
                return null;
            }

            var details = new FeatureDetails[caps.NumSupportedFeatures];
            fixed (FeatureDetails* list = details)
            {
                caps.FeatureDetails = list;
                if (_caps(device, &caps) != Success)
                {
                    return null;
                }
            }

            foreach (var detail in details)
            {
                if (detail.FeatureType == IntelLimitState.FrameLimitFeature && detail.ValueType == ValueTypeInt32)
                {
                    return (detail.Value.Min, detail.Value.Max, detail.PerAppSupport != 0);
                }
            }

            return null;
        }
    }
}
