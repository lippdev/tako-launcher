using System.Runtime.InteropServices;
using ConsoleMode.Runtime.SessionTools;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Limite de FPS da NVIDIA pelo banco de perfis do driver (DRS) do NVAPI, o mesmo "Max Frame Rate" do Painel de
/// Controle NVIDIA, no perfil de aplicativo do jogo (criado como "Tako · jogo.exe" quando o jogo não tem perfil).
/// <para>
/// A <c>nvapi64.dll</c> vem com o driver; é carregada sob demanda e as funções vêm de <c>nvapi_QueryInterface</c> pelos
/// ids públicos do NVAPI SDK (github.com/NVIDIA/nvapi, MIT: <c>nvapi_interface.h</c>, estruturas do <c>nvapi.h</c>,
/// configuração <c>FRL_FPS_ID</c> do <c>NvApiDriverSettings.h</c>). Nada é copiado do SDK além de ids e layout.
/// </para>
/// <para>
/// Gravar no DRS pode exigir administrador conforme o driver; se o NVAPI responder <c>NVAPI_INVALID_USER_PRIVILEGE</c>,
/// o limite passa a "indisponível sem permissão" e o Tako não pede elevação (ADR-0018).
/// </para>
/// </summary>
public sealed unsafe class NvapiFrameRateBackend : IDriverFrameRateBackend
{
    private const string Library = "nvapi64.dll";

    // Ids de nvapi_interface.h.
    private const uint IdInitialize = 0x0150E828;
    private const uint IdCreateSession = 0x0694D52E;
    private const uint IdDestroySession = 0xDAD9CFF8;
    private const uint IdLoadSettings = 0x375DBD6B;
    private const uint IdSaveSettings = 0xFCBC7E14;
    private const uint IdCreateProfile = 0xCC176068;
    private const uint IdDeleteProfile = 0x17093206;
    private const uint IdGetCurrentGlobalProfile = 0x617BFF9F;
    private const uint IdFindProfileByName = 0x7E4A9A0B;
    private const uint IdGetProfileInfo = 0x61CD6FD6;
    private const uint IdCreateApplication = 0x4347A9DE;
    private const uint IdFindApplicationByName = 0xEEE566B2;
    private const uint IdSetSetting = 0x577DD202;
    private const uint IdGetSetting = 0x73BF8338;
    private const uint IdDeleteProfileSetting = 0xE4A26362;

    // NvAPI_Status (nvapi_lite_common.h).
    private const int Ok = 0;
    private const int InvalidUserPrivilege = -137;
    private const int SettingNotFound = -160;
    private const int ProfileNotFound = -163;
    private const int ProfileNameInUse = -164;
    private const int ExecutableNotFound = -166;

    // Estruturas do nvapi.h: NvAPI_UnicodeString = NvU16[2048]; versão = sizeof | (n << 16).
    private const int UnicodeChars = 2048;
    private const int UnicodeBytes = UnicodeChars * 2;
    private const int SettingSize = 12320; // NVDRS_SETTING_V1 (#pragma pack 4)
    private const int SettingIdOffset = 4 + UnicodeBytes;
    private const int SettingTypeOffset = SettingIdOffset + 4;
    private const int SettingLocationOffset = SettingTypeOffset + 4;
    private const int SettingCurrentValueOffset = SettingLocationOffset + 12 + 4100; // depois de isCurrentPredefined, isPredefinedValid e do valor predefinido
    private const int ApplicationSize = 4 + 4 + (UnicodeBytes * 4) + 4 + UnicodeBytes; // NVDRS_APPLICATION_V4
    private const int ApplicationNameOffset = 8;
    private const int ApplicationFriendlyNameOffset = ApplicationNameOffset + UnicodeBytes;
    private const int ProfileSize = 4 + UnicodeBytes + 4 + 12; // NVDRS_PROFILE_V1
    private const int ProfileNameOffset = 4;
    private const uint SettingVersion = SettingSize | (1u << 16);
    private const uint ApplicationVersion = ApplicationSize | (4u << 16);
    private const uint ProfileVersion = ProfileSize | (1u << 16);
    private const int CurrentProfileLocation = 0; // NVDRS_CURRENT_PROFILE_LOCATION
    private const int DwordType = 0; // NVDRS_DWORD_TYPE

    private readonly object _gate = new();
    private readonly Action<string>? _log;
    private Api? _api;
    private bool _tried;
    private bool _needsAdmin;

    public NvapiFrameRateBackend(Action<string>? log = null) => _log = log;

    public GpuVendor Vendor => GpuVendor.Nvidia;

    public DriverProbe Probe(GpuAdapter adapter)
    {
        lock (_gate)
        {
            if (Open() is null)
            {
                return DriverProbe.Missing;
            }

            var availability = _needsAdmin ? FrameRateLimitAvailability.NeedsAdmin : FrameRateLimitAvailability.Available;
            return new DriverProbe(availability, 1, NvidiaLimitState.MaxFps, PerApplication: true, "Max Frame Rate");
        }
    }

    public int? Read(GpuAdapter adapter, string? exeName)
    {
        lock (_gate)
        {
            return WithSession<int?>(false, (api, session) =>
            {
                nint profile = 0;
                if (exeName is null || !api.FindApplication(session, exeName, out profile))
                {
                    if (api.GetCurrentGlobalProfile(session, &profile) != Ok)
                    {
                        return null;
                    }
                }

                var (status, _, value) = api.GetFrameRateLimit(session, profile);
                return status == Ok ? (int)value : status == SettingNotFound ? 0 : (int?)null;
            });
        }
    }

    public string? Capture(GpuAdapter adapter, string? exeName)
    {
        if (exeName is null)
        {
            return null;
        }

        lock (_gate)
        {
            return WithSession<string>(false, (api, session) =>
            {
                if (!api.FindApplication(session, exeName, out nint profile))
                {
                    return new NvidiaLimitState(false, false, 0).Serialize();
                }

                var (status, location, value) = api.GetFrameRateLimit(session, profile);
                return status switch
                {
                    Ok => new NvidiaLimitState(true, location == CurrentProfileLocation, location == CurrentProfileLocation ? value : 0).Serialize(),
                    SettingNotFound => new NvidiaLimitState(true, false, 0).Serialize(),
                    _ => null,
                };
            });
        }
    }

    public bool Apply(GpuAdapter adapter, string? exeName, int fps)
    {
        if (exeName is null)
        {
            return false;
        }

        lock (_gate)
        {
            return WithSession<bool>(true, (api, session) =>
            {
                if (!api.FindApplication(session, exeName, out nint profile) && !api.CreateProfileFor(session, exeName, out profile))
                {
                    return false;
                }

                return api.SetFrameRateLimit(session, profile, (uint)Math.Clamp(fps, 0, NvidiaLimitState.MaxFps)) == Ok;
            }) == true;
        }
    }

    public bool Restore(GpuAdapter adapter, string? exeName, string state)
    {
        if (exeName is null || NvidiaLimitState.Parse(state) is not { } before)
        {
            return false;
        }

        lock (_gate)
        {
            return WithSession<bool>(true, (api, session) =>
            {
                if (!api.FindApplication(session, exeName, out nint profile))
                {
                    return true; // o perfil já não existe (apagado pelo usuário): nada a devolver
                }

                var action = before.RestoreAction;

                // Só apaga um perfil que tenha o nome que o Tako dá; um perfil do usuário com o jogo perde só o limite.
                if (action == NvidiaRestoreAction.DeleteProfile && !api.ProfileNameIs(session, profile, NvidiaLimitState.ProfileName(exeName)))
                {
                    action = NvidiaRestoreAction.DeleteSetting;
                }

                int status = action switch
                {
                    NvidiaRestoreAction.DeleteProfile => api.DeleteProfile(session, profile),
                    NvidiaRestoreAction.DeleteSetting => api.DeleteFrameRateLimit(session, profile),
                    _ => api.SetFrameRateLimit(session, profile, before.Value),
                };
                return status is Ok or SettingNotFound or ProfileNotFound;
            }) == true;
        }
    }

    private Api? Open()
    {
        if (_tried)
        {
            return _api;
        }

        _tried = true;
        if (!NativeLibrary.TryLoad(Library, typeof(NvapiFrameRateBackend).Assembly, DllImportSearchPath.System32, out nint library)
            || !NativeLibrary.TryGetExport(library, "nvapi_QueryInterface", out nint query))
        {
            return null; // sem driver NVIDIA
        }

        _api = Api.TryCreate((delegate* unmanaged[Cdecl]<uint, nint>)query, _log);
        return _api;
    }

    /// <summary>Abre uma sessão do DRS, carrega, roda <paramref name="work"/> e grava se pedido. Nulo = não deu.</summary>
    private T? WithSession<T>(bool save, Func<Api, nint, T?> work)
    {
        if (Open() is not { } api || (save && _needsAdmin))
        {
            return default;
        }

        nint session = 0;
        int status = api.CreateSession(&session);
        if (status != Ok)
        {
            _log?.Invoke($"NVAPI: sessão do DRS recusada ({status})");
            return default;
        }

        try
        {
            if ((status = api.LoadSettings(session)) != Ok)
            {
                _log?.Invoke($"NVAPI: perfis não carregaram ({status})");
                return default;
            }

            var result = work(api, session);
            if (save && result is true)
            {
                status = api.SaveSettings(session);
                if (status == InvalidUserPrivilege)
                {
                    _needsAdmin = true;
                    _log?.Invoke("NVAPI: gravar perfis exige administrador; limite indisponível");
                    return default;
                }

                if (status != Ok)
                {
                    _log?.Invoke($"NVAPI: perfis não gravaram ({status})");
                    return default;
                }
            }

            return result;
        }
        finally
        {
            api.DestroySession(session);
        }
    }

    /// <summary>Funções do NVAPI usadas aqui, vindas de <c>nvapi_QueryInterface</c>. A biblioteca fica carregada até o fim do processo.</summary>
    private sealed class Api
    {
        private readonly delegate* unmanaged[Cdecl]<nint*, int> _createSession;
        private readonly delegate* unmanaged[Cdecl]<nint, int> _destroySession;
        private readonly delegate* unmanaged[Cdecl]<nint, int> _loadSettings;
        private readonly delegate* unmanaged[Cdecl]<nint, int> _saveSettings;
        private readonly delegate* unmanaged[Cdecl]<nint, byte*, nint*, int> _createProfile;
        private readonly delegate* unmanaged[Cdecl]<nint, nint, int> _deleteProfile;
        private readonly delegate* unmanaged[Cdecl]<nint, nint*, int> _getCurrentGlobalProfile;
        private readonly delegate* unmanaged[Cdecl]<nint, char*, nint*, int> _findProfileByName;
        private readonly delegate* unmanaged[Cdecl]<nint, nint, byte*, int> _getProfileInfo;
        private readonly delegate* unmanaged[Cdecl]<nint, nint, byte*, int> _createApplication;
        private readonly delegate* unmanaged[Cdecl]<nint, char*, nint*, byte*, int> _findApplicationByName;
        private readonly delegate* unmanaged[Cdecl]<nint, nint, byte*, int> _setSetting;
        private readonly delegate* unmanaged[Cdecl]<nint, nint, uint, byte*, int> _getSetting;
        private readonly delegate* unmanaged[Cdecl]<nint, nint, uint, int> _deleteProfileSetting;

        private Api(nint[] f)
        {
            _createSession = (delegate* unmanaged[Cdecl]<nint*, int>)f[0];
            _destroySession = (delegate* unmanaged[Cdecl]<nint, int>)f[1];
            _loadSettings = (delegate* unmanaged[Cdecl]<nint, int>)f[2];
            _saveSettings = (delegate* unmanaged[Cdecl]<nint, int>)f[3];
            _createProfile = (delegate* unmanaged[Cdecl]<nint, byte*, nint*, int>)f[4];
            _deleteProfile = (delegate* unmanaged[Cdecl]<nint, nint, int>)f[5];
            _getCurrentGlobalProfile = (delegate* unmanaged[Cdecl]<nint, nint*, int>)f[6];
            _findProfileByName = (delegate* unmanaged[Cdecl]<nint, char*, nint*, int>)f[7];
            _getProfileInfo = (delegate* unmanaged[Cdecl]<nint, nint, byte*, int>)f[8];
            _createApplication = (delegate* unmanaged[Cdecl]<nint, nint, byte*, int>)f[9];
            _findApplicationByName = (delegate* unmanaged[Cdecl]<nint, char*, nint*, byte*, int>)f[10];
            _setSetting = (delegate* unmanaged[Cdecl]<nint, nint, byte*, int>)f[11];
            _getSetting = (delegate* unmanaged[Cdecl]<nint, nint, uint, byte*, int>)f[12];
            _deleteProfileSetting = (delegate* unmanaged[Cdecl]<nint, nint, uint, int>)f[13];
        }

        public static Api? TryCreate(delegate* unmanaged[Cdecl]<uint, nint> query, Action<string>? log)
        {
            nint initialize = query(IdInitialize);
            if (initialize == 0)
            {
                return null;
            }

            int status = ((delegate* unmanaged[Cdecl]<int>)initialize)();
            if (status != Ok)
            {
                log?.Invoke($"NVAPI: NvAPI_Initialize falhou ({status})");
                return null;
            }

            uint[] ids =
            [
                IdCreateSession, IdDestroySession, IdLoadSettings, IdSaveSettings, IdCreateProfile, IdDeleteProfile,
                IdGetCurrentGlobalProfile, IdFindProfileByName, IdGetProfileInfo, IdCreateApplication, IdFindApplicationByName,
                IdSetSetting, IdGetSetting, IdDeleteProfileSetting,
            ];
            nint[] functions = [.. ids.Select(id => query(id))];
            if (functions.Any(f => f == 0))
            {
                log?.Invoke("NVAPI: driver sem as funções de perfis (DRS)");
                return null;
            }

            return new Api(functions);
        }

        public int CreateSession(nint* session) => _createSession(session);

        public void DestroySession(nint session) => _destroySession(session);

        public int LoadSettings(nint session) => _loadSettings(session);

        public int SaveSettings(nint session) => _saveSettings(session);

        public int DeleteProfile(nint session, nint profile) => _deleteProfile(session, profile);

        public int GetCurrentGlobalProfile(nint session, nint* profile) => _getCurrentGlobalProfile(session, profile);

        public bool FindApplication(nint session, string exeName, out nint profile)
        {
            profile = 0;
            using var name = new NativeBuffer(UnicodeBytes);
            using var app = new NativeBuffer(ApplicationSize);
            name.WriteString(0, exeName, UnicodeChars);
            *(uint*)app.Pointer = ApplicationVersion;
            nint found = 0;
            int status = _findApplicationByName(session, (char*)name.Pointer, &found, app.Pointer);
            profile = found;
            return status == Ok && found != 0;
        }

        /// <summary>Cria o perfil "Tako · jogo.exe" com o jogo dentro (ou reaproveita o de uma execução anterior).</summary>
        public bool CreateProfileFor(nint session, string exeName, out nint profile)
        {
            profile = 0;
            string profileName = NvidiaLimitState.ProfileName(exeName);
            nint created = 0;
            using (var info = new NativeBuffer(ProfileSize))
            {
                *(uint*)info.Pointer = ProfileVersion;
                info.WriteString(ProfileNameOffset, profileName, UnicodeChars);
                int status = _createProfile(session, info.Pointer, &created);
                if (status == ProfileNameInUse)
                {
                    using var name = new NativeBuffer(UnicodeBytes);
                    name.WriteString(0, profileName, UnicodeChars);
                    status = _findProfileByName(session, (char*)name.Pointer, &created);
                }

                if (status != Ok || created == 0)
                {
                    return false;
                }
            }

            using (var app = new NativeBuffer(ApplicationSize))
            {
                *(uint*)app.Pointer = ApplicationVersion;
                app.WriteString(ApplicationNameOffset, exeName, UnicodeChars);
                app.WriteString(ApplicationFriendlyNameOffset, exeName, UnicodeChars);
                if (_createApplication(session, created, app.Pointer) != Ok)
                {
                    return false;
                }
            }

            profile = created;
            return true;
        }

        public bool ProfileNameIs(nint session, nint profile, string expected)
        {
            using var info = new NativeBuffer(ProfileSize);
            *(uint*)info.Pointer = ProfileVersion;
            return _getProfileInfo(session, profile, info.Pointer) == Ok
                && string.Equals(info.ReadString(ProfileNameOffset, UnicodeChars), expected, StringComparison.Ordinal);
        }

        public (int Status, int Location, uint Value) GetFrameRateLimit(nint session, nint profile)
        {
            using var setting = new NativeBuffer(SettingSize);
            *(uint*)setting.Pointer = SettingVersion;
            int status = _getSetting(session, profile, NvidiaLimitState.FrameRateLimitSettingId, setting.Pointer);
            return status == Ok
                ? (status, *(int*)(setting.Pointer + SettingLocationOffset), *(uint*)(setting.Pointer + SettingCurrentValueOffset))
                : (status, -1, 0);
        }

        public int SetFrameRateLimit(nint session, nint profile, uint fps)
        {
            using var setting = new NativeBuffer(SettingSize);
            *(uint*)setting.Pointer = SettingVersion;
            *(uint*)(setting.Pointer + SettingIdOffset) = NvidiaLimitState.FrameRateLimitSettingId;
            *(int*)(setting.Pointer + SettingTypeOffset) = DwordType;
            *(uint*)(setting.Pointer + SettingCurrentValueOffset) = fps;
            return _setSetting(session, profile, setting.Pointer);
        }

        public int DeleteFrameRateLimit(nint session, nint profile) =>
            _deleteProfileSetting(session, profile, NvidiaLimitState.FrameRateLimitSettingId);
    }
}

/// <summary>Bloco de memória nativa zerado para as estruturas grandes do NVAPI (dezenas de KB, longe da pilha).</summary>
internal sealed unsafe class NativeBuffer : IDisposable
{
    private readonly int _size;

    public NativeBuffer(int size)
    {
        _size = size;
        Pointer = (byte*)NativeMemory.AllocZeroed((nuint)size);
    }

    public byte* Pointer { get; private set; }

    /// <summary>Texto UTF-16 terminado em zero, cortado em <paramref name="maxChars"/> − 1.</summary>
    public void WriteString(int offset, string text, int maxChars)
    {
        int length = Math.Min(text.Length, Math.Min(maxChars, (_size - offset) / 2) - 1);
        var target = new Span<char>(Pointer + offset, length + 1);
        text.AsSpan(0, length).CopyTo(target);
        target[length] = '\0';
    }

    public string ReadString(int offset, int maxChars)
    {
        var span = new ReadOnlySpan<char>(Pointer + offset, Math.Min(maxChars, (_size - offset) / 2));
        int end = span.IndexOf('\0');
        return new string(end >= 0 ? span[..end] : span);
    }

    public void Dispose()
    {
        if (Pointer != null)
        {
            NativeMemory.Free(Pointer);
            Pointer = null;
        }
    }
}
