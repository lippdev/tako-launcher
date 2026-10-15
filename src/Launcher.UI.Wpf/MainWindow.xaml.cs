using System.Collections.Specialized;
using System.ComponentModel;
using System.Diagnostics;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Threading;
using Launcher.Core.Input;
using Launcher.Core.Navigation;
using Launcher.UI.Wpf.Controls;
using Launcher.UI.Wpf.Input;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf;

/// <summary>
/// Janela do launcher. Lógica de view apenas: entrada (teclado, controle, mouse) vira <see cref="AppCommand"/>,
/// o <see cref="FocusNavigator"/> cuida do foco, e o estado fica no <see cref="ShellViewModel"/>.
/// </summary>
public partial class MainWindow : Window
{
    private static readonly Dictionary<Key, AppCommand> KeyMap = new()
    {
        [Key.Up] = AppCommand.Up,
        [Key.Down] = AppCommand.Down,
        [Key.Left] = AppCommand.Left,
        [Key.Right] = AppCommand.Right,
        [Key.Enter] = AppCommand.Accept,
        [Key.Space] = AppCommand.Accept,
        [Key.Escape] = AppCommand.Back,
        [Key.Back] = AppCommand.Back,
        [Key.F] = AppCommand.Favorite,
        [Key.Oem2] = AppCommand.Search,
        [Key.Divide] = AppCommand.Search,
        [Key.Q] = AppCommand.PreviousSection,
        [Key.E] = AppCommand.NextSection,
        [Key.M] = AppCommand.Menu,
    };

    /// <summary>Altura do palco em unidades de layout; a largura varia com a proporção da tela.</summary>
    private const double StageHeight = 1080;
    private const double MinStageWidth = 1440; // 4:3
    private const double MaxStageWidth = 3840; // 32:9

    private readonly ShellViewModel _shell;
    private readonly ILogger<MainWindow> _logger;
    private readonly ArtService _art;
    private readonly FocusNavigator _navigator;
    private readonly GamepadInterpreter _gamepad = new();
    private readonly SonyHidGamepad _sony;
    private readonly Stopwatch _clock = Stopwatch.StartNew();
    private readonly DispatcherTimer _gamepadTimer = new(DispatcherPriority.Input) { Interval = TimeSpan.FromMilliseconds(16) };
    private readonly Stack<FrameworkElement?> _focusBeforeOverlay = new();
    private long _wheelLockedUntil;
    private FrameworkElement? _draggingSlider;
    private Point? _lastMouse;

    public MainWindow(ShellViewModel shell, ArtService art, SonyHidGamepad sony, ILogger<MainWindow> logger)
    {
        InitializeComponent();
        _shell = shell;
        _logger = logger;
        _art = art;
        _sony = sony;
        SizeChanged += (_, _) => FitStage();
        DpiChanged += (_, _) => FitStage();
        DataContext = shell;
        _navigator = new FocusNavigator(Stage, Scopes, () => shell.Settings.PageDuration);
        _navigator.FocusChanged += OnNavigatorFocusChanged;
        _shell.FocusRequested += (_, e) => AfterLayout(() => { PositionPopover(); _navigator.FocusDefault(e.Key); });
        _shell.CommandRequested += (_, command) => Handle(command, _shell.InputKind);
        _shell.Overlays.CollectionChanged += OnOverlaysChanged;
        _shell.PropertyChanged += OnShellPropertyChanged;
        _shell.Settings.PropertyChanged += (_, _) => Backdrop.Transition = _shell.Settings.ColorDuration;

        PreviewKeyDown += OnPreviewKeyDown;
        PreviewMouseMove += OnPreviewMouseMove;
        PreviewMouseWheel += OnPreviewMouseWheel;
        PreviewMouseLeftButtonDown += OnPreviewMouseDown;
        PreviewMouseLeftButtonUp += (_, _) => _draggingSlider = null;
        PreviewGotKeyboardFocus += (_, e) =>
        {
            if (e.NewFocus is FrameworkElement element)
            {
                _navigator.Sync(element);
            }
        };
        _gamepadTimer.Tick += (_, _) => PollGamepad();
        Closed += (_, _) => _gamepadTimer.Stop();
        Dispatcher.UnhandledException += OnDispatcherUnhandledException;
        Loaded += (_, _) =>
        {
            Backdrop.Transition = _shell.Settings.ColorDuration;
            _gamepadTimer.Start();
            AfterLayout(() => _navigator.FocusDefault(null));
        };
    }

    private BackdropView Backdrop => Stage.Children.OfType<BackdropView>().First();

    /// <summary>Largura do palco na proporção da janela e densidade real (pixels físicos por unidade) para as imagens.</summary>
    private void FitStage()
    {
        if (ActualWidth <= 0 || ActualHeight <= 0)
        {
            return;
        }

        Stage.Width = Math.Clamp(StageHeight * ActualWidth / ActualHeight, MinStageWidth, MaxStageWidth);
        var dpi = VisualTreeHelper.GetDpi(this);
        _art.RenderScale = Math.Min(ActualHeight * dpi.DpiScaleY / StageHeight, ActualWidth * dpi.DpiScaleX / Stage.Width);
    }

    /// <summary>Exposto para o teste de fumaça (navegação real sobre a árvore visual).</summary>
    internal FocusNavigator Navigator => _navigator;

    internal FrameworkElement StageElement => Stage;

    internal void Dispatch(AppCommand command) => Handle(command, InputKind.Gamepad);

    // ---------------------------------------------------------------- Comandos

    private void Handle(AppCommand command, InputKind source)
    {
        _shell.InputKind = source;
        if (command == AppCommand.Guide)
        {
            _shell.Guide();
            return;
        }

        // Durante o jogo, só o Guia pertence ao launcher.
        if (_shell.IsInGame)
        {
            return;
        }

        var focused = _navigator.Focused;
        bool noOverlay = _shell.TopOverlay is null;
        switch (command)
        {
            case AppCommand.Left or AppCommand.Right:
                var direction = command == AppCommand.Left ? NavDirection.Left : NavDirection.Right;
                if (focused is not null && Nav.GetAdjustCommand(focused) is { } adjust)
                {
                    adjust.Execute(command == AppCommand.Left ? -1 : 1);
                }
                else if (_shell.TopOverlay is IOverlayViewModel overlay && overlay.HandleHorizontal(direction))
                {
                    // O painel tratou (ex.: acompanhar o ícone vizinho).
                }
                else
                {
                    _navigator.Move(direction);
                }

                break;
            case AppCommand.Up:
                _navigator.Move(NavDirection.Up);
                break;
            case AppCommand.Down:
                _navigator.Move(NavDirection.Down);
                break;
            case AppCommand.Accept:
                _navigator.Activate();
                break;
            case AppCommand.Back:
                _shell.Back();
                break;
            case AppCommand.Favorite when noOverlay && focused?.DataContext is GameItemViewModel game:
                _shell.ToggleFavorite(game);
                break;
            case AppCommand.Search when noOverlay:
                _shell.Toast("Busca com teclado virtual chega no UX-001 (pós-MVP)");
                break;
            case AppCommand.PreviousSection:
                _shell.CycleSection(-1);
                break;
            case AppCommand.NextSection:
                _shell.CycleSection(1);
                break;
            case AppCommand.Menu when noOverlay:
                _shell.OpenSettings();
                break;
        }
    }

    private void OnPreviewKeyDown(object sender, KeyEventArgs e)
    {
        var key = e.Key == Key.System ? e.SystemKey : e.Key;
        if (key == Key.Tab)
        {
            // Shift+Tab abre a Central de Controle (como a sobreposição da Steam); Tab sozinho não navega.
            e.Handled = true;
            if (Keyboard.Modifiers.HasFlag(ModifierKeys.Shift))
            {
                Handle(AppCommand.Guide, InputKind.Keyboard);
            }

            return;
        }

        // Campo de texto (ex.: editar jogo): letras, espaço, ← → e Backspace são do campo; ↑ ↓ e Esc navegam e Enter vai ao próximo.
        if (e.OriginalSource is TextBox)
        {
            if (key is not (Key.Up or Key.Down or Key.Enter or Key.Escape))
            {
                return;
            }

            if (key == Key.Enter)
            {
                e.Handled = true;
                Handle(AppCommand.Down, InputKind.Keyboard);
                return;
            }
        }

        if (KeyMap.TryGetValue(key, out var command))
        {
            e.Handled = true;
            Handle(command, InputKind.Keyboard);
        }
    }

    private void PollGamepad()
    {
        // Xbox/XInput primeiro; sem ele, controles PlayStation por HID.
        GamepadSnapshot? snapshot = XInputGamepad.ReadFirst()?.Snapshot;
        string? name = snapshot is null ? null : "Controle Xbox";
        if (snapshot is null && _sony.Read() is { } sony)
        {
            snapshot = sony;
            name = _sony.Model == SonyController.DualSense ? "Controle PS5 (DualSense)" : "Controle PS4 (DualShock 4)";
        }

        if (name != _shell.ControllerName)
        {
            _gamepad.Reset(); // MVP-031: o que estava pressionado no controle anterior não continua valendo
            _shell.ControllerName = name;
            _shell.IsPlayStationPad = name?.StartsWith("Controle PS", StringComparison.Ordinal) == true;
            _shell.Toast(name is null ? "Controle desconectado — o teclado continua funcionando" : $"Controle conectado: {name}");
        }

        if (snapshot is not { } pad || !IsActive)
        {
            // Sem controle ou sem foco: ao voltar, botões ainda pressionados só valem depois de soltos.
            _gamepad.Reset();
            return;
        }

        foreach (var command in _gamepad.Update(pad, _clock.Elapsed))
        {
            Handle(command, InputKind.Gamepad);
        }
    }

    // ---------------------------------------------------------------- Mouse

    /// <summary>Passar o mouse foca sem rolar a tela.</summary>
    private void OnPreviewMouseMove(object sender, MouseEventArgs e)
    {
        if (_draggingSlider is not null && e.LeftButton == MouseButtonState.Pressed)
        {
            SetSliderFromPointer(_draggingSlider, e);
            return;
        }

        // O WPF gera "mouse move" quando o layout muda sob o cursor parado: só foca com movimento real.
        var position = e.GetPosition(this);
        bool moved = _lastMouse is { } last && Math.Abs(last.X - position.X) + Math.Abs(last.Y - position.Y) > 2;
        _lastMouse = position;
        if (!moved)
        {
            return;
        }

        if (FindNavigable(e.OriginalSource as DependencyObject) is { } element && element != _navigator.Focused && _navigator.Navigables().Contains(element))
        {
            _shell.InputKind = InputKind.Mouse;
            _navigator.Focus(element, scroll: false);
        }
    }

    private void OnPreviewMouseDown(object sender, MouseButtonEventArgs e)
    {
        // Barra de volume: clique e arraste definem o valor.
        if (FindNavigable(e.OriginalSource as DependencyObject) is ButtonBase { DataContext: PanelRowViewModel { IsSlider: true } } slider)
        {
            _draggingSlider = slider;
            SetSliderFromPointer(slider, e);
            e.Handled = true;
        }
    }

    private void OnPreviewMouseWheel(object sender, MouseWheelEventArgs e)
    {
        e.Handled = true;
        if (Environment.TickCount64 < _wheelLockedUntil)
        {
            return;
        }

        _wheelLockedUntil = Environment.TickCount64 + 140;
        bool horizontal = Keyboard.Modifiers.HasFlag(ModifierKeys.Shift);
        var command = (horizontal, e.Delta < 0) switch
        {
            (true, true) => AppCommand.Right,
            (true, false) => AppCommand.Left,
            (false, true) => AppCommand.Down,
            _ => AppCommand.Up,
        };
        Handle(command, InputKind.Mouse);
    }

    private static void SetSliderFromPointer(FrameworkElement slider, MouseEventArgs e)
    {
        if (slider.DataContext is not PanelRowViewModel { SetFractionCommand: { } set } || FindByTag(slider, "bar") is not FrameworkElement bar)
        {
            return;
        }

        set.Execute(e.GetPosition(bar).X / bar.ActualWidth);
    }

    private static FrameworkElement? FindNavigable(DependencyObject? source)
    {
        for (var d = source; d is not null; d = d is Visual or System.Windows.Media.Media3D.Visual3D ? VisualTreeHelper.GetParent(d) : LogicalTreeHelper.GetParent(d))
        {
            if (d is FrameworkElement fe && Nav.GetGroup(fe) is not null)
            {
                return fe;
            }
        }

        return null;
    }

    private static DependencyObject? FindByTag(DependencyObject root, string tag)
    {
        for (int i = 0; i < VisualTreeHelper.GetChildrenCount(root); i++)
        {
            var child = VisualTreeHelper.GetChild(root, i);
            if (child is FrameworkElement { Tag: string t } && t == tag)
            {
                return child;
            }

            if (FindByTag(child, tag) is { } found)
            {
                return found;
            }
        }

        return null;
    }

    // ---------------------------------------------------------------- Foco e sobreposições

    /// <summary>Escopo da navegação: a sobreposição do topo, ou barra lateral + página.</summary>
    private IReadOnlyList<FrameworkElement> Scopes()
    {
        if (_shell.TopOverlay is { } top && OverlayHost.ItemContainerGenerator.ContainerFromItem(top) is FrameworkElement container)
        {
            return [container];
        }

        // Tema Playnite na tela inicial: só os elementos do tema (Menu/☰ continua levando às Configurações).
        if (_shell.IsPlayniteUiVisible)
        {
            return [PlayniteHost];
        }

        return [Sidebar, PageHost];
    }

    private void OnNavigatorFocusChanged(object? sender, FrameworkElement? element)
    {
        _shell.OnFocusChanged(element?.DataContext, element is null ? null : Nav.GetGroup(element), element is not null && Nav.GetAdjustCommand(element) is not null);
    }

    private void OnOverlaysChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        switch (e.Action)
        {
            case NotifyCollectionChangedAction.Add:
                _focusBeforeOverlay.Push(_navigator.Focused);
                AfterLayout(() => _navigator.FocusDefault(null));
                break;
            case NotifyCollectionChangedAction.Remove:
                var previous = _focusBeforeOverlay.Count > 0 ? _focusBeforeOverlay.Pop() : null;
                AfterLayout(() =>
                {
                    if (previous is not null && previous.IsVisible && _navigator.Navigables().Contains(previous))
                    {
                        _navigator.Focus(previous);
                    }
                    else
                    {
                        _navigator.FocusDefault(null);
                    }
                });
                break;
            case NotifyCollectionChangedAction.Reset:
                _focusBeforeOverlay.Clear();
                AfterLayout(() => _navigator.FocusDefault(null));
                break;
        }
    }

    private void OnShellPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(ShellViewModel.IsSidebarExpanded):
                var to = new Thickness(_shell.IsSidebarExpanded ? 400 : 120, 0, 0, 0);
                ContentArea.BeginAnimation(MarginProperty, new ThicknessAnimation(to, _shell.Settings.PageDuration) { EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut } });
                BrandLogo.BeginAnimation(OpacityProperty, new DoubleAnimation(_shell.IsSidebarExpanded ? 1 : 0, _shell.Settings.PageDuration));
                BrandMark.BeginAnimation(OpacityProperty, new DoubleAnimation(_shell.IsSidebarExpanded ? 0 : 1, _shell.Settings.PageDuration));
                break;
            case nameof(ShellViewModel.InputKind) or nameof(ShellViewModel.ControllerName):
                InputIcon.Key = _shell.ControllerName is null ? "Icon.Keyboard" : "Icon.Gamepad";
                break;
        }
    }

    /// <summary>Alinha o painel da Central de Controle sob o ícone que o abriu.</summary>
    private void PositionPopover()
    {
        if (_shell.TopOverlay is not ControlCenterViewModel { Popover: { } popover } ||
            OverlayHost.ItemContainerGenerator.ContainerFromItem(_shell.TopOverlay) is not FrameworkElement container)
        {
            return;
        }

        container.UpdateLayout();
        var icon = _navigator.Navigables().Concat(AllNavigables(container)).FirstOrDefault(e => FocusNavigator.MatchesKey(e, $"bar:{popover.Key}"));
        var panel = FindModal(container);
        if (icon is null || panel is null)
        {
            return;
        }

        double center = icon.TransformToAncestor(Stage).Transform(new Point(icon.ActualWidth / 2, 0)).X;
        panel.Margin = new Thickness(Math.Clamp(center - 300, 72, Math.Max(72, Stage.ActualWidth - 72 - 600)), 160, 0, 0);
    }

    private static IEnumerable<FrameworkElement> AllNavigables(DependencyObject root)
    {
        for (int i = 0; i < VisualTreeHelper.GetChildrenCount(root); i++)
        {
            var child = VisualTreeHelper.GetChild(root, i);
            if (child is FrameworkElement fe && Nav.GetGroup(fe) is not null)
            {
                yield return fe;
            }

            foreach (var nested in AllNavigables(child))
            {
                yield return nested;
            }
        }
    }

    private static FrameworkElement? FindModal(DependencyObject root)
    {
        for (int i = 0; i < VisualTreeHelper.GetChildrenCount(root); i++)
        {
            var child = VisualTreeHelper.GetChild(root, i);
            if (child is FrameworkElement fe && Nav.GetIsModalScope(fe))
            {
                return fe;
            }

            if (FindModal(child) is { } found)
            {
                return found;
            }
        }

        return null;
    }

    private void AfterLayout(Action action) => Dispatcher.BeginInvoke(action, DispatcherPriority.Loaded);

    /// <summary>
    /// Rede de segurança (MVP-021): um erro de XAML de tema que escape da validação e da carga não derruba o launcher;
    /// a prévia é fechada e a tela de recuperação explica o motivo.
    /// </summary>
    private void OnDispatcherUnhandledException(object sender, DispatcherUnhandledExceptionEventArgs e)
    {
        LogUnhandledException(_logger, e.Exception); // MVP-033: toda exceção não tratada da UI fica no log
        if (e.Exception is not System.Windows.Markup.XamlParseException xaml)
        {
            return;
        }

        e.Handled = true;
        _shell.RecoverFromThemeFailure(xaml.Message);
    }

    [LoggerMessage(Level = LogLevel.Error, Message = "Exceção não tratada na UI")]
    private static partial void LogUnhandledException(ILogger logger, Exception exception);
}
