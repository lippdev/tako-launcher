using System.Globalization;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.SessionTools;
using Launcher.Core.Session;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>O que o painel precisa saber do PC agora (telas, saídas de áudio) e como salvar.</summary>
/// <param name="Global">Configurações do Tako, mostradas como "Padrão: ...".</param>
/// <param name="Modes">Resoluções da tela de jogo escolhida (vazio sem tela escolhida ou desligada).</param>
/// <param name="Outputs">Saídas de áudio ativas (vazio quando o Windows não deixa trocar).</param>
/// <param name="RefreshHz">Taxa da tela, para o rótulo "Tela − 3".</param>
/// <param name="Save">Grava a escolha do jogo (na hora, como nos filtros).</param>
/// <param name="EditArguments">Abre o teclado virtual com os argumentos atuais; devolve o novo texto (nulo = cancelou).</param>
public sealed record GameSettingsContext(
    GlobalSessionSettings Global,
    Func<IReadOnlyList<DisplayMode>> Modes,
    Func<IReadOnlyList<AudioOutputChoice>> Outputs,
    int? RefreshHz,
    Action<GameSettingsOverride> Save,
    Action<string?, Action<string>> EditArguments,
    Action<string> Toast);

/// <summary>
/// "Configurações deste jogo" (UX-007): painel à direita, igual ao de filtros. Cada linha troca de valor com A e
/// começa em "Padrão: ..." (o valor do Tako); a mudança é salva na hora e só vale para este jogo.
/// </summary>
public sealed partial class GameSettingsViewModel : ObservableObject, IOverlayViewModel
{
    private const string Default = "Padrão";

    private readonly GameSettingsContext _context;
    private readonly PanelRowViewModel _screen;
    private readonly PanelRowViewModel _mode;
    private readonly PanelRowViewModel _hdr;
    private readonly PanelRowViewModel _audio;
    private readonly PanelRowViewModel _fps;
    private readonly PanelRowViewModel _showFps;
    private readonly PanelRowViewModel _arguments;
    private readonly PanelRowViewModel _reset;

    [ObservableProperty]
    private string _summary = string.Empty;

    public GameSettingsViewModel(string gameTitle, GameSettingsOverride current, GameSettingsContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        GameTitle = gameTitle;
        Current = current ?? GameSettingsOverride.None;
        _context = context;
        _screen = Row("screen", "Usar a Tela de jogo", () => Change(s => s with { UseGameScreen = Next(s.UseGameScreen) }));
        _mode = Row("mode", "Resolução e taxa", CycleMode);
        _hdr = Row("hdr", "HDR", () => Change(s => s with { Hdr = Next(s.Hdr) }));
        _audio = Row("audio", "Saída de áudio", CycleAudio);
        _fps = Row("fps", "Limite de FPS", () => Change(s => s with { FpsLimit = NextLimit(s.FpsLimit) }));
        _showFps = Row("show-fps", "Mostrar FPS", () => Change(s => s with { ShowFps = Next(s.ShowFps) }));
        _arguments = Row("args", "Argumentos extras", EditArguments);
        _reset = Row("reset", "Usar tudo do Tako", () => Change(_ => GameSettingsOverride.None));
        Sections =
        [
            new("Tela", "A tela é a escolhida em Configurações › Tela de jogo.", [_screen, _mode, _hdr]),
            new("Som", null, [_audio]),
            new("Desempenho", "Mostrar FPS precisa da permissão do Windows (Configurações › Mostrar FPS).", [_fps, _showFps]),
            new("Abrir o jogo", "Valem quando o Tako abre o executável. Jogos da Steam usam as opções de inicialização da Steam.", [_arguments]),
            new("Padrão", null, [_reset]),
        ];
        Update();
    }

    public string GameTitle { get; }

    public GameSettingsOverride Current { get; private set; }

    public IReadOnlyList<PanelSection> Sections { get; }

    private GlobalSessionSettings Global => _context.Global;

    private static bool? Next(bool? value) => value switch
    {
        null => true,
        true => false,
        false => null,
    };

    private static int? NextLimit(int? value)
    {
        if (value is null)
        {
            return FpsLimits.Options[0];
        }

        int index = FpsLimits.Options.ToList().IndexOf(FpsLimits.Normalize(value.Value));
        return index + 1 < FpsLimits.Options.Count ? FpsLimits.Options[index + 1] : null;
    }

    private static string YesNo(bool value) => value ? "Sim" : "Não";

    private static string OnOff(bool value) => value ? "Ligado" : "Desligado";

    private void CycleMode()
    {
        var modes = Global.GameScreenId.Length == 0 ? [] : _context.Modes();
        List<string?> options = [null, GameSettingsOverride.KeepCurrentMode, .. modes.Select(m => m.Key)];
        if (Current.ScreenMode is { Length: > 0 } saved && !options.Contains(saved))
        {
            options.Add(saved); // resolução de uma tela que não está ligada agora: continua escolhível
        }

        if (modes.Count == 0 && Current.ScreenMode is null)
        {
            _context.Toast(Global.GameScreenId.Length == 0 ? "Escolha a tela de jogo em Configurações primeiro" : "Ligue a tela de jogo para ver as resoluções dela");
        }

        int index = options.IndexOf(Current.ScreenMode);
        Change(s => s with { ScreenMode = options[(index + 1) % options.Count] });
    }

    private void CycleAudio()
    {
        var outputs = _context.Outputs().ToList();
        if (Current.AudioDeviceId is { Length: > 0 } saved && !outputs.Any(o => string.Equals(o.DeviceId, saved, StringComparison.OrdinalIgnoreCase)))
        {
            outputs.Add(new AudioOutputChoice(saved, Current.AudioDeviceName ?? saved)); // desconectada agora
        }

        List<AudioOutputChoice?> options = [null, new AudioOutputChoice(GameSettingsOverride.KeepAudio, string.Empty), .. outputs];
        int index = options.FindIndex(o => string.Equals(o?.DeviceId, Current.AudioDeviceId, StringComparison.OrdinalIgnoreCase));
        var next = options[(index + 1) % options.Count];
        Change(s => s with { AudioDeviceId = next?.DeviceId, AudioDeviceName = next is { Name.Length: > 0 } ? next.Name : null });
    }

    private void EditArguments() =>
        _context.EditArguments(Current.ExtraArguments, text => Change(s => s with { ExtraArguments = GameSettingsResolver.NormalizeArguments(text) }));

    private void Change(Func<GameSettingsOverride, GameSettingsOverride> change)
    {
        Current = change(Current);
        _context.Save(Current);
        Update();
    }

    private void Update()
    {
        var c = Current;
        _screen.EndText = c.UseGameScreen is { } use ? YesNo(use) : $"{Default}: {YesNo(Global.GameScreenEnabled)}";
        _mode.EndText = c.ScreenMode switch
        {
            null => $"{Default}: {ModeLabel(Global.GameScreenMode)}",
            GameSettingsOverride.KeepCurrentMode => "Manter a atual",
            string key => DisplayMode.TryParse(key)?.ToString() ?? key,
        };
        _hdr.EndText = c.Hdr is { } hdr ? OnOff(hdr) : $"{Default}: {OnOff(Global.GameScreenHdr)}";
        _audio.EndText = c.AudioDeviceId switch
        {
            null => $"{Default}: {Global.GameAudio?.Name ?? "não alterar"}",
            GameSettingsOverride.KeepAudio => "Não alterar",
            string id => c.AudioDeviceName ?? id,
        };
        _fps.EndText = c.FpsLimit is { } fps ? FpsLimits.Label(fps, _context.RefreshHz) : $"{Default}: {FpsLimits.Label(Global.FpsLimit, _context.RefreshHz)}";
        _showFps.EndText = c.ShowFps is { } show ? YesNo(show) : $"{Default}: {YesNo(Global.ShowFps)}";
        _arguments.EndText = GameSettingsResolver.NormalizeArguments(c.ExtraArguments) ?? "Nenhum";
        foreach (var row in (PanelRowViewModel[])[_screen, _mode, _hdr, _audio, _fps, _showFps])
        {
            row.IsOn = !row.EndText.StartsWith(Default, StringComparison.Ordinal);
        }

        _reset.EndText = c.ChangedCount switch
        {
            0 => string.Empty,
            1 => "1 mudança",
            int n => string.Create(CultureInfo.InvariantCulture, $"{n} mudanças"),
        };
        Summary = c.IsDefault ? "Tudo como nas Configurações do Tako" : $"{_reset.EndText} só para este jogo";
    }

    private static string ModeLabel(DisplayMode? mode) => mode?.ToString() ?? "manter a atual";

    private static PanelRowViewModel Row(string key, string label, Action action) =>
        new() { Key = $"gs:{key}", Label = label, Command = new RelayCommand(action) };
}
