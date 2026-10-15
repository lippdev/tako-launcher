using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Session.Devices;

namespace Launcher.Core.Session.GameScreen;

/// <summary>O que fazer com o "modo console ao abrir" nesta abertura do Tako.</summary>
public enum ConsoleModeStart
{
    /// <summary>Opção desligada: o Tako abre na mesa, como sempre.</summary>
    Off,

    /// <summary>Opção ligada, mas sem tela de jogo escolhida (ou com a "Tela de jogo" desligada).</summary>
    NoGameScreen,

    /// <summary>Modo seguro: nada que mexa nas telas sozinho.</summary>
    SafeMode,

    /// <summary>Vai para a tela de jogo agora.</summary>
    Enter,
}

/// <summary>Passos da tela de jogo, da TV e dos dispositivos para um jogo (dependem de o modo console estar ativo).</summary>
/// <param name="WakeTv">Ligar a TV e esperar antes de focar a tela de jogo.</param>
/// <param name="FocusScreen">Focar a tela de jogo ao abrir o jogo e desfazer ao fechar.</param>
/// <param name="Devices">Áudio, Steam e TV da sessão do jogo.</param>
/// <param name="Note">Aviso para o usuário quando uma configuração deste jogo não vale no modo console.</param>
public sealed record GameScreenSteps(bool WakeTv, bool FocusScreen, SessionDeviceOptions Devices, string? Note = null);

/// <summary>
/// Decisões puras do "modo console" (MVP-002/012): ao abrir o Tako, ir para a tela de jogo e só voltar para a mesa ao
/// sair (ou em "Voltar ao PC"). Com o modo console ativo, cada jogo só cuida do que é dele.
/// </summary>
public static class ConsoleModePolicy
{
    /// <summary>Atalho de emergência que devolve as telas para a mesa (documentado no checklist e nas Configurações).</summary>
    public const string EmergencyKeys = "Ctrl+Alt+Shift+D";

    public static ConsoleModeStart AtStartup(bool optionOn, GameScreenSettings? screen, bool safeMode)
    {
        if (!optionOn)
        {
            return ConsoleModeStart.Off;
        }

        if (screen is null)
        {
            return ConsoleModeStart.NoGameScreen;
        }

        return safeMode ? ConsoleModeStart.SafeMode : ConsoleModeStart.Enter;
    }

    /// <summary>
    /// Passos de um jogo a partir do efetivo do <see cref="GameSettingsResolver"/> (Configurações do Tako + deste jogo).
    /// <para>Fora do modo console, como na ADR-0015/0016: TV antes da tela de jogo (se o jogo usa a tela de jogo e a TV
    /// está ligada), tela de jogo durante o jogo, áudio e TV da sessão.</para>
    /// <para>Dentro dele, a tela de jogo, a TV e o áudio são do modo console: o jogo não foca nem restaura a tela, não liga
    /// a TV nem a põe em espera. Das configurações deste jogo valem só as que não mexem nas telas:</para>
    /// <list type="bullet">
    /// <item>"Usar a Tela de jogo" desligado, resolução ou HDR próprios: não valem (o jogo abre na tela de jogo do modo
    /// console, no modo dela); o aviso diz isso. Para jogar na mesa, "Voltar ao PC" antes.</item>
    /// <item>Saída de áudio própria (diferente da do modo console): troca no início e volta para a do modo console no fim.
    /// Sem saída própria, o áudio fica como o modo console deixou.</item>
    /// <item>FPS, contador e argumentos: valem como sempre (não passam por aqui).</item>
    /// </list>
    /// A Steam na bandeja continua sendo do fim de cada jogo.
    /// </summary>
    /// <param name="consoleScreen">Tela de jogo que o modo console focou (a das Configurações do Tako).</param>
    /// <param name="consoleAudio">Saída que o modo console usa (a das Configurações do Tako).</param>
    public static GameScreenSteps ForGame(bool consoleModeActive, EffectiveSessionSettings effective, SessionDeviceOptions devices, GameScreenSettings? consoleScreen, AudioOutputChoice? consoleAudio)
    {
        ArgumentNullException.ThrowIfNull(effective);
        ArgumentNullException.ThrowIfNull(devices);
        if (!consoleModeActive)
        {
            return new GameScreenSteps(effective.GameScreen is not null && devices.Tv.IsActive, effective.GameScreen is not null, devices);
        }

        var audio = devices.GameAudio is { } own && !SameDevice(own, consoleAudio) ? own : null;
        string? note = effective.GameScreen switch
        {
            null => "Modo console: este jogo abre na tela de jogo · para jogar na mesa, use \"Voltar ao PC\"",
            { } screen when consoleScreen is not null && (screen.Mode != consoleScreen.Mode || screen.EnableHdr != consoleScreen.EnableHdr) =>
                "Modo console: a resolução e o HDR deste jogo não valem · a tela de jogo fica como o modo console deixou",
            _ => null,
        };
        return new GameScreenSteps(false, false, devices with { GameAudio = audio, Tv = TvControlConfig.Default }, note);
    }

    private static bool SameDevice(AudioOutputChoice a, AudioOutputChoice? b) =>
        b is not null && string.Equals(a.DeviceId, b.DeviceId, StringComparison.OrdinalIgnoreCase);
}
