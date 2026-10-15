using System.IO;
using System.Windows;
using System.Windows.Media;
using Launcher.Core.Themes;
using Launcher.PlayniteCompat.P10.Markup;
using Launcher.Themes.Xaml;

namespace Launcher.PlayniteCompat.P10;

/// <summary>Resultado de carregar um tema Fullscreen do Playnite 10: o que entrou, o que foi recusado e por quê.</summary>
public sealed record P10ThemeReport(
    string Name,
    string? Version,
    string? ThemeApiVersion,
    ResourceDictionary Resources,
    IReadOnlyList<string> Loaded,
    IReadOnlyList<(string File, string Reason)> Rejected,
    ThemeManifestResult Manifest)
{
    /// <summary>Arquivos XAML do próprio tema que o adaptador tentou carregar.</summary>
    public int Total => Loaded.Count + Rejected.Count;

    /// <summary>Arquivos que vieram do tema base (Fullscreen padrão) porque o tema não os tem ou foram recusados.</summary>
    public IReadOnlyList<string> Inherited { get; init; } = [];

    /// <summary>XAML do tema que o Playnite também ignora (não existe no tema padrão, ex.: App.xaml de projeto Blend).</summary>
    public IReadOnlyList<string> Ignored { get; init; } = [];
}

/// <summary>
/// Adaptador de temas Fullscreen do Playnite 10 (H2, MVP-014/015/017/018). Amplia a política do carregador restrito
/// com os tipos que o adaptador implementa e compõe os dicionários do tema por código, na ordem do Playnite
/// (cada arquivo validado isoladamente; nada carrega outro arquivo por conta própria).
/// </summary>
public static class P10Theme
{
    private const string PlayniteMarkupNamespace = "clr-namespace:Playnite.FullscreenApp.Markup;assembly=Playnite.FullscreenApp";
    private const string AdapterMarkupNamespace = "clr-namespace:Launcher.PlayniteCompat.P10.Markup;assembly=Launcher.PlayniteCompat.P10";

    /// <summary>Ordem de composição usada pelo Playnite: constantes, mídia, controles padrão, estilos derivados, controles e views.</summary>
    private static readonly string[] Folders = [string.Empty, "DefaultControls", "DerivedStyles", "CustomControls", "Views"];

    /// <summary>Política do carregador para temas P10.</summary>
    public static XamlSecurityPolicy Policy(XamlSecurityPolicy basePolicy)
    {
        ArgumentNullException.ThrowIfNull(basePolicy);
        return basePolicy.Extend(
            presentationTypes:
            [
                "ButtonEx", "ItemsControlEx", "ListBoxEx", "ScrollViewerEx", "FadeImage", "GameListItem", "CheckBoxEx", "SliderEx",
                "ToggleButtonEx", "FilterDbItemtSelection", "FilterEnumListSelection", "FilterStringListSelection", "FilterPresetSelector",
                "Filters", "FiltersAdditional", "GameDetails", "GameStatus", "ComboBoxEx",
            ],
            presentationExtensions: ["ThemeFile", "Settings"],
            remappedNamespaces: [(PlayniteMarkupNamespace, AdapterMarkupNamespace, ["ThemeFile", "ThemeFileExtension"])],
            popupHostTemplates: ["ComboBoxEx"]);
    }

    /// <summary>
    /// Carrega o tema. Deve rodar na thread de UI (STA). Com <paramref name="baseThemeDirectory"/> (o Fullscreen padrão),
    /// segue a regra do Playnite (<c>Themes.ApplyTheme</c>): só valem os arquivos que existem no tema base, e cada um
    /// entra na ordem base → tema. Assim temas parciais herdam o que não alteram (MVP-017), e um arquivo recusado do
    /// tema cai para a versão do tema base.
    /// </summary>
    public static P10ThemeReport Load(string themeDirectory, RestrictedXamlLoader loader, string? baseThemeDirectory = null)
    {
        ArgumentNullException.ThrowIfNull(loader);
        string root = Path.GetFullPath(themeDirectory);
        string? baseRoot = baseThemeDirectory is null ? null : Path.GetFullPath(baseThemeDirectory);
        if (baseRoot is not null && (string.Equals(baseRoot.TrimEnd(Path.DirectorySeparatorChar), root.TrimEnd(Path.DirectorySeparatorChar), StringComparison.OrdinalIgnoreCase) || !Directory.Exists(baseRoot)))
        {
            baseRoot = null;
        }

        var manifest = ThemeManifest.Read(root);
        var resources = new ResourceDictionary();
        resources.MergedDictionaries.Add(CompatibilityResources());
        var loaded = new List<string>();
        var rejected = new List<(string, string)>();
        var inherited = new List<string>();
        var files = ThemeFiles(baseRoot ?? root);
        List<string> ignored = baseRoot is null ? [] : ThemeFiles(root).Except(files, StringComparer.OrdinalIgnoreCase).ToList();

        // Como no Playnite, cada arquivo enxerga os recursos dos anteriores (StaticResource entre arquivos).
        // O dicionário em composição fica visível no app só durante a carga.
        var appResources = Application.Current?.Resources;
        appResources?.MergedDictionaries.Add(resources);
        try
        {
            foreach (string relative in files)
            {
                bool baseLoaded = baseRoot is not null && TryLoad(loader, baseRoot, relative, resources, out _);
                if (!File.Exists(Path.Combine(root, relative)))
                {
                    if (baseLoaded)
                    {
                        inherited.Add(relative);
                    }

                    continue;
                }

                if (TryLoad(loader, root, relative, resources, out string? reason))
                {
                    loaded.Add(relative);
                }
                else
                {
                    rejected.Add((relative, reason!));
                    if (baseLoaded)
                    {
                        inherited.Add(relative);
                    }
                }
            }
        }
        finally
        {
            appResources?.MergedDictionaries.Remove(resources);
        }

        return new P10ThemeReport(
            manifest.Manifest.Name ?? Path.GetFileName(root),
            manifest.Manifest.Version,
            manifest.Manifest.ThemeApiVersionText,
            resources,
            loaded,
            rejected,
            manifest)
        {
            Inherited = inherited,
            Ignored = ignored,
        };
    }

    /// <summary>XAML das pastas do tema, na ordem de composição do Playnite.</summary>
    private static List<string> ThemeFiles(string root)
    {
        var files = new List<string>();
        foreach (string folder in Folders)
        {
            string directory = Path.Combine(root, folder);
            if (Directory.Exists(directory))
            {
                files.AddRange(Directory.EnumerateFiles(directory, "*.xaml")
                    .Order(StringComparer.OrdinalIgnoreCase)
                    .OrderBy(Priority)
                    .Select(f => Path.GetRelativePath(root, f).Replace('\\', '/')));
            }
        }

        return files;
    }

    private static bool TryLoad(RestrictedXamlLoader loader, string root, string relative, ResourceDictionary target, out string? reason)
    {
        try
        {
            target.MergedDictionaries.Add(loader.LoadResourceDictionary(root, relative));
            reason = null;
            return true;
        }
        catch (ThemeXamlRejectedException ex)
        {
            reason = ex.Violations.Count > 0 ? ex.Violations[0].ToString() : ex.InnerException?.Message ?? ex.Message;
            return false;
        }
    }

    /// <summary>Constants.xaml antes de Media.xaml (Media usa recursos de Constants).</summary>
    private static int Priority(string file) => Path.GetFileName(file) switch
    {
        "Constants.xaml" => 0,
        "Media.xaml" => 1,
        _ => 2,
    };

    /// <summary>Recursos que o próprio Playnite fornece aos temas (fontes, booleanos e conversores); aqui com equivalentes do launcher.</summary>
    private static ResourceDictionary CompatibilityResources()
    {
        var resources = PlayniteConverters.Resources();
        resources["FontTitilliumWebLight"] = new FontFamily("Segoe UI Light");
        resources["FontTitilliumWeb"] = new FontFamily("Segoe UI");
        resources["FontIcoFont"] = new FontFamily("Segoe MDL2 Assets");
        return resources;
    }
}
