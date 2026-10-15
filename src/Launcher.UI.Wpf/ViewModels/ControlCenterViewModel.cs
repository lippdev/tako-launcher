using System.Collections.ObjectModel;
using System.Globalization;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.SessionTools;
using Launcher.Core.Navigation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Central de Controle (menu rápido do jogo): cartões vivos, janelas abertas, barra de ícones no topo e painel por
/// ícone. Comportamento descrito no DESIGN.md › Components › Menu rápido. Janelas, contador de FPS (PresentMon, ADR-0017),
/// limite de FPS (driver de vídeo, ADR-0018), ponteiro, ControlFS e desempenho vêm do "menu de sessão" do Console Mode (<see cref="SessionToolsService"/>).
/// </summary>
public sealed partial class ControlCenterViewModel : ObservableObject, IOverlayViewModel, IDisposable
{
    /// <summary>Painéis que acompanham ← →; Encerrar nunca abre deslizando.</summary>
    private static readonly string[] SlidePopovers = ["perf", "audio", "controller", "display", "pointer", "files"];

    private const int HistoryLength = 40;
    private const int MaxWindows = 12;
    private const int IconSize = 64;

    private readonly ShellViewModel _shell;
    private readonly GameSessionViewModel _session;
    private readonly SessionOptions _options;
    private readonly SessionToolsService _tools;
    private readonly DispatcherTimer _sampler = new() { Interval = TimeSpan.FromSeconds(1) };
    private readonly Queue<double> _cpuHistory = new();
    private readonly List<(PanelRowViewModel Row, Func<HardwareSample, string> Text)> _liveRows = [];
    private HardwareSample? _sample;
    private PanelRowViewModel? _fpsRow;
    private bool _sampling;
    private bool _disposed;

    [ObservableProperty]
    private ControlCenterPopover? _popover;

    public ControlCenterViewModel(ShellViewModel shell, GameSessionViewModel session, SessionToolsService tools)
    {
        _shell = shell;
        _session = session;
        _options = session.Options;
        _tools = tools;
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
            new("pointer", "Ponteiro", "Icon.Pointer"),
            new("files", "ControlFS", "Icon.Folder"),
            new("sep2", string.Empty, null, IsSeparator: true),
            new("quit", "Encerrar jogo", "Icon.Power", IsDanger: true),
        ];
        Cards = [new("game"), new("capture"), new("perf"), new("audio"), new("controller")];
        // Com a Central aberta o controle navega o menu; o ponteiro pelo analógico volta quando ela fecha.
        tools.Cursor.IsControllerMousePaused = true;
        _sampler.Tick += (_, _) => _ = SampleAsync();
        _sampler.Start();
        _ = SampleAsync();
        _ = RefreshWindowsAsync();
        UpdateCards();
    }

    public GameItemViewModel Game => _session.Game;

    public IReadOnlyList<BarItemViewModel> Bar { get; }

    public ObservableCollection<CardViewModel> Cards { get; }

    /// <summary>Janelas abertas (as do Alt+Tab), da frente para trás.</summary>
    public ObservableCollection<WindowCardViewModel> Windows { get; } = [];

    public bool HasWindows => Windows.Count > 0;

    public bool IsRealGame => _session.IsRealGame;

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

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _sampler.Stop();
        _tools.Cursor.IsControllerMousePaused = false;
    }

    /// <summary>X numa janela: pede para ela fechar, como o X dela (o programa pode perguntar se quer salvar).</summary>
    public void CloseWindow(WindowCardViewModel window)
    {
        ArgumentNullException.ThrowIfNull(window);
        if (_tools.Windows.RequestClose(window.Window.Handle))
        {
            // O foco passa para a janela vizinha (ou volta ao cartão do jogo quando não sobra nenhuma).
            int index = Windows.IndexOf(window);
            Windows.Remove(window);
            OnPropertyChanged(nameof(HasWindows));
            _shell.Toast($"Pedimos para {window.ProcessName} fechar");
            _shell.RequestFocus(Windows.Count > 0 ? Windows[Math.Clamp(index, 0, Windows.Count - 1)].NavKey : "card:game");
        }
        else
        {
            _shell.Toast($"{window.ProcessName} não pôde ser fechado");
        }

        // Quem perguntou "salvar?" continua aberto: a lista confere de novo logo depois.
        _ = RefreshWindowsAsync(TimeSpan.FromMilliseconds(800));
    }

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
            default:
                OpenPopover(key);
                break;
        }
    }

    [RelayCommand]
    private void ActivateWindow(WindowCardViewModel window) => _shell.SwitchToWindow(this, window.Window);

    private void Capture()
    {
        _options.Captures++;
        _shell.CloseOverlay(this);
        _shell.Toast("Captura salva em Imagens › Tako");
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
                return BuildPerfSections();
            case "audio":
                return BuildAudioSections();
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
            case "pointer":
                return BuildPointerSections();
            case "files":
                return BuildControlFsSections();
            default:
                string warning = IsRealGame
                    ? $"O Tako pede para {Game.Title} fechar, como o X da janela; se ele perguntar se quer salvar, responda no jogo. Monitores e áudio voltam ao estado anterior à sessão."
                    : $"Progresso não salvo de {Game.Title} pode ser perdido. Monitores e áudio voltam ao estado anterior à sessão.";
                return
                [
                    new("Encerrar jogo", warning,
                    [
                        new PanelRowViewModel { Label = "Encerrar jogo", IconKey = "Icon.Power", IsDanger = true, Command = new RelayCommand(() => _shell.EndSessionFromControlCenter(this)) },
                        new PanelRowViewModel { Label = "Cancelar", Command = new RelayCommand(() => ClosePopover()) },
                    ]),
                ];
        }
    }

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

        UpdateAudioCard();

        var pad = Cards[4];
        pad.Caption = _shell.ControllerName ?? "Nenhum controle conectado";
        pad.Value = _shell.ControllerName is null ? "Teclado" : "Conectado";
        pad.Pill = _shell.ControllerName is null ? "Conecte um controle Xbox" : "Bateria via GameInput (MVP-031)";
    }

    private void UpdatePerfCard()
    {
        var perf = Cards[2];
        perf.TagIconKey = null;
        perf.Caption = "Desempenho do PC";
        if (_sample is not { } sample)
        {
            perf.Value = "Medindo…";
            perf.Pill = null;
            perf.Sparkline = null;
            return;
        }

        perf.Value = $"CPU {Percent(sample.CpuUsage)}";
        string fps = _tools.CurrentFrames is { } frames ? $"{FpsOverlay.FpsText(frames)} FPS · " : string.Empty;
        perf.Pill = $"{fps}GPU {Percent(sample.GpuUsage)} · RAM {Percent(sample.RamUsage)}";
        var points = new PointCollection();
        int i = 0, n = Math.Max(1, _cpuHistory.Count - 1);
        foreach (double cpu in _cpuHistory)
        {
            points.Add(new Point(i++ * 300d / n, 80 - (Math.Clamp(cpu, 0, 100) / 100 * 72)));
        }

        points.Freeze();
        perf.Sparkline = points;
    }

    private static string Percent(double? value) => value is { } v ? string.Create(CultureInfo.InvariantCulture, $"{v:0}%") : "--";

    private static string GpuText(HardwareSample s)
    {
        string text = Percent(s.GpuUsage);
        return s.GpuTempC is { } temp ? string.Create(CultureInfo.InvariantCulture, $"{text} · {temp:0}°C") : text;
    }

    private static BitmapSource? ToImage(IconBitmap? icon)
    {
        if (icon is null || icon.Width <= 0 || icon.Height <= 0)
        {
            return null;
        }

        var image = BitmapSource.Create(icon.Width, icon.Height, 96, 96, PixelFormats.Pbgra32, null, icon.Pixels, icon.Width * 4);
        image.Freeze();
        return image;
    }

    /// <summary>Linha que troca de valor: A avança (voltando ao começo), ← → ajustam; o painel fica aberto.</summary>
    private static PanelRowViewModel CycleRow(string key, string label, string value, Action<int> change) => new()
    {
        Key = key,
        Label = label,
        EndText = value,
        Command = new RelayCommand(() => change(0)),
        AdjustCommand = new RelayCommand<object?>(delta => change(delta is int d ? d : 0)),
    };

    /// <summary>Próxima opção: <paramref name="direction"/> 0 (A) avança e volta ao começo; ← → param nas pontas.</summary>
    private static T Next<T>(IReadOnlyList<T> options, T current, int direction)
    {
        int i = Math.Max(0, options.ToList().IndexOf(current));
        return direction == 0 ? options[(i + 1) % options.Count] : options[Math.Clamp(i + Math.Sign(direction), 0, options.Count - 1)];
    }

    private static string StyleLabel(string style) => style switch
    {
        FpsOverlay.Off => "Desligado",
        FpsOverlay.Compact => "Compacto",
        FpsOverlay.Detailed => "Detalhado",
        _ => "Só FPS",
    };

    private static string PositionLabel(string position) => position switch
    {
        FpsOverlay.TopRight => "Superior direita",
        FpsOverlay.BottomLeft => "Inferior esquerda",
        FpsOverlay.BottomRight => "Inferior direita",
        _ => "Superior esquerda",
    };

    private static string SizeLabel(string size) => size switch
    {
        FpsOverlay.Medium => "Médio",
        FpsOverlay.Large => "Grande",
        _ => "Pequeno",
    };

    private string FpsNowText() => _tools.Fps.Status switch
    {
        FpsCounterStatus.Denied => "o Windows recusou · saia e entre de novo",
        FpsCounterStatus.Failed => "não foi possível medir",
        _ => FramesText(_tools.CurrentFrames),
    };

    /// <summary>"60 FPS · 1% low 48" para a linha viva do painel; funciona até em tela cheia exclusiva.</summary>
    private static string FramesText(FrameStats? frames) => frames is null
        ? "medindo…"
        : frames.OnePercentLow is { } low
            ? string.Create(CultureInfo.InvariantCulture, $"{frames.Fps:0} FPS · 1% low {low:0}")
            : string.Create(CultureInfo.InvariantCulture, $"{frames.Fps:0} FPS");

    /// <summary>Uma leitura por segundo fora da thread da UI; atualiza o cartão e as linhas "Agora" do painel.</summary>
    private async Task SampleAsync()
    {
        if (_sampling || _disposed)
        {
            return;
        }

        _sampling = true;
        try
        {
            var sample = await _tools.SampleAsync().ConfigureAwait(true);
            if (_disposed || sample is null)
            {
                return;
            }

            _sample = sample;
            if (sample.CpuUsage is { } cpu)
            {
                _cpuHistory.Enqueue(cpu);
                while (_cpuHistory.Count > HistoryLength)
                {
                    _cpuHistory.Dequeue();
                }
            }

            UpdatePerfCard();
            if (_fpsRow is not null)
            {
                _fpsRow.EndText = FpsNowText();
            }

            foreach (var (row, text) in _liveRows)
            {
                row.EndText = text(sample);
            }
        }
        finally
        {
            _sampling = false;
        }
    }

    /// <summary>Lista as janelas e os ícones fora da thread da UI (ler o ícone de um exe leva alguns ms).</summary>
    private async Task RefreshWindowsAsync(TimeSpan delay = default)
    {
        if (delay > TimeSpan.Zero)
        {
            await Task.Delay(delay).ConfigureAwait(true);
        }

        if (_disposed)
        {
            return;
        }

        var switcher = _tools.Windows;
        var found = await Task.Run(() => switcher.List().Take(MaxWindows).Select(w => (Window: w, Icon: ToImage(switcher.LoadIcon(w, IconSize)))).ToList()).ConfigureAwait(true);
        if (_disposed)
        {
            return;
        }

        // Mantém os cartões que continuam abertos (o foco não pula) e acrescenta os novos na ordem do Windows.
        var open = found.Select(f => f.Window.Handle).ToHashSet();
        foreach (var gone in Windows.Where(w => !open.Contains(w.Window.Handle)).ToList())
        {
            Windows.Remove(gone);
        }

        for (int i = 0; i < found.Count; i++)
        {
            var (window, icon) = found[i];
            if (Windows.All(w => w.Window.Handle != window.Handle))
            {
                Windows.Insert(Math.Min(i, Windows.Count), new WindowCardViewModel(window) { Icon = icon });
            }
        }

        OnPropertyChanged(nameof(HasWindows));
    }

    private void RebuildPopover(string focusKey)
    {
        if (Popover is not { } current)
        {
            return;
        }

        Popover = new ControlCenterPopover(current.Key, BuildSections(current.Key));
        _shell.RequestFocus(focusKey);
    }

    private IReadOnlyList<PanelSection> BuildPerfSections()
    {
        _liveRows.Clear();
        _fpsRow = null;
        PanelRowViewModel Live(string label, Func<HardwareSample, string> text)
        {
            var row = new PanelRowViewModel { Label = label, EndText = _sample is { } s ? text(s) : "--", Command = new RelayCommand(() => { }) };
            _liveRows.Add((row, text));
            return row;
        }

        var nowRows = new List<PanelRowViewModel>();
        var state = _tools.CounterState;
        if (state == FpsCounterState.Ready)
        {
            _fpsRow = new PanelRowViewModel { Key = "fps:now", Label = "FPS do jogo", EndText = FpsNowText(), Command = new RelayCommand(() => { }) };
            nowRows.Add(_fpsRow);
        }

        nowRows.Add(Live("Uso do PC", s => $"CPU {Percent(s.CpuUsage)} · GPU {GpuText(s)} · RAM {Percent(s.RamUsage)}"));
        var now = new PanelSection("Agora", null, nowRows);
        return [BuildCounterSection(state), BuildLimitSection(), now];
    }

    /// <summary>
    /// Contador de FPS. Ligar pede o UAC, que não aceita o controle: por isso ele é ligado nas Configurações, com
    /// teclado e mouse. Aqui, sem o contador pronto, as opções aparecem esmaecidas com o caminho para ligar.
    /// </summary>
    private PanelSection BuildCounterSection(FpsCounterState state)
    {
        string style = _tools.FpsStyle;
        var layout = _tools.Layout;
        if (state != FpsCounterState.Ready)
        {
            string why = state switch
            {
                FpsCounterState.NotBundled => "O contador de FPS não veio nesta versão do Tako.",
                FpsCounterState.NeedsSignOut => "Saia e entre no Windows de novo para o contador funcionar.",
                _ => "Ative o contador de FPS nas Configurações do Tako, com teclado e mouse.",
            };
            PanelRowViewModel Dimmed(string key, string label, string value) => new()
            {
                Key = key,
                Label = label,
                EndText = value,
                IsDimmed = true,
                Command = new RelayCommand(() => _shell.Toast(why)),
            };
            return new PanelSection("Contador de FPS", why,
            [
                Dimmed("fps:style", "Estilo", StyleLabel(style)),
                Dimmed("fps:position", "Posição", PositionLabel(layout.Position)),
                Dimmed("fps:size", "Tamanho", SizeLabel(layout.Size)),
            ]);
        }

        var counter = new List<PanelRowViewModel>
        {
            CycleRow("fps:style", "Estilo", StyleLabel(style), d =>
            {
                _tools.SetFpsStyle(Next(FpsOverlay.Styles, _tools.FpsStyle, d));
                RebuildPopover("fps:style");
            }),
        };
        if (style != FpsOverlay.Off)
        {
            counter.Add(CycleRow("fps:position", "Posição", PositionLabel(layout.Position), d =>
            {
                _tools.SetLayout(_tools.Layout with { Position = Next(FpsOverlay.Positions, _tools.Layout.Position, d) });
                RebuildPopover("fps:position");
            }));
            counter.Add(CycleRow("fps:size", "Tamanho", SizeLabel(layout.Size), d =>
            {
                _tools.SetLayout(_tools.Layout with { Size = Next(FpsOverlay.Sizes, _tools.Layout.Size, d) });
                RebuildPopover("fps:size");
            }));
            counter.Add(new PanelRowViewModel
            {
                Key = "fps:card",
                Label = "Fundo escuro",
                IsToggle = true,
                IsOn = layout.Card,
                Command = new RelayCommand(() =>
                {
                    _tools.SetLayout(_tools.Layout with { Card = !_tools.Layout.Card });
                    RebuildPopover("fps:card");
                }),
            });
        }

        return new PanelSection(
            "Contador de FPS",
            "Medido pelo Windows, sem tocar no jogo. Em tela cheia exclusiva o contador não aparece sobre o jogo; o FPS continua aqui.",
            counter);
    }

    /// <summary>Limite de FPS pelo driver de vídeo (ADR-0018): independe do contador.</summary>
    private PanelSection BuildLimitSection()
    {
        var caps = _tools.LimitCapabilities;
        if (caps is not { IsAvailable: true })
        {
            string reason = caps?.Explanation ?? "Verificando o driver de vídeo…";
            return new("Limite de FPS", "Feito pelo driver da placa de vídeo, sem programas extras.",
            [
                new PanelRowViewModel { Key = "fps:limit", Label = "Limite", EndText = caps is null ? "verificando" : "indisponível", Command = new RelayCommand(() => _shell.Toast(reason)) },
            ]);
        }

        var row = CycleRow("fps:limit", "Limite", FpsLimits.Label(_tools.FpsLimit, _tools.DisplayRefreshHz), d =>
        {
            _tools.SetFpsLimit(d == 0 ? FpsLimits.Cycle(_tools.FpsLimit) : FpsLimits.Step(_tools.FpsLimit, d));
            RebuildPopover("fps:limit");
        });
        string scope = caps.PerApplication ? "só neste jogo" : "em toda a placa enquanto você joga";
        return new("Limite de FPS", $"Pelo driver {GpuVendors.Name(caps.Vendor)}, {scope}. O valor que você tinha volta ao sair do jogo.", [row]);
    }

    private IReadOnlyList<PanelSection> BuildPointerSections()
    {
        var pointer = _tools.Cursor;
        return
        [
            new("Ponteiro do mouse", "Para jogos e emuladores que deixam o ponteiro na tela ou têm menus só de mouse.",
            [
                new PanelRowViewModel
                {
                    Key = "pointer:hide",
                    Label = "Esconder o ponteiro",
                    IsToggle = true,
                    IsOn = pointer.IsCursorHidden,
                    Command = new RelayCommand(() =>
                    {
                        pointer.SetCursorHidden(!pointer.IsCursorHidden);
                        RebuildPopover("pointer:hide");
                        _shell.Toast(pointer.IsCursorHidden ? "Ponteiro escondido até o fim da sessão" : "Ponteiro visível");
                    }),
                },
                new PanelRowViewModel
                {
                    Key = "pointer:stick",
                    Label = "Mover com o analógico",
                    IsToggle = true,
                    IsOn = pointer.IsControllerMouseOn,
                    Command = new RelayCommand(() =>
                    {
                        pointer.SetControllerMouse(!pointer.IsControllerMouseOn, 0);
                        RebuildPopover("pointer:stick");
                        _shell.Toast(pointer.IsControllerMouseOn ? "O analógico move o ponteiro quando você voltar ao jogo" : "Analógico de volta ao jogo");
                    }),
                },
            ]),
            new("Com o analógico ligado",
                "Esquerdo move · direito rola · A clica · A duas vezes: botão direito · segurar A: arrastar. Fica pausado enquanto a Central está aberta.",
                []),
        ];
    }

    private IReadOnlyList<PanelSection> BuildControlFsSections()
    {
        if (_tools.ControlFs.IsInstalled)
        {
            return
            [
                new("ControlFS", "Explorador de arquivos feito para o controle. Abre por cima do jogo; o Guia traz a Central de volta.",
                [
                    new PanelRowViewModel { Key = "files:open", Label = "Abrir o ControlFS", IconKey = "Icon.Folder", Command = new RelayCommand(() => _shell.OpenControlFs(this)) },
                ]),
            ];
        }

        return
        [
            new("ControlFS não instalado", "Explorador de arquivos para controle, gratuito, da mesma equipe do Tako. Instale e ele abre por aqui.",
            [
                new PanelRowViewModel
                {
                    Key = "files:download",
                    Label = "Baixar o ControlFS",
                    EndText = "abre o navegador",
                    Command = new RelayCommand(() => OpenPage(ControlFsLocator.ReleasesUrl, "Página do ControlFS aberta no navegador")),
                },
            ]),
        ];
    }

    private void OpenPage(string url, string message)
    {
        ClosePopover();
        _shell.Toast(_tools.OpenPage(url) ? message : "Não foi possível abrir o navegador");
    }
}
