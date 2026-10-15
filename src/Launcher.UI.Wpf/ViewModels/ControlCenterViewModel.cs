using System.Collections.ObjectModel;
using System.Windows;
using System.Windows.Media;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Navigation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Central de Controle (menu rápido do jogo): cartões vivos, barra de ícones no topo e painel por ícone.
/// Comportamento descrito no DESIGN.md › Components › Menu rápido.
/// </summary>
public sealed partial class ControlCenterViewModel : ObservableObject, IOverlayViewModel, IDisposable
{
    /// <summary>Painéis que acompanham ← →; Encerrar nunca abre deslizando.</summary>
    private static readonly string[] SlidePopovers = ["perf", "audio", "controller", "display"];

    private readonly ShellViewModel _shell;
    private readonly GameSessionViewModel _session;
    private readonly SessionOptions _options;
    private readonly PerformanceSimulator _performance;

    [ObservableProperty]
    private ControlCenterPopover? _popover;

    public ControlCenterViewModel(ShellViewModel shell, GameSessionViewModel session)
    {
        _shell = shell;
        _session = session;
        _options = session.Options;
        _performance = session.Performance;
        Bar =
        [
            new("home", "Início", "Icon.Home"),
            new("game", "Voltar ao jogo", null, IsGame: true),
            new("sep1", string.Empty, null, IsSeparator: true),
            new("capture", "Capturar tela", "Icon.Camera"),
            new("perf", "Desempenho", "Icon.Gauge"),
            new("audio", "Áudio", "Icon.Volume"),
            new("controller", "Controle", "Icon.Gamepad"),
            new("display", "Tela", "Icon.Monitor"),
            new("sep2", string.Empty, null, IsSeparator: true),
            new("quit", "Encerrar jogo", "Icon.Power", IsDanger: true),
        ];
        Cards = [new("game"), new("capture"), new("perf"), new("audio"), new("controller")];
        _performance.Sampled += OnSampled;
        UpdateCards();
    }

    public GameItemViewModel Game => _session.Game;

    public IReadOnlyList<BarItemViewModel> Bar { get; }

    public ObservableCollection<CardViewModel> Cards { get; }

    public bool IsPopoverOpen => Popover is not null;

    public bool HandleBack()
    {
        if (Popover is null)
        {
            return false;
        }

        ClosePopover();
        return true;
    }

    public bool HandleHorizontal(NavDirection direction)
    {
        if (Popover is null)
        {
            return false;
        }

        var items = Bar.Where(b => !b.IsSeparator).ToList();
        int i = items.FindIndex(b => b.Key == Popover.Key) + (direction == NavDirection.Right ? 1 : -1);
        if (i < 0 || i >= items.Count)
        {
            return true;
        }

        var next = items[i];
        ClosePopover(focusIcon: false);
        if (SlidePopovers.Contains(next.Key))
        {
            OpenPopover(next.Key);
        }
        else
        {
            _shell.RequestFocus($"bar:{next.Key}");
        }

        return true;
    }

    public void Dispose() => _performance.Sampled -= OnSampled;

    partial void OnPopoverChanged(ControlCenterPopover? value) => OnPropertyChanged(nameof(IsPopoverOpen));

    [RelayCommand]
    private void ActivateBar(string key)
    {
        switch (key)
        {
            case "home":
                _shell.GoHomeKeepingGame();
                break;
            case "game":
                _shell.CloseOverlay(this);
                break;
            case "capture":
                Capture();
                break;
            default:
                OpenPopover(key);
                break;
        }
    }

    [RelayCommand]
    private void ActivateCard(string key)
    {
        switch (key)
        {
            case "game":
                _shell.CloseOverlay(this);
                break;
            case "capture":
                Capture();
                break;
            case "perf":
                _options.PerfMode = (_options.PerfMode + 1) % SessionOptions.PerfModes.Length;
                UpdateCards();
                break;
            default:
                OpenPopover(key);
                break;
        }
    }

    private void Capture()
    {
        _options.Captures++;
        _shell.CloseOverlay(this);
        _shell.Toast("Captura salva em Imagens › UltraLauncher");
    }

    private void OpenPopover(string key)
    {
        Popover = new ControlCenterPopover(key, BuildSections(key));
        _shell.RequestFocus(null);
    }

    private void ClosePopover(bool focusIcon = true)
    {
        string? key = Popover?.Key;
        Popover = null;
        if (focusIcon && key is not null)
        {
            _shell.RequestFocus($"bar:{key}");
        }
    }

    /// <summary>Escolher uma opção aplica, confirma e fecha o painel.</summary>
    private void Choose(string message)
    {
        UpdateCards();
        ClosePopover();
        _shell.Toast(message);
    }

    private IReadOnlyList<PanelSection> BuildSections(string key)
    {
        PanelRowViewModel Radio(string label, bool on, Action apply, string message) => new()
        {
            Label = label,
            IsRadio = true,
            IsOn = on,
            Command = new RelayCommand(() => { apply(); Choose(message); }),
        };

        switch (key)
        {
            case "perf":
                return [new("Sobreposição de desempenho", "Aparece no canto superior esquerdo do jogo.",
                    SessionOptions.PerfModes.Select((label, i) => Radio(label, _options.PerfMode == i, () => _options.PerfMode = i, $"HUD de desempenho: {label}")).ToList())];
            case "audio":
                PanelRowViewModel? slider = null;
                slider = new PanelRowViewModel
                {
                    Label = "Volume",
                    IconKey = "Icon.Volume",
                    IsSlider = true,
                    Fraction = _options.Volume / 100d,
                    EndText = $"{_options.Volume}%",
                    Command = new RelayCommand(() => { }),
                    AdjustCommand = new RelayCommand<object?>(delta =>
                    {
                        int step = delta is int d ? d : 0;
                        _options.Volume = Math.Clamp(_options.Volume + (step * 5), 0, 100);
                        slider!.Fraction = _options.Volume / 100d; // Atribuído antes de qualquer ajuste.
                        slider.EndText = $"{_options.Volume}%";
                        UpdateCards();
                    }),
                    SetFractionCommand = new RelayCommand<double>(fraction =>
                    {
                        _options.Volume = (int)Math.Round(Math.Clamp(fraction, 0, 1) * 20) * 5;
                        slider!.Fraction = _options.Volume / 100d;
                        slider.EndText = $"{_options.Volume}%";
                        UpdateCards();
                    }),
                };
                return
                [
                    new("Volume", null, [slider]),
                    new("Saída de áudio", null, SessionOptions.Outputs.Select((label, i) => Radio(label, _options.Output == i, () => _options.Output = i, $"Áudio saindo por: {label}")).ToList()),
                ];
            case "controller":
                return
                [
                    new("Conectados", null, [new PanelRowViewModel { Label = _shell.ControllerName ?? "Nenhum controle conectado", IconKey = "Icon.Gamepad", IsBattery = _shell.ControllerName is not null, EndText = _shell.ControllerName is null ? "use o teclado" : "XInput", Command = new RelayCommand(() => _shell.Toast("Bateria e remapeamento chegam com o GameInput (MVP-031)")) }]),
                    new("Ajustes", null, [new PanelRowViewModel { Label = "Remapear botões", EndText = "pós-MVP", Command = new RelayCommand(() => _shell.Toast("Remapeamento por jogo chega depois do MVP")) }]),
                ];
            case "display":
                var hdr = new PanelRowViewModel { Label = "HDR", IsToggle = true, IsOn = _options.Hdr, Command = new RelayCommand(() => { _options.Hdr = !_options.Hdr; Choose(_options.Hdr ? "HDR ligado" : "HDR desligado"); }) };
                return
                [
                    new("Resolução", null, SessionOptions.Resolutions.Select((label, i) => Radio(label, _options.Resolution == i, () => _options.Resolution = i, $"Resolução: {label}")).ToList()),
                    new("Imagem", "O Console Runtime restaura a configuração original ao encerrar a sessão.", [hdr]),
                ];
            default:
                return
                [
                    new("Encerrar jogo", $"Progresso não salvo de {Game.Title} pode ser perdido. Monitores e áudio voltam ao estado anterior à sessão.",
                    [
                        new PanelRowViewModel { Label = "Encerrar jogo", IconKey = "Icon.Power", IsDanger = true, Command = new RelayCommand(_shell.EndSession) },
                        new PanelRowViewModel { Label = "Cancelar", Command = new RelayCommand(() => ClosePopover()) },
                    ]),
                ];
        }
    }

    private void OnSampled(object? sender, EventArgs e) => UpdatePerfCard();

    private void UpdateCards()
    {
        var mins = Math.Max(1, (int)(DateTimeOffset.UtcNow - _session.Started).TotalMinutes);
        var game = Cards[0];
        game.Caption = $"Jogando agora · {Game.StoreName}";
        game.Value = Game.Title;
        game.Pill = $"Sessão de {mins} min · {Game.PlayTimeText} no total";
        game.Background = _session.Frame;

        var capture = Cards[1];
        capture.TagIconKey = "Icon.Camera";
        capture.Caption = _options.Captures == 0 ? "Nenhuma captura ainda" : $"{_options.Captures} {(_options.Captures == 1 ? "captura" : "capturas")} nesta sessão";
        capture.Value = _options.Captures == 0 ? "Capturar tela" : "Captura de agora há pouco";
        capture.Background = _options.Captures == 0 ? null : _session.Frame;

        UpdatePerfCard();

        var audio = Cards[3];
        audio.Caption = $"Volume · {SessionOptions.Outputs[_options.Output]}";
        audio.Value = $"{_options.Volume}%";
        audio.Meter = _options.Volume / 100d;

        var pad = Cards[4];
        pad.Caption = _shell.ControllerName ?? "Nenhum controle conectado";
        pad.Value = _shell.ControllerName is null ? "Teclado" : "Conectado";
        pad.Pill = _shell.ControllerName is null ? "Conecte um controle Xbox" : "Bateria via GameInput (MVP-031)";
    }

    private void UpdatePerfCard()
    {
        var perf = Cards[2];
        perf.Caption = "Desempenho ao vivo";
        perf.Value = $"{_performance.Fps} FPS";
        perf.Pill = $"HUD: {SessionOptions.PerfModes[_options.PerfMode]} · simulado";
        var points = new PointCollection();
        int i = 0, n = Math.Max(1, _performance.History.Count - 1);
        foreach (int fps in _performance.History)
        {
            points.Add(new Point(i++ * 300d / n, 80 - ((fps - 52) / 12d * 72)));
        }

        points.Freeze();
        perf.Sparkline = points;
    }
}
