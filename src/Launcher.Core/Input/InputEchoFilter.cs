namespace Launcher.Core.Input;

/// <summary>
/// Evita o comando em dobro quando o mesmo toque chega pelo controle e pelo teclado. A Steam aberta (com o suporte a
/// PlayStation ou Xbox ligado) aplica a "configuração de área de trabalho" fora dos jogos e transforma o direcional em
/// setas e os botões em Enter/Esc; o Tako lê o controle por HID/XInput ao mesmo tempo. Sem o filtro, ↑ andaria duas
/// casas e, ao segurar, a repetição do teclado do Windows atropelaria a do controle.
/// Regra: o teclado é ignorado enquanto o controle mantém o mesmo comando ou logo depois de o controle dispará-lo;
/// o controle é ignorado logo depois de o teclado disparar o mesmo comando (quando a tecla chega primeiro).
/// </summary>
public sealed class InputEchoFilter
{
    /// <summary>Distância máxima entre o toque no controle e o eco no teclado (ou o contrário).</summary>
    public static readonly TimeSpan Window = TimeSpan.FromMilliseconds(150);

    private readonly Dictionary<AppCommand, TimeSpan> _gamepad = [];
    private readonly Dictionary<AppCommand, TimeSpan> _keyboard = [];

    /// <summary>Comando do controle: falso se for eco de uma tecla que acabou de chegar.</summary>
    public bool AcceptGamepad(AppCommand command, TimeSpan now)
    {
        if (Recent(_keyboard, command, now))
        {
            _keyboard.Remove(command); // a tecla já fez o trabalho; a repetição do controle segue normal
            return false;
        }

        _gamepad[command] = now;
        return true;
    }

    /// <summary>Comando do teclado: falso se for eco do controle (<paramref name="gamepadHeld"/> = comandos mantidos agora).</summary>
    public bool AcceptKeyboard(AppCommand command, TimeSpan now, IReadOnlySet<AppCommand> gamepadHeld)
    {
        ArgumentNullException.ThrowIfNull(gamepadHeld);
        if (gamepadHeld.Contains(command) || Recent(_gamepad, command, now))
        {
            return false;
        }

        _keyboard[command] = now;
        return true;
    }

    private static bool Recent(Dictionary<AppCommand, TimeSpan> last, AppCommand command, TimeSpan now) =>
        last.TryGetValue(command, out var at) && now - at <= Window;
}
