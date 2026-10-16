using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações em categorias, como num console (UX-006/009, ADR-0020): coluna de categorias à esquerda e as linhas
/// da categoria à direita. Focar uma categoria mostra o conteúdo dela; → ou A entram no conteúdo; LB/RB trocam de
/// categoria; B no conteúdo volta para a coluna de categorias e B na coluna sai das Configurações. O que veio do
/// Console Mode fica no menu Modo Console (<see cref="ConsoleModeViewModel"/>), não aqui.
/// </summary>
public sealed partial class SettingsViewModel : RowsPageViewModel, IKeyRevealer
{
    [ObservableProperty]
    private SettingsCategoryViewModel _selectedCategory;

    public SettingsViewModel(ShellViewModel shell, SettingsService settings, ThemeService themes)
        : base(shell)
    {
        Settings = settings;
        Themes = themes;
        Categories =
        [
            new("geral", "Geral", "Icon.Gear", "Como o Tako abre, idioma e atualizações.", GeneralRows()),
            new("biblioteca", "Biblioteca e lojas", "Icon.Grid", "De onde vêm os seus jogos e como organizá-los.", LibraryRows()),
            new("emulacao", "Emulação", "Icon.Gamepad", "Emuladores, pastas de ROMs e como abrir e sair dos jogos de console.", EmulationRows()),
            new("aparencia", "Aparência", "Icon.Palette", "Tema, cores e animações.", AppearanceRows()),
            new("controles", "Controles", "Icon.Gamepad", "Controles, teclado e atalhos.", ControlsRows()),
            new("sistema", "Sistema", "Icon.Monitor", "Telas e som depois do jogo, diagnóstico e recuperação.", SystemRows()),
            new("sobre", "Sobre", "Icon.Info", "Versão, licença e créditos.", AboutRows()),
        ];
        foreach (var category in Categories)
        {
            category.Rows.First(r => !r.IsHeader).IsEntry = true;
        }

        _selectedCategory = Categories[0];
        _selectedCategory.IsSelected = true;
        shell.SessionTools.RefreshPermission(); // o usuário pode ter saído e entrado no Windows desde a última vez
    }

    public SettingsService Settings { get; }

    public ThemeService Themes { get; }

    public IReadOnlyList<SettingsCategoryViewModel> Categories { get; }

    public override string Title => "Configurações";

    public override string RouteKey => "settings";

    /// <summary>Abre com o foco na categoria escolhida (a primeira, ou a última visitada ao voltar).</summary>
    public override string? DefaultFocusKey => SelectedCategory.NavKey;

    /// <summary>Dica do rodapé para LB/RB nesta página.</summary>
    public static string CategoryHint => "Categoria";

    protected override IEnumerable<SettingRowViewModel> AllRows => Categories.SelectMany(c => c.Rows);

    public SettingsCategoryViewModel? Category(string id) => Categories.FirstOrDefault(c => c.Id == id);

    public void Select(SettingsCategoryViewModel category)
    {
        ArgumentNullException.ThrowIfNull(category);
        SelectedCategory = category;
    }

    /// <summary>LB/RB: categoria anterior ou seguinte (dá a volta).</summary>
    public SettingsCategoryViewModel CycleCategory(int step)
    {
        int index = Categories.ToList().IndexOf(SelectedCategory);
        Select(Categories[(index + step + Categories.Count) % Categories.Count]);
        return SelectedCategory;
    }

    /// <summary>Primeira linha que recebe foco na categoria escolhida (para LB/RB com o foco no conteúdo).</summary>
    public string? FirstRowKey => SelectedCategory.Rows.FirstOrDefault(r => !r.IsHeader && r.IsShown)?.Key;

    /// <summary>Uma linha de outra categoria foi pedida (ex.: voltar a ela, teste): troca para a categoria dela.</summary>
    public bool RevealKey(string key)
    {
        // A chave vem sozinha ("settings:store:epic") ou com o grupo na frente, como o histórico guarda.
        string? bare = key.IndexOf(':', StringComparison.Ordinal) is int colon and > 0 ? key[(colon + 1)..] : null;
        bool Matches(string own) => own == key || own == bare;
        var target = Categories.FirstOrDefault(c => Matches(c.NavKey) || c.Rows.Any(r => Matches(r.Key)));
        if (target is null || target == SelectedCategory)
        {
            return false;
        }

        Select(target);
        return true;
    }

    partial void OnSelectedCategoryChanged(SettingsCategoryViewModel? oldValue, SettingsCategoryViewModel newValue)
    {
        if (oldValue is not null)
        {
            oldValue.IsSelected = false;
        }

        newValue.IsSelected = true;
        foreach (var row in newValue.Rows)
        {
            row.Refresh();
        }
    }

    [RelayCommand]
    private void OpenCategory(SettingsCategoryViewModel category)
    {
        Select(category);
        Shell.RequestFocus(FirstRowKey);
    }

    private static SettingRowViewModel Row(string key, SettingKind kind, string title, Action? execute = null, Func<string?>? description = null, Func<string?>? value = null, Func<bool>? isOn = null, Func<bool>? visible = null, string? icon = null) =>
        new($"settings:{key}", kind, title, execute is null ? null : new RelayCommand(execute), description, value, isOn, visible, icon: icon);

    private static SettingRowViewModel Header(string key, string title) => new($"settings:header:{key}", SettingKind.Header, title);
}
