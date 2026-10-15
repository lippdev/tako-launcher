using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.SessionTools;

namespace Launcher.Core.Session;

/// <summary>
/// "Configurações deste jogo" (UX-007): o que vale só para um jogo, por cima das Configurações do Tako. Cada campo nulo
/// é "Usar o padrão do Tako"; o registro vazio (<see cref="None"/>) não muda nada.
/// </summary>
public sealed record GameSettingsOverride
{
    /// <summary>Valor de <see cref="ScreenMode"/> para "Manter a resolução atual" (sem trocar o modo da tela).</summary>
    public const string KeepCurrentMode = "";

    /// <summary>Valor de <see cref="AudioDeviceId"/> para "Não alterar" a saída de áudio.</summary>
    public const string KeepAudio = "";

    public static GameSettingsOverride None { get; } = new();

    /// <summary>Usar (ou não) a Tela de jogo. A tela em si é a escolhida nas Configurações.</summary>
    public bool? UseGameScreen { get; init; }

    /// <summary>Resolução e taxa (<c>3840x2160@120</c>) ou <see cref="KeepCurrentMode"/>.</summary>
    public string? ScreenMode { get; init; }

    public bool? Hdr { get; init; }

    /// <summary>Saída de áudio durante o jogo ou <see cref="KeepAudio"/>.</summary>
    public string? AudioDeviceId { get; init; }

    /// <summary>Nome da saída, para mostrar mesmo com ela desconectada.</summary>
    public string? AudioDeviceName { get; init; }

    /// <summary>Opção de <see cref="FpsLimits.Options"/> (0 = sem limite).</summary>
    public int? FpsLimit { get; init; }

    public bool? ShowFps { get; init; }

    /// <summary>Acrescentados aos argumentos do jogo ao abrir pelo executável.</summary>
    public string? ExtraArguments { get; init; }

    /// <summary>Quantas opções fogem do padrão do Tako.</summary>
    public int ChangedCount =>
        (UseGameScreen is null ? 0 : 1) + (ScreenMode is null ? 0 : 1) + (Hdr is null ? 0 : 1) + (AudioDeviceId is null ? 0 : 1)
        + (FpsLimit is null ? 0 : 1) + (ShowFps is null ? 0 : 1) + (string.IsNullOrWhiteSpace(ExtraArguments) ? 0 : 1);

    public bool IsDefault => ChangedCount == 0;
}

/// <summary>Configurações do Tako que a sessão de jogo usa (Tela de jogo, áudio, limite e contador de FPS).</summary>
/// <param name="GameScreenId">Tela escolhida, mesmo com a Tela de jogo desligada (um jogo pode pedir para usá-la).</param>
public sealed record GlobalSessionSettings(
    bool GameScreenEnabled,
    string GameScreenId,
    HideOthersMode HideOthers,
    DisplayMode? GameScreenMode,
    bool GameScreenHdr,
    AudioOutputChoice? GameAudio,
    int FpsLimit,
    bool ShowFps);

/// <summary>O que vale na sessão deste jogo: o global com as escolhas do jogo por cima.</summary>
/// <param name="GameScreen">Nulo = não usar a Tela de jogo.</param>
/// <param name="GameScreenProblem">Por que o jogo pediu a Tela de jogo e ela não pode ser usada.</param>
/// <param name="FpsLimitFromGame">O limite vem do jogo: mudar na Central vale só para esta sessão.</param>
/// <param name="ShowFpsFromGame">"Mostrar FPS" vem do jogo.</param>
public sealed record EffectiveSessionSettings(
    GameScreenSettings? GameScreen,
    string? GameScreenProblem,
    AudioOutputChoice? GameAudio,
    int FpsLimit,
    bool FpsLimitFromGame,
    bool ShowFps,
    bool ShowFpsFromGame,
    string? ExtraArguments)
{
    /// <summary>Argumentos do jogo seguidos dos extras deste jogo.</summary>
    public string Arguments(string? gameArguments) => GameSettingsResolver.JoinArguments(gameArguments, ExtraArguments);
}

/// <summary>
/// Ponto único onde a sessão decide o que vale (UX-007): Configurações do Tako + configurações deste jogo → efetivo.
/// Puro e sem efeitos: a sessão (tela de jogo, áudio, limitador e contador de FPS) só lê o resultado.
/// </summary>
public static class GameSettingsResolver
{
    public const string NoScreenChosen = "escolha a tela de jogo em Configurações";

    public static EffectiveSessionSettings Resolve(GlobalSessionSettings global, GameSettingsOverride? game)
    {
        ArgumentNullException.ThrowIfNull(global);
        game ??= GameSettingsOverride.None;

        GameScreenSettings? screen = null;
        string? problem = null;
        if (game.UseGameScreen ?? global.GameScreenEnabled)
        {
            if (string.IsNullOrWhiteSpace(global.GameScreenId))
            {
                // Só é um problema quando o próprio jogo pediu; desligada no global e sem tela é o normal.
                problem = game.UseGameScreen == true ? NoScreenChosen : null;
            }
            else
            {
                var mode = game.ScreenMode switch
                {
                    null => global.GameScreenMode,
                    GameSettingsOverride.KeepCurrentMode => null,
                    string key => DisplayMode.TryParse(key) ?? global.GameScreenMode, // valor ilegível volta ao padrão
                };
                screen = new GameScreenSettings(global.GameScreenId, global.HideOthers, mode, game.Hdr ?? global.GameScreenHdr);
            }
        }

        var audio = game.AudioDeviceId switch
        {
            null => global.GameAudio,
            GameSettingsOverride.KeepAudio => null,
            string id => new AudioOutputChoice(id, string.IsNullOrWhiteSpace(game.AudioDeviceName) ? id : game.AudioDeviceName),
        };

        return new EffectiveSessionSettings(
            screen,
            problem,
            audio,
            FpsLimits.Normalize(game.FpsLimit ?? global.FpsLimit),
            game.FpsLimit is not null,
            game.ShowFps ?? global.ShowFps,
            game.ShowFps is not null,
            NormalizeArguments(game.ExtraArguments));
    }

    /// <summary>Junta argumentos com um espaço, ignorando os vazios.</summary>
    public static string JoinArguments(string? first, string? second) =>
        string.Join(' ', new[] { first, second }.Select(a => a?.Trim()).Where(a => !string.IsNullOrEmpty(a)));

    /// <summary>Texto digitado → argumentos (sem espaços nas pontas); vazio vira nulo.</summary>
    public static string? NormalizeArguments(string? text) => string.IsNullOrWhiteSpace(text) ? null : text.Trim();
}

/// <summary>
/// Combinações de tela + modo que o usuário já confirmou ver ("Você consegue ver esta tela?"). Com resolução por jogo
/// há mais de uma; guardar só a última faria a pergunta voltar a cada troca de jogo. Uma por linha, a mais recente primeiro.
/// </summary>
public static class GameScreenConfirmations
{
    public const int MaxRemembered = 8;

    /// <summary><paramref name="setupKey"/> quando já confirmada, senão vazio (o formato que a sessão de tela espera).</summary>
    public static string Matching(string? saved, string setupKey) =>
        Split(saved).Contains(setupKey, StringComparer.OrdinalIgnoreCase) ? setupKey : string.Empty;

    /// <summary>Lembra a combinação confirmada, mantendo as <see cref="MaxRemembered"/> mais recentes.</summary>
    public static string Remember(string? saved, string setupKey)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(setupKey);
        var keys = Split(saved).Where(k => !string.Equals(k, setupKey, StringComparison.OrdinalIgnoreCase)).Prepend(setupKey).Take(MaxRemembered);
        return string.Join('\n', keys);
    }

    private static string[] Split(string? saved) =>
        (saved ?? string.Empty).Split('\n', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
}
