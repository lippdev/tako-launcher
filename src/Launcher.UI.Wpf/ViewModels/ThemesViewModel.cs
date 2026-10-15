using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Themes;
using Launcher.PlayniteCompat.P10;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

public enum ThemeKind
{
    /// <summary>Tema do próprio Tako (tokens do DESIGN.md).</summary>
    Tako,

    /// <summary>Tema Playnite 10 que vem com o Tako (Fullscreen padrão do Playnite).</summary>
    BuiltIn,

    /// <summary>Importado de um pacote <c>.pthm</c>.</summary>
    Imported,
}

/// <summary>Um tema na lista de Configurações › Temas.</summary>
public sealed partial class ThemeItemViewModel : ObservableObject
{
    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Detail))]
    private string? _loadSummary;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Badge))]
    private bool _isSelected;

    public ThemeItemViewModel(ThemeKind kind, string name, InstalledTheme? theme)
    {
        Kind = kind;
        Name = name;
        Theme = theme;
    }

    public ThemeKind Kind { get; }

    public string Name { get; }

    /// <summary>Pasta e manifesto (nulo para o tema do Tako, que não é um tema Playnite).</summary>
    public InstalledTheme? Theme { get; }

    public string Key => Kind == ThemeKind.Tako ? "theme:tako" : $"theme:{Theme!.Id}";

    public bool IsPlaynite => Theme is not null;

    public bool CanRemove => Kind == ThemeKind.Imported;

    public bool IsCompatible => Theme?.Manifest.IsCompatible ?? true;

    public string Badge => (IsSelected, IsCompatible) switch
    {
        (_, false) => "Incompatível",
        (true, _) => "Em uso",
        _ => "Opções",
    };

    /// <summary>Linha de baixo: origem, autor, versão, API e o último resultado de carga (X de Y arquivos).</summary>
    public string Detail
    {
        get
        {
            if (Theme is null)
            {
                return "Cores, tipografia e formas do Tako, carregadas pelo carregador XAML restrito";
            }

            var m = Theme.Manifest.Manifest;
            string origin = Kind == ThemeKind.BuiltIn ? "Embutido" : "Importado";
            var parts = new List<string> { origin };
            if (m.Author is not null)
            {
                parts.Add(m.Author);
            }

            if (m.Version is not null)
            {
                parts.Add($"versão {m.Version}");
            }

            parts.Add($"ThemeApiVersion {m.ThemeApiVersionText ?? "?"}");
            if (Theme.Manifest.Errors.FirstOrDefault() is { } error)
            {
                parts.Add(error.Message);
            }
            else if (LoadSummary is not null)
            {
                parts.Add(LoadSummary);
            }

            return string.Join(" · ", parts);
        }
    }

    public static string Summarize(P10ThemeReport report) =>
        $"{report.Loaded.Count} de {report.Total} arquivos carregados" + (report.Inherited.Count > 0 ? $", {report.Inherited.Count} herdados do padrão" : string.Empty);
}

/// <summary>Configurações › Temas (MVP-020): tema do Tako, tema Playnite embutido e temas importados de <c>.pthm</c>.</summary>
public sealed partial class ThemesViewModel : PageViewModel
{
    private readonly PlayniteThemeService _playnite;
    private readonly IReadOnlyDictionary<string, string> _loadSummaries;

    /// <param name="loadSummaries">Último resultado de carga por pasta de tema (sobrevive ao recarregar a lista).</param>
    public ThemesViewModel(ShellViewModel shell, PlayniteThemeService playnite, IReadOnlyDictionary<string, string> loadSummaries)
        : base(shell)
    {
        _playnite = playnite;
        _loadSummaries = loadSummaries;
        Refresh();
    }

    public override string Title => "Temas";

    public override string RouteKey => "settings";

    public override string? DefaultFocusKey => "themes:import";

    public ObservableCollection<ThemeItemViewModel> Themes { get; } = [];

    public string FolderText => $"Temas importados ficam em {_playnite.UserThemesRoot}. Temas Fullscreen do Playnite com ThemeApiVersion {ThemeManifestValidator.SupportedRange}.";

    public override void Refresh()
    {
        // Em uso: o tema Playnite aplicado à interface, ou o do Tako quando não há um.
        string? selected = _playnite.SelectedForUi()?.Directory;
        Themes.Clear();
        Themes.Add(new ThemeItemViewModel(ThemeKind.Tako, "Tako (padrão)", null) { IsSelected = selected is null });
        foreach (var (kind, theme) in _playnite.Imported().Select(t => (ThemeKind.Imported, t)).Prepend((ThemeKind.BuiltIn, _playnite.BuiltIn())))
        {
            Themes.Add(new ThemeItemViewModel(kind, theme.Manifest.Manifest.DisplayName, theme)
            {
                IsSelected = string.Equals(theme.Directory, selected, StringComparison.OrdinalIgnoreCase),
                LoadSummary = _loadSummaries.GetValueOrDefault(theme.Directory),
            });
        }
    }

    [RelayCommand]
    private void Import() => Shell.ImportTheme();

    [RelayCommand]
    private void RestoreDefault() => Shell.RestoreDefaultTheme();

    [RelayCommand]
    private void Open(ThemeItemViewModel item) => Shell.ShowThemeOptions(item);
}
