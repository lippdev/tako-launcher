// FpsLimits: adaptado de lippdev/consolemode: src/ConsoleMode/Services/SessionMenuMath.cs
using System.Globalization;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>Por que o limite de FPS pode (ou não) ser usado nesta máquina.</summary>
public enum FrameRateLimitAvailability
{
    Available,

    /// <summary>Nenhuma placa de vídeo de verdade encontrada (só o adaptador básico da Microsoft).</summary>
    NoGpu,

    /// <summary>A biblioteca do driver (nvapi64, amdadlx64, ControlLib) não existe ou não abriu.</summary>
    NoDriver,

    /// <summary>O driver abriu, mas não oferece limite de FPS para esta placa.</summary>
    Unsupported,

    /// <summary>O driver só aceita a mudança como administrador; o Tako não pede elevação (ADR-0018).</summary>
    NeedsAdmin,
}

/// <summary>
/// O que o limitador consegue fazer na placa que roda o jogo: fabricante, faixa aceita e se o valor vale só para o jogo
/// (perfil por aplicativo) ou para tudo (global, como o FRTC/Chill da AMD).
/// </summary>
public sealed record FrameRateLimitCapabilities(
    GpuVendor Vendor,
    string? AdapterName,
    FrameRateLimitAvailability Availability,
    int MinFps,
    int MaxFps,
    bool PerApplication,
    string Method)
{
    public static FrameRateLimitCapabilities None { get; } = new(GpuVendor.Unknown, null, FrameRateLimitAvailability.NoGpu, 0, 0, false, string.Empty);

    public bool IsAvailable => Availability == FrameRateLimitAvailability.Available;

    /// <summary>Um valor pedido dentro da faixa do driver (0 continua sendo "sem limite").</summary>
    public int Clamp(int fps)
    {
        if (fps <= 0)
        {
            return 0;
        }

        int low = Math.Max(1, MinFps);
        return Math.Clamp(fps, low, Math.Max(low, MaxFps));
    }

    /// <summary>Frase curta para a Central de Controle explicar por que o limite está indisponível.</summary>
    public string Explanation => Availability switch
    {
        FrameRateLimitAvailability.Available => $"Pelo driver {GpuVendors.Name(Vendor)} ({Method})",
        FrameRateLimitAvailability.NoGpu => "Nenhuma placa de vídeo compatível encontrada",
        FrameRateLimitAvailability.NoDriver => $"Driver {GpuVendors.Name(Vendor)} não encontrado: atualize o driver da placa de vídeo",
        FrameRateLimitAvailability.NeedsAdmin => "Indisponível sem permissão de administrador",
        _ => $"O driver {GpuVendors.Name(Vendor)} desta placa não tem limite de FPS",
    };
}

/// <summary>
/// Limite de FPS pelo driver de vídeo (NVIDIA, AMD ou Intel), sem RTSS nem injeção no jogo (ADR-0018).
/// O valor que o usuário tinha é guardado em arquivo antes da primeira mudança e volta em <see cref="Restore"/>, também
/// na abertura seguinte a uma queda do Tako. Nunca lança: driver ausente ou recusa viram falso.
/// </summary>
public interface IFrameRateLimiter
{
    /// <summary>Placa escolhida e o que o driver dela aceita. A primeira leitura abre o driver (pode levar centenas de ms).</summary>
    FrameRateLimitCapabilities Capabilities { get; }

    /// <summary>Limite em vigor para o jogo (0 = sem limite); nulo quando não deu para ler.</summary>
    int? ReadCurrent(string? gameExePath);

    /// <summary>
    /// Aplica <paramref name="fps"/> ao jogo (0 = devolve o valor do usuário). <paramref name="gameExePath"/> escolhe o
    /// perfil por aplicativo e a placa (preferência de GPU do Windows para o exe); nulo serve só para limites globais.
    /// </summary>
    bool Apply(string? gameExePath, int fps);

    /// <summary>Fim da sessão ou abertura depois de uma queda: devolve tudo o que o Tako mudou. Sem nada guardado, não faz nada.</summary>
    void Restore();
}

/// <summary>Sem limitador (testes, ou enquanto nenhum driver foi integrado).</summary>
public sealed class NullFrameRateLimiter : IFrameRateLimiter
{
    public FrameRateLimitCapabilities Capabilities => FrameRateLimitCapabilities.None;

    public int? ReadCurrent(string? gameExePath) => null;

    public bool Apply(string? gameExePath, int fps) => false;

    public void Restore()
    {
    }
}

/// <summary>Limites de FPS oferecidos na Central de Controle. 0 = sem limite; <see cref="MatchDisplay"/> = taxa da tela − 3.</summary>
public static class FpsLimits
{
    /// <summary>"Igual à tela − 3": fica logo abaixo da taxa da tela para o VRR (G-Sync/FreeSync) não desligar.</summary>
    public const int MatchDisplay = -1;

    /// <summary>Quanto abaixo da taxa da tela fica o limite <see cref="MatchDisplay"/>.</summary>
    public const int DisplayMargin = 3;

    public static IReadOnlyList<int> Options { get; } = [0, 30, 40, 45, 60, 90, 120, 144, MatchDisplay];

    /// <summary>Um valor salvo que não está na lista vira o mais próximo (outros negativos = sem limite).</summary>
    public static int Normalize(int fps) =>
        fps == MatchDisplay ? MatchDisplay
        : fps <= 0 ? 0
        : Options.Where(o => o > 0).OrderBy(o => Math.Abs(o - fps)).ThenBy(o => o).First();

    /// <summary>Próximo valor para ← → (<paramref name="direction"/> negativo volta), parando nas pontas.</summary>
    public static int Step(int current, int direction)
    {
        int index = Options.ToList().IndexOf(Normalize(current));
        int next = Math.Clamp(index + Math.Sign(direction), 0, Options.Count - 1);
        return Options[next];
    }

    /// <summary>Próximo valor para A, voltando ao começo depois do último.</summary>
    public static int Cycle(int current)
    {
        int index = Options.ToList().IndexOf(Normalize(current));
        return Options[(index + 1) % Options.Count];
    }

    /// <summary>FPS de verdade para o driver: 0 = sem limite; nulo quando a opção depende da tela e a taxa é desconhecida.</summary>
    public static int? Resolve(int option, int? refreshHz)
    {
        if (option != MatchDisplay)
        {
            return Math.Max(0, option);
        }

        return refreshHz is { } hz && hz - DisplayMargin >= 20 ? hz - DisplayMargin : null;
    }

    public static string Label(int fps, int? refreshHz = null) => fps switch
    {
        MatchDisplay when Resolve(fps, refreshHz) is { } resolved => string.Create(CultureInfo.InvariantCulture, $"Tela − 3 ({resolved} FPS)"),
        MatchDisplay => "Tela − 3 (VRR)",
        <= 0 => "Sem limite",
        _ => string.Create(CultureInfo.InvariantCulture, $"{fps} FPS"),
    };
}

/// <summary>Qual executável de um jogo recebe o perfil do driver.</summary>
public static class GameExecutables
{
    // Processos que vivem na pasta do jogo mas não desenham o jogo.
    private static readonly string[] Helpers =
    [
        "crash", "launcher", "updater", "easyanticheat", "eac_", "battleye", "beservice", "uninst", "setup", "redist",
        "vcredist", "dxsetup", "webhelper", "cefprocess", "reporter", "unitycrashhandler",
    ];

    /// <summary>O alvo do atalho quando é um .exe (nulo para URI da loja, .lnk e afins).</summary>
    public static string? FromLaunchTarget(string? launchTarget)
    {
        if (string.IsNullOrWhiteSpace(launchTarget))
        {
            return null;
        }

        string path = launchTarget.Trim().Trim('"');
        return path.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) && !path.Contains("://", StringComparison.Ordinal) ? path : null;
    }

    /// <summary>
    /// Entre os processos do jogo, o que provavelmente desenha: pula ajudantes (anticheat, relatório de falha, launcher).
    /// Só ajudantes: o primeiro deles. Nada: nulo.
    /// </summary>
    public static string? Pick(IEnumerable<string?> executablePaths)
    {
        ArgumentNullException.ThrowIfNull(executablePaths);
        var paths = executablePaths.Where(p => !string.IsNullOrWhiteSpace(p) && p!.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
            .Select(p => p!)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToList();
        return paths.FirstOrDefault(p => !IsHelper(p)) ?? paths.FirstOrDefault();
    }

    /// <summary>Nome do arquivo (como os perfis do driver identificam o jogo), em minúsculas.</summary>
    public static string? FileName(string? exePath)
    {
        if (string.IsNullOrWhiteSpace(exePath))
        {
            return null;
        }

        string name = exePath.Trim().Trim('"').Replace('\\', '/');
        int slash = name.LastIndexOf('/');
        name = slash >= 0 ? name[(slash + 1)..] : name;
        return name.Length == 0 ? null : name.ToLowerInvariant();
    }

    private static bool IsHelper(string path)
    {
        string name = FileName(path) ?? string.Empty;
        return Helpers.Any(h => name.Contains(h, StringComparison.Ordinal));
    }
}
