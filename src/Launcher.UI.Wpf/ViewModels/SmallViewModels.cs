using System.Windows.Input;
using System.Windows.Media;
using CommunityToolkit.Mvvm.ComponentModel;
using Launcher.Core.Input;
using Launcher.Core.Library;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Item da barra lateral.</summary>
public sealed partial class SidebarItemViewModel(string key, string label, string iconKey, ICommand command) : ObservableObject
{
    [ObservableProperty]
    private bool _isSelected;

    [ObservableProperty]
    private int? _count;

    public string Key { get; } = key;

    public string Label { get; } = label;

    public string IconKey { get; } = iconKey;

    /// <summary>Lojas usam o logo num selo colorido; demais itens usam ícone de traço.</summary>
    public Brush? BadgeBrush { get; init; }

    public bool IsSection { get; init; }

    public string? SectionTitle { get; init; }

    public bool IsBottom { get; init; }

    public LibraryFilter? Filter { get; init; }

    public ICommand Command { get; } = command;
}

/// <summary>Dica da barra inferior: glifos dos botões e a ação. Clicável.</summary>
public sealed record HintViewModel(IReadOnlyList<string> Glyphs, string Label, bool IsKey, AppCommand Command);

/// <summary>Estado do fundo (cor do jogo em foco e arte).</summary>
public sealed record BackdropState(string GameId, Color Accent, Color Deep, ImageSource? Hero, System.Windows.Point HeroAlignment);
