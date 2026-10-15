using System.Globalization;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>O que fazer para devolver o perfil NVIDIA como estava.</summary>
public enum NvidiaRestoreAction
{
    /// <summary>O perfil foi criado pelo Tako: apaga o perfil inteiro.</summary>
    DeleteProfile,

    /// <summary>O perfil já existia sem o limite próprio (herdava do global): apaga só a configuração.</summary>
    DeleteSetting,

    /// <summary>O perfil já tinha um limite próprio: volta o valor.</summary>
    SetValue,
}

/// <summary>
/// Limite NVIDIA antes do Tako, no perfil de aplicativo do jogo (DRS, configuração <c>FRL_FPS</c> = "Max Frame Rate"):
/// se o perfil existia, se o limite estava no próprio perfil e qual era.
/// </summary>
public sealed record NvidiaLimitState(bool ProfileExisted, bool SettingOnProfile, uint Value)
{
    /// <summary>Id do "Max Frame Rate" (Frame Rate Limiter v3) no NvApiDriverSettings.h do NVAPI SDK.</summary>
    public const uint FrameRateLimitSettingId = 0x10835002;

    /// <summary>Maior valor aceito pela configuração (<c>FRL_FPS_MAX</c>); 0 = desligado.</summary>
    public const int MaxFps = 0x3FF;

    private const string Tag = "nv";

    public NvidiaRestoreAction RestoreAction =>
        !ProfileExisted ? NvidiaRestoreAction.DeleteProfile
        : !SettingOnProfile ? NvidiaRestoreAction.DeleteSetting
        : NvidiaRestoreAction.SetValue;

    /// <summary>Nome do perfil que o Tako cria quando o jogo não está em nenhum perfil do driver.</summary>
    public static string ProfileName(string exeName) => $"Tako · {exeName}";

    public string Serialize() => string.Create(CultureInfo.InvariantCulture, $"{Tag};{Bit(ProfileExisted)};{Bit(SettingOnProfile)};{Value}");

    public static NvidiaLimitState? Parse(string? text)
    {
        string[] parts = Split(text, Tag, 4);
        return parts.Length == 4 && TryBit(parts[1], out bool existed) && TryBit(parts[2], out bool onProfile)
            && uint.TryParse(parts[3], NumberStyles.None, CultureInfo.InvariantCulture, out uint value)
            ? new NvidiaLimitState(existed, onProfile, value)
            : null;
    }

    internal static string Bit(bool value) => value ? "1" : "0";

    internal static bool TryBit(string text, out bool value)
    {
        value = text == "1";
        return text is "0" or "1";
    }

    internal static string[] Split(string? text, string tag, int count)
    {
        string[] parts = (text ?? string.Empty).Split(';');
        return parts.Length == count && parts[0] == tag ? parts : [];
    }
}

/// <summary>Qual recurso do Adrenalin limita o FPS: FRTC (alvo fixo) ou Radeon Chill (mínimo e máximo).</summary>
public enum AmdLimitMethod
{
    Frtc,
    Chill,
}

/// <summary>Limite AMD (ADLX, global da placa) antes do Tako. No FRTC só <see cref="MaxFps"/> vale.</summary>
public sealed record AmdLimitState(AmdLimitMethod Method, bool Enabled, int MinFps, int MaxFps)
{
    private const string Tag = "amd";

    public string Serialize() => string.Create(
        CultureInfo.InvariantCulture,
        $"{Tag};{(Method == AmdLimitMethod.Frtc ? "frtc" : "chill")};{NvidiaLimitState.Bit(Enabled)};{MinFps};{MaxFps}");

    public static AmdLimitState? Parse(string? text)
    {
        string[] parts = NvidiaLimitState.Split(text, Tag, 5);
        if (parts.Length != 5 || !NvidiaLimitState.TryBit(parts[2], out bool enabled)
            || !int.TryParse(parts[3], NumberStyles.Integer, CultureInfo.InvariantCulture, out int min)
            || !int.TryParse(parts[4], NumberStyles.Integer, CultureInfo.InvariantCulture, out int max))
        {
            return null;
        }

        return parts[1] switch
        {
            "frtc" => new AmdLimitState(AmdLimitMethod.Frtc, enabled, min, max),
            "chill" => new AmdLimitState(AmdLimitMethod.Chill, enabled, min, max),
            _ => null,
        };
    }

    /// <summary>
    /// Ordem para trocar mínimo e máximo do Chill sem nunca deixar o máximo abaixo do mínimo em vigor (o driver recusa):
    /// verdadeiro = muda o máximo primeiro.
    /// </summary>
    public static bool ChillSetsMaxFirst(int currentMin, int newMax) => newMax >= currentMin;
}

/// <summary>Limite Intel (IGCL, <c>CTL_3D_FEATURE_FRAME_LIMIT</c>) antes do Tako: por aplicativo ou global.</summary>
public sealed record IntelLimitState(bool PerApplication, bool Enabled, int Value)
{
    /// <summary><c>CTL_3D_FEATURE_FRAME_LIMIT</c> em <c>ctl_3d_feature_t</c>.</summary>
    public const int FrameLimitFeature = 2;

    private const string Tag = "igcl";

    public string Serialize() => string.Create(
        CultureInfo.InvariantCulture,
        $"{Tag};{NvidiaLimitState.Bit(PerApplication)};{NvidiaLimitState.Bit(Enabled)};{Value}");

    public static IntelLimitState? Parse(string? text)
    {
        string[] parts = NvidiaLimitState.Split(text, Tag, 4);
        return parts.Length == 4 && NvidiaLimitState.TryBit(parts[1], out bool perApp) && NvidiaLimitState.TryBit(parts[2], out bool enabled)
            && int.TryParse(parts[3], NumberStyles.Integer, CultureInfo.InvariantCulture, out int value)
            ? new IntelLimitState(perApp, enabled, value)
            : null;
    }
}
