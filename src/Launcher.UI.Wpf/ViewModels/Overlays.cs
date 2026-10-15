using System.Collections.ObjectModel;
using System.Windows.Input;
using System.Windows.Media;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Navigation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Sobreposição empilhada sobre o launcher (diálogo, jogo, Central de Controle...).</summary>
public interface IOverlayViewModel
{
    /// <summary>B/Esc. Retorna falso para a Shell fechar a sobreposição.</summary>
    bool HandleBack() => false;

    /// <summary>← → antes da navegação espacial (ex.: painel que acompanha o ícone vizinho).</summary>
    bool HandleHorizontal(NavDirection direction) => false;
}

public enum ActionStyle
{
    Primary,
    Secondary,
    Danger,
}

public sealed record OverlayAction(string Label, ActionStyle Style, ICommand Command, bool IsDefault = false);

public sealed class DialogViewModel(string title, string text, IReadOnlyList<OverlayAction> actions) : IOverlayViewModel
{
    public string Title { get; } = title;

    public string Text { get; } = text;

    public IReadOnlyList<OverlayAction> Actions { get; } = actions;
}

/// <summary>Tela de recuperação (MVP-021/036): tema rejeitado, o app segue no tema padrão.</summary>
public sealed class RecoveryViewModel(string detail, IReadOnlyList<OverlayAction> actions) : IOverlayViewModel
{
    public string Detail { get; } = detail;

    public IReadOnlyList<OverlayAction> Actions { get; } = actions;
}

/// <summary>"Iniciando" com a capa e a barra de progresso.</summary>
public sealed partial class LaunchOverlayViewModel(GameItemViewModel game, ICommand cancel) : ObservableObject, IOverlayViewModel
{
    public GameItemViewModel Game { get; } = game;

    public ICommand Cancel { get; } = cancel;

    public string Status { get; } = "Preparando a sessão: monitores, áudio e controle";
}

/// <summary>
/// Jogo em execução. Marco 0: a biblioteca é fictícia, então a "tela do jogo" é a arte em tela cheia; iniciar e
/// acompanhar processos reais é o MVP-026. Só o Guia é do launcher; o resto da entrada é do jogo.
/// </summary>
public sealed partial class GameSessionViewModel : ObservableObject, IOverlayViewModel
{
    [ObservableProperty]
    private ImageSource? _frame;

    public GameSessionViewModel(GameItemViewModel game, SessionOptions options, PerformanceSimulator performance)
    {
        Game = game;
        Options = options;
        Performance = performance;
        Started = DateTimeOffset.UtcNow;
    }

    public GameItemViewModel Game { get; }

    public SessionOptions Options { get; }

    public PerformanceSimulator Performance { get; }

    public DateTimeOffset Started { get; }

    public bool HandleBack() => true; // B pertence ao jogo.
}

/// <summary>Linha de painel da Central de Controle.</summary>
public sealed partial class PanelRowViewModel : ObservableObject
{
    [ObservableProperty]
    private string _endText = string.Empty;

    [ObservableProperty]
    private bool _isOn;

    [ObservableProperty]
    private double _fraction;

    public required string Label { get; init; }

    public string? IconKey { get; init; }

    public bool IsRadio { get; init; }

    public bool IsToggle { get; init; }

    public bool IsSlider { get; init; }

    public bool IsDanger { get; init; }

    public bool IsBattery { get; init; }

    public required ICommand Command { get; init; }

    public ICommand? AdjustCommand { get; init; }

    /// <summary>Clique ou arraste na barra (fração 0..1).</summary>
    public ICommand? SetFractionCommand { get; init; }
}

public sealed record PanelSection(string Title, string? Description, IReadOnlyList<PanelRowViewModel> Rows);

public sealed record ControlCenterPopover(string Key, IReadOnlyList<PanelSection> Sections);

public sealed record BarItemViewModel(string Key, string Label, string? IconKey, bool IsSeparator = false, bool IsDanger = false, bool IsGame = false);

/// <summary>Cartão vivo da Central de Controle.</summary>
public sealed partial class CardViewModel(string key) : ObservableObject
{
    [ObservableProperty]
    private string _caption = string.Empty;

    [ObservableProperty]
    private string _value = string.Empty;

    [ObservableProperty]
    private string? _pill;

    [ObservableProperty]
    private double? _meter;

    [ObservableProperty]
    private PointCollection? _sparkline;

    [ObservableProperty]
    private ImageSource? _background;

    [ObservableProperty]
    private string? _tagIconKey;

    public string Key { get; } = key;

    public bool IsWide => Key == "game";

    /// <summary>Valor em texto (título) em vez de número grande.</summary>
    public bool IsTextValue => Key is "game" or "capture";
}
