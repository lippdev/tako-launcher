using System.IO;
using System.Windows;
using System.Windows.Media;
using Launcher.Themes.Xaml;

namespace Launcher.PlayniteCompat.P10;

/// <summary>Resultado de carregar um tema Fullscreen do Playnite 10: o que entrou, o que foi recusado e por quê.</summary>
public sealed record P10ThemeReport(
    string Name,
    string? Version,
    string? ThemeApiVersion,
    ResourceDictionary Resources,
    IReadOnlyList<string> Loaded,
    IReadOnlyList<(string File, string Reason)> Rejected);

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
            presentationTypes: ["ButtonEx", "ItemsControlEx", "ListBoxEx", "ScrollViewerEx", "FadeImage", "GameListItem"],
            presentationExtensions: ["ThemeFile", "Settings"],
            remappedNamespaces: [(PlayniteMarkupNamespace, AdapterMarkupNamespace, ["ThemeFile", "ThemeFileExtension"])]);
    }

    /// <summary>Carrega o tema. Deve rodar na thread de UI (STA).</summary>
    public static P10ThemeReport Load(string themeDirectory, RestrictedXamlLoader loader)
    {
        ArgumentNullException.ThrowIfNull(loader);
        string root = Path.GetFullPath(themeDirectory);
        var manifest = ReadManifest(Path.Combine(root, "theme.yaml"));
        var resources = new ResourceDictionary();
        resources.MergedDictionaries.Add(CompatibilityResources());
        var loaded = new List<string>();
        var rejected = new List<(string, string)>();

        foreach (string folder in Folders)
        {
            string directory = Path.Combine(root, folder);
            if (!Directory.Exists(directory))
            {
                continue;
            }

            foreach (string file in Directory.EnumerateFiles(directory, "*.xaml").Order(StringComparer.OrdinalIgnoreCase).OrderBy(Priority))
            {
                string relative = Path.GetRelativePath(root, file).Replace('\\', '/');
                try
                {
                    resources.MergedDictionaries.Add(loader.LoadResourceDictionary(root, relative));
                    loaded.Add(relative);
                }
                catch (ThemeXamlRejectedException ex)
                {
                    string reason = ex.Violations.Count > 0 ? ex.Violations[0].ToString() : ex.InnerException?.Message ?? ex.Message;
                    rejected.Add((relative, reason));
                }
            }
        }

        return new P10ThemeReport(
            manifest.GetValueOrDefault("Name") ?? Path.GetFileName(root),
            manifest.GetValueOrDefault("Version"),
            manifest.GetValueOrDefault("ThemeApiVersion"),
            resources,
            loaded,
            rejected);
    }

    /// <summary>Constants.xaml antes de Media.xaml (Media usa recursos de Constants).</summary>
    private static int Priority(string file) => Path.GetFileName(file) switch
    {
        "Constants.xaml" => 0,
        "Media.xaml" => 1,
        _ => 2,
    };

    /// <summary>Recursos que o próprio Playnite fornece aos temas (fontes); aqui com equivalentes do sistema.</summary>
    private static ResourceDictionary CompatibilityResources() => new()
    {
        ["FontTitilliumWebLight"] = new FontFamily("Segoe UI Light"),
        ["FontTitilliumWeb"] = new FontFamily("Segoe UI"),
        ["FontIcoFont"] = new FontFamily("Segoe MDL2 Assets"),
    };

    /// <summary>theme.yaml do Playnite é "Chave: valor" em linhas; não precisa de parser YAML completo.</summary>
    private static Dictionary<string, string> ReadManifest(string path)
    {
        var values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (!File.Exists(path))
        {
            return values;
        }

        foreach (string line in File.ReadLines(path))
        {
            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon > 0 && !line.TrimStart().StartsWith('#'))
            {
                values[line[..colon].Trim().TrimStart('﻿')] = line[(colon + 1)..].Trim();
            }
        }

        return values;
    }
}
