using System.Globalization;

namespace Launcher.Core.Input;

/// <summary>
/// Atalho do controle que abre e fecha a Central de Controle (UX-005): uma combinação de botões segurada por
/// <see cref="HoldTime"/>. Padrão: View + Y (Create/Share + △ no PlayStation). O Guia/PS deixou de ser o padrão porque
/// abre a Barra de Jogo do Xbox ou a Steam junto. Segurar (em vez de apertar junto) evita abrir sem querer no meio do jogo.
/// </summary>
public static class ControlCenterChord
{
    public const GamepadButtons Default = GamepadButtons.View | GamepadButtons.Y;

    /// <summary>Botões que não têm ação própria no Tako: a combinação precisa de pelo menos um deles.</summary>
    public const GamepadButtons Free = GamepadButtons.View | GamepadButtons.Guide;

    /// <summary>Botões que o Tako reconhece (os do analógico clicado, por exemplo, ficam de fora).</summary>
    public const GamepadButtons Known = GamepadButtons.DPadUp | GamepadButtons.DPadDown | GamepadButtons.DPadLeft | GamepadButtons.DPadRight
        | GamepadButtons.Start | GamepadButtons.View | GamepadButtons.LeftShoulder | GamepadButtons.RightShoulder | GamepadButtons.Guide
        | GamepadButtons.A | GamepadButtons.B | GamepadButtons.X | GamepadButtons.Y;

    public const int MaxButtons = 3;

    public static readonly TimeSpan HoldTime = TimeSpan.FromMilliseconds(500);

    /// <summary>Ordem de exibição: primeiro os "modificadores" (View, Guia), depois ombros, faces e direcional.</summary>
    private static readonly GamepadButtons[] DisplayOrder =
    [
        GamepadButtons.View, GamepadButtons.Guide, GamepadButtons.Start, GamepadButtons.LeftShoulder, GamepadButtons.RightShoulder,
        GamepadButtons.A, GamepadButtons.B, GamepadButtons.X, GamepadButtons.Y,
        GamepadButtons.DPadUp, GamepadButtons.DPadDown, GamepadButtons.DPadLeft, GamepadButtons.DPadRight,
    ];

    public static int Count(GamepadButtons buttons) => DisplayOrder.Count(b => buttons.HasFlag(b));

    /// <summary>Nome de cada botão no controle em uso: "View", "Y" no Xbox; "Create", "△" no DualSense; "Share" no DualShock 4.</summary>
    public static IReadOnlyList<string> Names(GamepadButtons buttons, GamepadFamily family) =>
        DisplayOrder.Where(b => buttons.HasFlag(b)).Select(b => Name(b, family)).ToList();

    /// <summary>"View + Y", "Create + △".</summary>
    public static string Format(GamepadButtons buttons, GamepadFamily family) => string.Join(" + ", Names(buttons, family));

    public static string Name(GamepadButtons button, GamepadFamily family) => (button, family) switch
    {
        (GamepadButtons.A, GamepadFamily.PlayStation5 or GamepadFamily.PlayStation4) => "✕",
        (GamepadButtons.B, GamepadFamily.PlayStation5 or GamepadFamily.PlayStation4) => "○",
        (GamepadButtons.X, GamepadFamily.PlayStation5 or GamepadFamily.PlayStation4) => "□",
        (GamepadButtons.Y, GamepadFamily.PlayStation5 or GamepadFamily.PlayStation4) => "△",
        (GamepadButtons.LeftShoulder, GamepadFamily.PlayStation5 or GamepadFamily.PlayStation4) => "L1",
        (GamepadButtons.RightShoulder, GamepadFamily.PlayStation5 or GamepadFamily.PlayStation4) => "R1",
        (GamepadButtons.Start, GamepadFamily.PlayStation5 or GamepadFamily.PlayStation4) => "Options",
        (GamepadButtons.Guide, GamepadFamily.PlayStation5 or GamepadFamily.PlayStation4) => "PS",
        (GamepadButtons.View, GamepadFamily.PlayStation5) => "Create",
        (GamepadButtons.View, GamepadFamily.PlayStation4) => "Share",
        (GamepadButtons.A, _) => "A",
        (GamepadButtons.B, _) => "B",
        (GamepadButtons.X, _) => "X",
        (GamepadButtons.Y, _) => "Y",
        (GamepadButtons.LeftShoulder, _) => "LB",
        (GamepadButtons.RightShoulder, _) => "RB",
        (GamepadButtons.Start, _) => "Menu",
        (GamepadButtons.Guide, _) => "Guia",
        (GamepadButtons.View, _) => "View",
        (GamepadButtons.DPadUp, _) => "↑",
        (GamepadButtons.DPadDown, _) => "↓",
        (GamepadButtons.DPadLeft, _) => "←",
        (GamepadButtons.DPadRight, _) => "→",
        _ => button.ToString(),
    };

    /// <summary>Valor guardado nas configurações (número); inválido ou vazio volta ao padrão.</summary>
    public static GamepadButtons Parse(string? text) =>
        int.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, out int value) && value > 0 && ((GamepadButtons)value & ~Known) == 0
            ? (GamepadButtons)value
            : Default;

    public static string Serialize(GamepadButtons buttons) => ((int)buttons).ToString(CultureInfo.InvariantCulture);
}

/// <summary>Família do controle, para os nomes dos botões nas dicas e nas mensagens.</summary>
public enum GamepadFamily
{
    Xbox,
    PlayStation5,
    PlayStation4,
}

/// <summary>
/// Reconhece a combinação segurada: dispara uma vez depois de <see cref="ControlCenterChord.HoldTime"/> com todos os
/// botões dela pressionados e só volta a disparar depois que algum for solto. Um único detector serve com o Tako em
/// primeiro e em segundo plano, então a mesma pressão não abre e fecha a Central.
/// </summary>
public sealed class ButtonChordDetector(GamepadButtons chord)
{
    private GamepadButtons _chord = chord & ControlCenterChord.Known;
    private TimeSpan? _since;
    private bool _fired;

    public GamepadButtons Chord
    {
        get => _chord;
        set
        {
            _chord = value & ControlCenterChord.Known;
            _since = null;
            _fired = true; // a combinação nova só vale depois de soltar tudo
        }
    }

    /// <summary>Fração do tempo de segurar já cumprida (0 a 1), para um indicador; 0 sem a combinação pressionada.</summary>
    public double Progress { get; private set; }

    public bool Update(GamepadButtons pressed, TimeSpan now)
    {
        if (_chord == GamepadButtons.None || (pressed & _chord) != _chord)
        {
            _since = null;
            _fired = false;
            Progress = 0;
            return false;
        }

        if (_fired)
        {
            return false;
        }

        _since ??= now;
        var held = now - _since.Value;
        Progress = Math.Clamp(held / ControlCenterChord.HoldTime, 0, 1);
        if (held < ControlCenterChord.HoldTime)
        {
            return false;
        }

        _fired = true;
        Progress = 0;
        return true;
    }

    /// <summary>
    /// Botões da combinação que não devem virar comando agora: com os botões "livres" dela (View, Guia) pressionados,
    /// o Y de View + Y não abre a busca.
    /// </summary>
    public GamepadButtons Suppressed(GamepadButtons pressed)
    {
        var free = _chord & ControlCenterChord.Free;
        return free != GamepadButtons.None && (pressed & free) == free ? _chord & ~free : GamepadButtons.None;
    }
}

public enum ChordCaptureState
{
    /// <summary>Esperando soltar o que já estava pressionado ao abrir a tela.</summary>
    Releasing,

    Waiting,
    Holding,
    Captured,
    Cancelled,
}

/// <summary>
/// Tela "Segure os botões que quer usar": a combinação vale quando fica igual por <see cref="HoldTime"/>. Tocar B/○
/// sozinho (apertar e soltar antes disso) cancela; o que já estava pressionado ao abrir a tela é ignorado até ser solto.
/// </summary>
public sealed class ChordCapture
{
    public static readonly TimeSpan HoldTime = TimeSpan.FromSeconds(1);

    private TimeSpan _since;

    public ChordCaptureState State { get; private set; } = ChordCaptureState.Releasing;

    /// <summary>Botões pressionados agora (só os reconhecidos).</summary>
    public GamepadButtons Holding { get; private set; }

    public GamepadButtons Result { get; private set; }

    public double Progress { get; private set; }

    /// <summary>Recomeça depois de uma combinação recusada.</summary>
    public void Restart()
    {
        State = ChordCaptureState.Releasing;
        Holding = GamepadButtons.None;
        Result = GamepadButtons.None;
        Progress = 0;
    }

    public ChordCaptureState Update(GamepadButtons pressed, TimeSpan now)
    {
        pressed &= ControlCenterChord.Known;
        switch (State)
        {
            case ChordCaptureState.Captured or ChordCaptureState.Cancelled:
                return State;
            case ChordCaptureState.Releasing:
                if (pressed == GamepadButtons.None)
                {
                    State = ChordCaptureState.Waiting;
                }

                return State;
        }

        if (pressed == GamepadButtons.None)
        {
            bool tappedBack = State == ChordCaptureState.Holding && Holding == GamepadButtons.B;
            State = tappedBack ? ChordCaptureState.Cancelled : ChordCaptureState.Waiting;
            Holding = GamepadButtons.None;
            Progress = 0;
            return State;
        }

        if (State == ChordCaptureState.Waiting || pressed != Holding)
        {
            // Botão novo ou um a menos: a contagem recomeça com o conjunto de agora.
            State = ChordCaptureState.Holding;
            Holding = pressed;
            _since = now;
        }

        var held = now - _since;
        Progress = Math.Clamp(held / HoldTime, 0, 1);
        if (held >= HoldTime)
        {
            Result = Holding;
            State = ChordCaptureState.Captured;
        }

        return State;
    }
}

/// <summary>Modificadores com os mesmos valores do <c>ModifierKeys</c> do WPF.</summary>
[Flags]
public enum KeyModifiers
{
    None = 0,
    Alt = 1,
    Ctrl = 2,
    Shift = 4,
    Win = 8,
}

/// <summary>
/// Atalho de teclado (UX-005), com a tecla pelo nome do <c>Key</c> do WPF ("T", "F10", "OemTilde"). Padrão
/// <c>Ctrl+Shift+T</c>: o Shift+Tab é da Steam (e do GOG Galaxy), e o <c>Ctrl+Shift+O</c> é o painel de métricas do AMD Adrenalin.
/// </summary>
public readonly record struct KeyShortcut(KeyModifiers Modifiers, string Key)
{
    public static readonly KeyShortcut Default = new(KeyModifiers.Ctrl | KeyModifiers.Shift, "T");

    /// <summary>Teclas que só modificam: o atalho espera a tecla de verdade.</summary>
    public static bool IsModifierKey(string key) => key is "LeftCtrl" or "RightCtrl" or "LeftAlt" or "RightAlt" or "LeftShift" or "RightShift"
        or "LWin" or "RWin" or "System" or "None" or "ImeProcessed" or "DeadCharProcessed";

    /// <summary>"Ctrl+Shift+T".</summary>
    public string Display
    {
        get
        {
            var parts = new List<string>(4);
            if (Modifiers.HasFlag(KeyModifiers.Ctrl))
            {
                parts.Add("Ctrl");
            }

            if (Modifiers.HasFlag(KeyModifiers.Alt))
            {
                parts.Add("Alt");
            }

            if (Modifiers.HasFlag(KeyModifiers.Shift))
            {
                parts.Add("Shift");
            }

            if (Modifiers.HasFlag(KeyModifiers.Win))
            {
                parts.Add("Win");
            }

            parts.Add(KeyName(Key));
            return string.Join("+", parts);
        }
    }

    /// <summary>Formato guardado: modificadores (número) e tecla, ex.: "6|T". Inválido volta ao padrão.</summary>
    public string Serialize() => $"{(int)Modifiers}|{Key}";

    public static KeyShortcut Parse(string? text)
    {
        if (text?.Split('|') is [string mods, string key] && int.TryParse(mods, NumberStyles.Integer, CultureInfo.InvariantCulture, out int value)
            && value is >= 0 and <= 15 && key.Length > 0 && !IsModifierKey(key))
        {
            return new KeyShortcut((KeyModifiers)value, key);
        }

        return Default;
    }

    public static string KeyName(string key) => key switch
    {
        ['D', char digit] when char.IsAsciiDigit(digit) => digit.ToString(),
        _ when key.StartsWith("NumPad", StringComparison.Ordinal) => "Num " + key[6..],
        "Return" or "Enter" => "Enter",
        "Escape" => "Esc",
        "Back" => "Backspace",
        "Space" => "Espaço",
        "Prior" or "PageUp" => "Page Up",
        "Next" or "PageDown" => "Page Down",
        "OemTilde" or "Oem3" => "'",
        "OemMinus" => "-",
        "OemPlus" => "=",
        "OemComma" => ",",
        "OemPeriod" => ".",
        _ => key,
    };
}

/// <summary>Resultado da validação de um atalho: erro impede salvar; aviso salva e explica o risco.</summary>
public readonly record struct ShortcutCheck(string? Error, string? Warning)
{
    public bool IsValid => Error is null;

    public static ShortcutCheck Ok { get; } = new(null, null);
}

/// <summary>
/// Regras dos atalhos da Central (UX-005): nenhuma combinação pode servir a duas ações do Tako; atalhos do Windows são
/// recusados; conflitos conhecidos com Steam, Barra de Jogo, NVIDIA, AMD e lojas viram aviso.
/// </summary>
public static class ShortcutRules
{
    /// <summary>Teclas sozinhas que já são ações do Tako (navegação, favoritar, busca, seções, menu).</summary>
    private static readonly Dictionary<string, string> TakoKeys = new(StringComparer.Ordinal)
    {
        ["Up"] = "navegar",
        ["Down"] = "navegar",
        ["Left"] = "navegar",
        ["Right"] = "navegar",
        ["Return"] = "selecionar",
        ["Enter"] = "selecionar",
        ["Space"] = "selecionar",
        ["Escape"] = "voltar",
        ["Back"] = "voltar",
        ["Tab"] = "navegar",
        ["F"] = "favoritar",
        ["Oem2"] = "buscar",
        ["OemQuestion"] = "buscar",
        ["Divide"] = "buscar",
        ["Q"] = "trocar de seção",
        ["E"] = "trocar de seção",
        ["M"] = "abrir o menu",
    };

    /// <summary>Do Windows: o Tako não consegue (nem deve) tomar.</summary>
    private static readonly HashSet<string> Reserved = new(StringComparer.Ordinal)
    {
        "Alt+Tab", "Alt+F4", "Alt+Esc", "Alt+Espaço", "Ctrl+Esc", "Ctrl+Alt+Delete", "Ctrl+Shift+Esc", "Ctrl+Alt+Tab",
    };

    /// <summary>Atalhos de outros programas comuns em PCs de jogo: o Tako aceita, mas avisa.</summary>
    private static readonly Dictionary<string, string> KnownConflicts = new(StringComparer.Ordinal)
    {
        ["Shift+Tab"] = "abre a sobreposição da Steam e do GOG Galaxy",
        ["Shift+F1"] = "abre a sobreposição do EA app",
        ["Shift+F2"] = "abre a sobreposição do Ubisoft Connect",
        ["Shift+F3"] = "abre a sobreposição da Epic Games",
        ["Alt+Z"] = "abre a sobreposição da NVIDIA",
        ["Alt+R"] = "abre o painel de desempenho da NVIDIA e da AMD",
        ["Alt+F1"] = "tira captura de tela na NVIDIA",
        ["Alt+F9"] = "grava a tela na NVIDIA",
        ["Alt+F10"] = "salva o replay instantâneo da NVIDIA",
        ["Alt+Enter"] = "alterna a tela cheia em muitos jogos",
        ["Ctrl+Shift+O"] = "liga as métricas do AMD Adrenalin",
        ["Ctrl+Shift+E"] = "grava a tela no AMD Adrenalin",
        ["Ctrl+Shift+S"] = "salva o replay do AMD Adrenalin",
        ["Ctrl+Shift+R"] = "liga o replay do AMD Adrenalin",
        ["Ctrl+Shift+I"] = "tira captura de tela no AMD Adrenalin",
        ["Ctrl+Shift+G"] = "transmite pelo AMD Adrenalin",
        ["Ctrl+Shift+M"] = "liga o microfone no AMD Adrenalin e no Discord",
        ["Ctrl+Shift+C"] = "liga a câmera no AMD Adrenalin",
        ["Ctrl+Shift+L"] = "registra o desempenho no AMD Adrenalin",
    };

    /// <summary>
    /// Atalho de teclado para a Central. <paramref name="takoShortcuts"/>: outros atalhos globais do Tako
    /// (ex.: o de emergência do modo console), pelo texto de <see cref="KeyShortcut.Display"/>, com a ação.
    /// </summary>
    public static ShortcutCheck CheckKeyboard(KeyShortcut shortcut, IReadOnlyDictionary<string, string>? takoShortcuts = null)
    {
        string text = shortcut.Display;
        if (KeyShortcut.IsModifierKey(shortcut.Key))
        {
            return new("Aperte uma tecla junto com Ctrl ou Alt", null);
        }

        if (shortcut.Modifiers.HasFlag(KeyModifiers.Win))
        {
            return new("Atalhos com a tecla Windows são do Windows", null);
        }

        if (Reserved.Contains(text))
        {
            return new($"{text} é do Windows", null);
        }

        if (takoShortcuts?.TryGetValue(text, out string? action) == true)
        {
            return new($"{text} já serve para {action} no Tako", null);
        }

        if ((shortcut.Modifiers & (KeyModifiers.Ctrl | KeyModifiers.Alt)) == KeyModifiers.None)
        {
            string why = TakoKeys.TryGetValue(shortcut.Key, out string? tako) && shortcut.Modifiers == KeyModifiers.None
                ? $"{KeyShortcut.KeyName(shortcut.Key)} já serve para {tako} no Tako"
                : KnownConflicts.TryGetValue(text, out string? other) ? $"{text} {other}" : $"{text} sem Ctrl ou Alt é usado pelos jogos";
            return new($"{why} · use Ctrl ou Alt junto", null);
        }

        if (KnownConflicts.TryGetValue(text, out string? conflict))
        {
            return new(null, $"{text} também {conflict}");
        }

        if (shortcut.Modifiers.HasFlag(KeyModifiers.Ctrl) && shortcut.Modifiers.HasFlag(KeyModifiers.Alt) && IsCharacterKey(shortcut.Key))
        {
            return new(null, "Em teclados ABNT2, Ctrl+Alt com letra ou número pode digitar símbolos (AltGr)");
        }

        return ShortcutCheck.Ok;
    }

    /// <summary>Combinação do controle para a Central (sempre segurada por meio segundo).</summary>
    public static ShortcutCheck CheckChord(GamepadButtons chord, GamepadFamily family)
    {
        chord &= ControlCenterChord.Known;
        int count = ControlCenterChord.Count(chord);
        if (count == 0)
        {
            return new("Segure pelo menos um botão", null);
        }

        if (count > ControlCenterChord.MaxButtons)
        {
            return new($"Use no máximo {ControlCenterChord.MaxButtons} botões", null);
        }

        string text = ControlCenterChord.Format(chord, family);
        if ((chord & ControlCenterChord.Free) == GamepadButtons.None)
        {
            string free = $"{ControlCenterChord.Name(GamepadButtons.View, family)} ou {ControlCenterChord.Name(GamepadButtons.Guide, family)}";
            return new($"{text} já {(count == 1 ? "serve" : "servem")} para navegar no Tako · inclua {free}", null);
        }

        if (chord == GamepadButtons.Guide)
        {
            return new(null, family == GamepadFamily.Xbox
                ? "O Guia também abre a Barra de Jogo do Xbox e a Steam"
                : "O PS também abre a Steam e outros programas");
        }

        if (chord == GamepadButtons.View)
        {
            return new(null, $"Sozinho, o {text} é usado pelos jogos (mapa, placar)");
        }

        return ShortcutCheck.Ok;
    }

    private static bool IsCharacterKey(string key) =>
        (key.Length == 1 && char.IsAsciiLetter(key[0])) || (key.Length == 2 && key[0] == 'D' && char.IsAsciiDigit(key[1])) || key.StartsWith("Oem", StringComparison.Ordinal);
}
