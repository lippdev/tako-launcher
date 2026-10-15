// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/SteamShutdown.cs
namespace ConsoleMode.Runtime.Steam;

/// <summary>Estado do cliente da Steam e a janela dele (implementação por plataforma).</summary>
public interface ISteamClient
{
    /// <summary>O cliente da Steam está aberto.</summary>
    bool IsRunning();

    /// <summary>
    /// Jogo que a Steam diz estar rodando (<c>RunningAppID</c>): 0 = nenhum; nulo = não foi possível saber.
    /// </summary>
    int? RunningAppId();

    /// <summary>
    /// Fecha a janela do cliente para a Steam ir para a bandeja. Nunca encerra o processo. Falso se não havia janela
    /// para fechar ou o pedido falhou.
    /// </summary>
    bool CloseWindowToTray();
}

/// <summary>Decisão de mandar a Steam para a bandeja quando o jogo termina.</summary>
public enum SteamTrayDecision
{
    /// <summary>Fechar a janela do cliente para a bandeja.</summary>
    CloseToTray,

    /// <summary>O jogo não é da Steam: a Steam não é nossa para fechar.</summary>
    NotSteamGame,

    /// <summary>Desligado nas Configurações.</summary>
    SettingOff,

    /// <summary>A Steam não está aberta: nada a fechar (e pedir a abriria).</summary>
    NotRunning,

    /// <summary>Outro jogo da Steam está rodando (<c>RunningAppID</c> diferente de 0).</summary>
    GameRunning,

    /// <summary>Não deu para saber se há jogo rodando: a Steam fica aberta.</summary>
    GameStateUnknown,
}

/// <summary>Regras puras sobre a Steam: ler <c>RunningAppID</c>, escolher a janela do cliente e decidir a bandeja.</summary>
public static class SteamClientRules
{
    /// <summary>
    /// Processos da própria Steam. Só as janelas deles podem receber o pedido de fechar: um jogo é uma janela SDL_app
    /// em tela cheia como o Big Picture e nunca deve receber o WM_CLOSE da Steam.
    /// </summary>
    public static bool IsSteamProcess(string? processName) =>
        string.Equals(processName, "steam", StringComparison.OrdinalIgnoreCase)
        || string.Equals(processName, "steamwebhelper", StringComparison.OrdinalIgnoreCase);

    /// <summary>Janela principal do cliente: visível, sem dona, título "Steam", de um processo da Steam.</summary>
    public static bool IsClientWindowCandidate(string processName, string title, string className, bool visible, bool hasOwner)
    {
        ArgumentNullException.ThrowIfNull(title);
        return visible && !hasOwner
            && IsSteamProcess(processName)
            && string.Equals(title.Trim(), "Steam", StringComparison.OrdinalIgnoreCase)
            && (string.Equals(className, "Chrome_WidgetWin_1", StringComparison.Ordinal)
                || string.Equals(className, "vguiPopupWindow", StringComparison.Ordinal)
                || string.Equals(className, "SDL_app", StringComparison.Ordinal));
    }

    public static SteamTrayDecision DecideTray(bool steamGame, bool settingOn, bool steamRunning, int? runningAppId)
    {
        if (!steamGame)
        {
            return SteamTrayDecision.NotSteamGame;
        }

        if (!settingOn)
        {
            return SteamTrayDecision.SettingOff;
        }

        if (!steamRunning)
        {
            return SteamTrayDecision.NotRunning;
        }

        return runningAppId switch
        {
            null => SteamTrayDecision.GameStateUnknown,
            > 0 => SteamTrayDecision.GameRunning,
            _ => SteamTrayDecision.CloseToTray,
        };
    }

    /// <summary>
    /// A Steam guarda o jogo em execução no registro (HKCU\Software\Valve\Steam\RunningAppID, DWORD). Zero: lido e sem
    /// jogo. Nulo: ausente, inválido, fora da faixa ou ilegível, ou seja, estado desconhecido.
    /// </summary>
    public static int? ParseRunningAppId(object? registryValue) => registryValue switch
    {
        int id when id >= 0 => id,
        uint id when id <= int.MaxValue => (int)id,
        string text when int.TryParse(text, System.Globalization.NumberStyles.None, System.Globalization.CultureInfo.InvariantCulture, out int id) => id,
        _ => null,
    };
}
