using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;
using Launcher.Core.Presentation;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Um passo do assistente na barra de progresso ("Tela", "Imagem"...).</summary>
public sealed partial class WizardStepViewModel(string label) : ObservableObject
{
    [ObservableProperty]
    private bool _isCurrent;

    [ObservableProperty]
    private bool _isDone;

    public string Label { get; } = label;
}

/// <summary>
/// Assistente na tela (ADR-0020): título, texto, os passos no topo e as escolhas do passo atual, uma por linha. B volta
/// um passo (no primeiro, fecha).
/// </summary>
public sealed partial class SetupWizardViewModel : ObservableObject, IOverlayViewModel
{
    private readonly Func<bool> _back;

    [ObservableProperty]
    private string _title = string.Empty;

    [ObservableProperty]
    private string _text = string.Empty;

    [ObservableProperty]
    private string _stepText = string.Empty;

    public SetupWizardViewModel(string heading, IEnumerable<string> steps, Func<bool> back)
    {
        Heading = heading;
        Steps = [.. steps.Select(s => new WizardStepViewModel(s))];
        _back = back;
    }

    public string Heading { get; }

    public IReadOnlyList<WizardStepViewModel> Steps { get; }

    public ObservableCollection<OverlayAction> Actions { get; } = [];

    /// <summary>Mostra um passo: marca o progresso e troca título, texto e escolhas.</summary>
    public void Show(int step, string title, string text, IEnumerable<OverlayAction> actions)
    {
        for (int i = 0; i < Steps.Count; i++)
        {
            Steps[i].IsCurrent = i == step;
            Steps[i].IsDone = i < step;
        }

        StepText = $"Passo {step + 1} de {Steps.Count}";
        Title = title;
        Text = text;
        Actions.Clear();
        foreach (var action in actions)
        {
            Actions.Add(action);
        }
    }

    public bool HandleBack() => _back();
}

/// <summary>
/// "Configurar para jogar na TV": escolher a tela → testar a imagem → TV → áudio → pronto. Cada passo já salva o que
/// foi escolhido (sair no meio não desfaz nada) e pode ser pulado; tudo pode ser mudado depois nos cartões.
/// </summary>
public sealed class TvSetupWizard(ConsoleModeViewModel page, ShellViewModel shell)
{
    private const int ScreenStep = 0;
    private const int TestStep = 1;
    private const int TvStep = 2;
    private const int AudioStep = 3;
    private const int DoneStep = 4;

    private readonly ShellViewModel _shell = shell;
    private SetupWizardViewModel? _view;
    private int _step;
    private bool _busy;

    public SetupWizardViewModel? View => _view;

    public void Start()
    {
        _view = new SetupWizardViewModel("Configurar para jogar na TV", ["Tela", "Imagem", "TV", "Áudio", "Pronto"], Back);
        Go(ScreenStep);
        _shell.PushOverlay(_view);
    }

    private bool Back()
    {
        if (_busy)
        {
            return true; // testando a tela: o próprio teste responde ao B
        }

        if (_step == ScreenStep || _step == DoneStep)
        {
            Close();
            return true;
        }

        Go(_step - 1);
        return true;
    }

    private void Go(int step)
    {
        _step = step;
        switch (step)
        {
            case ScreenStep:
                ShowScreens();
                break;
            case TestStep:
                ShowTest(problem: null);
                break;
            case TvStep:
                ShowTv();
                break;
            case AudioStep:
                ShowAudio();
                break;
            default:
                ShowDone();
                break;
        }

        _shell.RequestFocus(null); // foco na escolha padrão do passo novo
    }

    private void ShowScreens()
    {
        var settings = _shell.Settings;
        IReadOnlyList<ConsoleMode.Runtime.Display.ConnectedDisplay> displays;
        try
        {
            displays = _shell.GameScreen.ListDisplays();
        }
        catch (RuntimeEnvironmentException)
        {
            displays = [];
        }

        if (displays.Count == 0)
        {
            _view!.Show(ScreenStep, "Nenhuma tela encontrada", "Ligue a TV no PC pelo cabo HDMI e tente de novo.",
            [
                Action("Tentar de novo", ActionStyle.Primary, () => Go(ScreenStep), isDefault: true),
                Action("Fechar", ActionStyle.Secondary, Close),
            ]);
            return;
        }

        // Sugestão: a tela já escolhida; senão, uma que não é a principal (normalmente a TV).
        var suggested = displays.FirstOrDefault(d => string.Equals(d.Id, settings.GameScreenId, StringComparison.OrdinalIgnoreCase))
            ?? displays.FirstOrDefault(d => !d.IsPrimary) ?? displays[0];
        var actions = displays.Select(d => Action(
            d.Name + (d.IsPrimary ? " (principal)" : string.Empty),
            d == suggested ? ActionStyle.Primary : ActionStyle.Secondary,
            () =>
            {
                page.ChooseScreenForWizard(d);
                Go(TestStep);
            },
            isDefault: d == suggested)).ToList();
        _view!.Show(ScreenStep, "Em qual tela você vai jogar?", "Escolha a TV (ou o monitor) da sala. Durante os jogos o Tako usa só ela e as outras telas voltam ao fim.", actions);
    }

    private void ShowTest(string? problem)
    {
        string name = _shell.Settings.GameScreenName;
        string text = problem is null
            ? $"O Tako vai aparecer em {name} por alguns segundos. Se você vir a pergunta na TV, confirme com o controle. Se não vir, espere: as telas voltam sozinhas."
            : $"{problem}. Confira se a TV está ligada na entrada do PC e tente de novo, ou pule este passo.";
        _view!.Show(TestStep, problem is null ? "Vamos testar a imagem" : "A imagem não foi confirmada", text,
        [
            Action(problem is null ? "Testar agora" : "Tentar de novo", ActionStyle.Primary, () => _ = TestAsync(), isDefault: true),
            Action("Pular", ActionStyle.Secondary, () => Go(TvStep)),
        ]);
    }

    private async Task TestAsync()
    {
        if (_busy)
        {
            return;
        }

        _busy = true;
        try
        {
            var (seen, problem) = await _shell.TestGameScreenAsync().ConfigureAwait(true);
            if (seen)
            {
                Go(TvStep);
            }
            else
            {
                ShowTest(problem ?? "A imagem não foi confirmada");
                _shell.RequestFocus(null);
            }
        }
        finally
        {
            _busy = false;
        }
    }

    private void ShowTv()
    {
        var tv = _shell.Settings.TvControl;
        string text = tv.IsActive
            ? $"A TV já está configurada ({ConsoleFeatures.Tv(tv).Summary}). Ela liga e troca para a entrada do PC ao abrir um jogo."
            : "O Tako pode ligar a TV e trocar para a entrada do PC sozinho ao abrir um jogo (Google TV, LG ou Samsung, pela rede).";
        _view!.Show(TvStep, "Ligar a TV junto com o jogo?", text,
        [
            Action(tv.IsActive ? "Mudar a TV" : "Configurar a TV", ActionStyle.Primary, OpenTv, isDefault: true),
            Action(tv.IsActive ? "Continuar" : "Pular", ActionStyle.Secondary, () => Go(AudioStep)),
        ]);
    }

    private void OpenTv()
    {
        // O editor da TV abre por cima do assistente; ao salvar ou cancelar, o assistente segue para o áudio.
        int before = _shell.Overlays.Count;
        _shell.OpenTvSettingsCommand.Execute(null);
        if (_shell.Overlays.Count > before && _shell.TopOverlay is TvSettingsViewModel editor)
        {
            void OnClosed(object? sender, System.Collections.Specialized.NotifyCollectionChangedEventArgs e)
            {
                if (!_shell.Overlays.Contains(editor))
                {
                    _shell.Overlays.CollectionChanged -= OnClosed;
                    if (_view is not null && _shell.Overlays.Contains(_view))
                    {
                        Go(AudioStep);
                    }
                }
            }

            _shell.Overlays.CollectionChanged += OnClosed;
        }
    }

    private void ShowAudio()
    {
        var settings = _shell.Settings;
        IReadOnlyList<AudioDevice> outputs;
        try
        {
            outputs = _shell.AudioVolume.CanChangeOutput ? _shell.AudioVolume.Outputs() : [];
        }
        catch (RuntimeEnvironmentException)
        {
            outputs = [];
        }

        if (outputs.Count == 0)
        {
            _view!.Show(AudioStep, "Som do jogo", "Não foi possível ler as saídas de áudio do Windows; o som fica na saída atual.",
                [Action("Continuar", ActionStyle.Primary, () => Go(DoneStep), isDefault: true)]);
            return;
        }

        string? suggested = settings.GameAudioOutput?.DeviceId
            ?? ConsoleFeatures.SuggestAudioOutput(settings.GameScreenName, outputs.Select(o => (o.Id, o.Name)));
        var actions = outputs.Select(o =>
        {
            bool isSuggested = string.Equals(o.Id, suggested, StringComparison.OrdinalIgnoreCase);
            return Action(o.Name, isSuggested ? ActionStyle.Primary : ActionStyle.Secondary, () =>
            {
                settings.GameAudioOutput = new AudioOutputChoice(o.Id, o.Name);
                Go(DoneStep);
            }, isDefault: isSuggested);
        }).ToList();
        actions.Add(Action("Não alterar o som", ActionStyle.Secondary, () =>
        {
            settings.GameAudioOutput = null;
            Go(DoneStep);
        }, isDefault: suggested is null));
        _view!.Show(AudioStep, "Por onde sai o som do jogo?", "Ao abrir um jogo, o som passa a sair por aqui; ao fechar, volta para a saída de antes.", actions);
    }

    private void ShowDone()
    {
        var settings = _shell.Settings;
        var lines = new List<string>
        {
            $"• Tela de jogo: {(settings.GameScreenEnabled ? settings.GameScreenName : "desligada")}",
            $"• TV: {(settings.TvControl.IsActive ? ConsoleFeatures.Tv(settings.TvControl).Summary : "o Tako não mexe na TV")}",
            $"• Som: {settings.GameAudioOutput?.Name ?? "não muda"}",
        };
        _view!.Show(DoneStep, "Tudo pronto", "Ao abrir um jogo, o Tako faz isto sozinho e desfaz ao fechar:\n" + string.Join('\n', lines) + "\nVocê pode mudar cada parte nos cartões do Modo Console.",
            [Action("Concluir", ActionStyle.Primary, Close, isDefault: true)]);
    }

    private void Close()
    {
        if (_view is not null && _shell.Overlays.Contains(_view))
        {
            _shell.CloseOverlay(_view);
        }

        page.Refresh();
    }

    private static OverlayAction Action(string label, ActionStyle style, Action execute, bool isDefault = false) =>
        new(label, style, new RelayCommand(execute), isDefault);
}
