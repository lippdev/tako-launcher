using Launcher.Themes.Xaml;

namespace Launcher.PlayniteCompat.P10;

/// <summary>
/// O que o adaptador P10 acrescenta à lista de permissões do carregador restrito (MVP-016/018/019). Cada item é um tipo
/// ou extensão implementado aqui, no adaptador, sem executar nada do tema; a justificativa de cada liberação está em
/// <c>docs/spikes/MVP-016-xaml-restrito.md</c>. Fica fora de <see cref="P10Theme"/> (sem WPF) para a ferramenta de
/// homologação poder validar temas em qualquer sistema.
/// </summary>
public static class P10Policy
{
    public const string PlayniteMarkupNamespace = "clr-namespace:Playnite.FullscreenApp.Markup;assembly=Playnite.FullscreenApp";
    public const string AdapterMarkupNamespace = "clr-namespace:Launcher.PlayniteCompat.P10.Markup;assembly=Launcher.PlayniteCompat.P10";
    public const string PlayniteBehaviorsNamespace = "clr-namespace:Playnite.Behaviors;assembly=Playnite";
    public const string AdapterBehaviorsNamespace = "clr-namespace:Launcher.PlayniteCompat.P10.Behaviors;assembly=Launcher.PlayniteCompat.P10";
    public const string PlayniteInputNamespace = "clr-namespace:Playnite.Input;assembly=Playnite";
    public const string AdapterInputNamespace = "clr-namespace:Launcher.PlayniteCompat.P10.Input;assembly=Launcher.PlayniteCompat.P10";

    /// <summary>Controles e views do Playnite com equivalente no adaptador.</summary>
    public static readonly string[] Types =
    [
        "ButtonEx", "ItemsControlEx", "ListBoxEx", "ScrollViewerEx", "FadeImage", "GameListItem", "CheckBoxEx", "SliderEx",
        "ToggleButtonEx", "FilterDbItemtSelection", "FilterEnumListSelection", "FilterStringListSelection", "FilterPresetSelector",
        "Filters", "FiltersAdditional", "GameDetails", "GameStatus", "ComboBoxEx",
        // MVP-019: a tela principal do tema (PART_ ligados pelo adaptador).
        "Main",
        // Substituto seguro: mesmo nome do controle do Playnite, mas só mostra texto puro (nenhum HTML é interpretado).
        "HtmlTextView",
    ];

    /// <summary>
    /// Extensões de marcação do Playnite. Todas criam bindings só para objetos do adaptador: nenhuma resolve tipos,
    /// arquivos fora do tema ou serviços do launcher.
    /// </summary>
    public static readonly string[] Extensions =
    [
        "ThemeFile", "Settings",
        // MVP-018
        "ThemeFileBinding", "SettingsBinding", "PluginSettings", "PluginStatus", "Api",
    ];

    /// <summary>Donos de propriedades anexadas inofensivas (não podem ser instanciados como elemento).</summary>
    public static readonly string[] OwnerTypes = ["MediaElementBehaviors"];

    public static XamlSecurityPolicy Create(XamlSecurityPolicy basePolicy)
    {
        ArgumentNullException.ThrowIfNull(basePolicy);
        return basePolicy.Extend(
            presentationTypes: Types,
            presentationExtensions: Extensions,
            remappedNamespaces:
            [
                (PlayniteMarkupNamespace, AdapterMarkupNamespace, ["ThemeFile", "ThemeFileExtension"]),
                // Behaviors e entrada do Playnite: só os nomes listados, nunca o namespace inteiro (spike MVP-016, questão 3).
                (PlayniteBehaviorsNamespace, AdapterBehaviorsNamespace, ["FocusBahaviors"]),
                (PlayniteInputNamespace, AdapterInputNamespace, ["GameControllerInputBinding"]),
            ],
            popupHostTemplates: ["ComboBoxEx"],
            presentationOwnerTypes: OwnerTypes);
    }
}
