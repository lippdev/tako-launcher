using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using Launcher.Core.Themes;
using Launcher.PlayniteCompat.P10.Controls;
using Launcher.PlayniteCompat.P10.Data;
using Launcher.PlayniteCompat.P10.Markup;
using Launcher.Themes.Security;
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

    /// <summary>A tela principal (<c>Views/Main.xaml</c>) veio do próprio tema.</summary>
    public bool MainFromTheme => Loaded.Contains(P10Theme.MainView, StringComparer.OrdinalIgnoreCase);

    /// <summary>Há uma tela principal para aplicar à interface inteira: do tema ou herdada do tema padrão (MVP-018/019).</summary>
    public bool HasMainView => MainFromTheme || Inherited.Contains(P10Theme.MainView, StringComparer.OrdinalIgnoreCase);
}

/// <summary>
/// Adaptador de temas Fullscreen do Playnite 10 (H2, MVP-014/015/017/018). Amplia a política do carregador restrito
/// com os tipos que o adaptador implementa e compõe os dicionários do tema por código, na ordem do Playnite
/// (cada arquivo validado isoladamente; nada carrega outro arquivo por conta própria).
/// </summary>
public static class P10Theme
{
    /// <summary>Arquivo da tela principal do tema.</summary>
    public const string MainView = "Views/Main.xaml";

    /// <summary>Ordem de composição usada pelo Playnite: constantes, mídia, controles padrão, estilos derivados, controles e views.</summary>
    private static readonly string[] Folders = [string.Empty, "DefaultControls", "DerivedStyles", "CustomControls", "Views"];

    /// <summary>Política do carregador para temas P10 (ver <see cref="P10Policy"/>).</summary>
    public static XamlSecurityPolicy Policy(XamlSecurityPolicy basePolicy) => P10Policy.Create(basePolicy);

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

        // Frente H (questão aberta 8): antes de qualquer XAML do tema, a guarda de mídia entra no binding desta thread e
        // as pastas do tema e do tema padrão passam a ser as únicas de onde um caminho montado por dados pode carregar.
        // Sem a guarda (WPF diferente do esperado), nada do tema é carregado: falha fechada.
        if (!ThemeMediaGuard.EnsureInstalled(out string? guardProblem))
        {
            return new P10ThemeReport(
                manifest.Manifest.Name ?? Path.GetFileName(root),
                manifest.Manifest.Version,
                manifest.Manifest.ThemeApiVersionText,
                new ResourceDictionary(),
                [],
                [.. ThemeFiles(root).Select(f => (f, guardProblem))],
                manifest);
        }

        ThemeSourcePolicy.Shared.AllowTheme(root);
        if (baseRoot is not null)
        {
            ThemeSourcePolicy.Shared.AllowTheme(baseRoot, isBase: true);
            Markup.ThemeFiles.FallbackRoot = Path.TrimEndingDirectorySeparator(baseRoot);
        }

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

    /// <summary>
    /// Ensaio da tela principal antes de aplicá-la à interface inteira (MVP-021): monta <see cref="Main"/> e
    /// <see cref="GameDetails"/> fora da tela, com o modelo real, e faz medida e arranjo. Erros que o WPF só mostra ao
    /// instanciar o template (propriedade inexistente, conversão inválida, binding TwoWay em valor só-leitura) aparecem
    /// aqui, e o launcher fica na UI padrão. Retorna o motivo da falha, ou nulo. Roda na thread de UI.
    /// </summary>
    public static string? Probe(P10ThemeReport report, P10MainModel model, P10Api api, Size size)
    {
        ArgumentNullException.ThrowIfNull(report);
        if (!report.HasMainView)
        {
            return $"{MainView} do tema e do tema padrão foram recusados pelo carregador XAML restrito.";
        }

        var main = new Main { DataContext = model };
        var details = new GameDetails { DataContext = model.Games.FirstOrDefault(), Model = model };
        var root = new Grid();
        root.Children.Add(main);
        root.Children.Add(details);
        root.Resources.MergedDictionaries.Add(report.Resources);
        P10Context.SetApi(root, api);
        string? problem;
        try
        {
            root.Measure(size);
            root.Arrange(new Rect(size));
            root.UpdateLayout();
            problem = main.Template is null ? $"O tema não define o template de Main ({MainView})." : null;
        }
        catch (Exception ex) when (ex is not (OutOfMemoryException or StackOverflowException))
        {
            problem = Describe(ex);
        }

        // Desmontar também é ensaio: a lista do tema é recriada ao filtrar ou quando a biblioteca muda. E o ensaio não
        // pode continuar ligado ao modelo (a lista dele escreveria a seleção).
        try
        {
            main.DetachSelection();
            main.DataContext = null;
            details.DataContext = null;
            details.Model = null;
            root.Children.Clear();
        }
        catch (Exception ex) when (ex is not (OutOfMemoryException or StackOverflowException))
        {
            problem ??= Describe(ex);
        }

        return problem;
    }

    /// <summary>Erro de tema para a tela de recuperação e o log: tipo, mensagem e onde aconteceu.</summary>
    public static string Describe(Exception error)
    {
        ArgumentNullException.ThrowIfNull(error);
        var root = error;
        while (root.InnerException is { } inner)
        {
            root = inner;
        }

        string where = root.StackTrace?.Split('\n', StringSplitOptions.RemoveEmptyEntries).FirstOrDefault()?.Trim() ?? string.Empty;
        string text = ReferenceEquals(root, error) ? $"{error.GetType().Name}: {error.Message}" : $"{error.GetType().Name}: {error.Message} ({root.GetType().Name}: {root.Message})";
        return where.Length > 0 ? $"{text} [{where}]" : text;
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
            reason = ex.Violations.Count > 0 ? ex.Violations[0].ToString() : Innermost(ex);
            return false;
        }
    }

    /// <summary>Mensagem mais interna (ex.: a chave de um StaticResource que não existe), sem a pilha.</summary>
    private static string Innermost(Exception error)
    {
        var messages = new List<string>();
        for (var e = error.InnerException ?? error; e is not null; e = e.InnerException)
        {
            messages.Add(e.Message);
        }

        return string.Join(" → ", messages.Distinct());
    }

    /// <summary>Constants.xaml antes de Media.xaml (Media usa recursos de Constants).</summary>
    private static int Priority(string file) => Path.GetFileName(file) switch
    {
        "Constants.xaml" => 0,
        "Media.xaml" => 1,
        _ => 2,
    };

    /// <summary>
    /// Recursos que o próprio Playnite fornece aos temas (fontes, booleanos, conversores e textos <c>LOC*</c>); aqui com
    /// equivalentes do launcher, em português. O tema pode redefinir qualquer um.
    /// </summary>
    private static ResourceDictionary CompatibilityResources()
    {
        var resources = PlayniteConverters.Resources();
        resources["FontTitilliumWebLight"] = new FontFamily("Segoe UI Light");
        resources["FontTitilliumWeb"] = new FontFamily("Segoe UI");
        resources["FontTitilliumWebBold"] = new FontFamily("Segoe UI Semibold");
        resources["FontIcoFont"] = new FontFamily("Segoe MDL2 Assets");
        resources["DesignWidth"] = 1920.0;
        resources["DesignHeight"] = 1080.0;
        foreach (var (key, text) in Strings)
        {
            resources[key] = text;
        }

        return resources;
    }

    /// <summary>Textos da interface do Playnite usados pelas telas principais dos temas homologados.</summary>
    private static readonly (string Key, string Text)[] Strings =
    [
        ("LOCPlayGame", "Jogar"),
        ("LOCInstallGame", "Instalar"),
        ("LOCGameDetails", "Detalhes"),
        ("LOCGameOptions", "Opções"),
        ("LOCSearchLabel", "Buscar"),
        ("LOCFilter", "Filtrar"),
        ("LOCNoGamesFound", "Nenhum jogo encontrado"),
        ("LOCGameListSearchResults", "Resultados da busca"),
        ("LOCGameListExtraFiltered", "Filtro adicional ativo"),
        ("LOCGameListFiltered", "Lista filtrada"),
        ("LOCTimePlayed", "Tempo jogado"),
        ("LOCGameLastActivityTitle", "Última vez"),
        ("LOCGameReleaseDateTitle", "Lançamento"),
        ("LOCGamePlatformTitle", "Plataforma"),
        ("LOCGameProviderTitle", "Biblioteca"),
        ("LOCGameDescriptionTitle", "Descrição"),
        ("LOCGenreLabel", "Gênero"),
        ("LOCPublisherLabel", "Distribuidora"),
        ("LOCCompletionStatus", "Status"),
        ("LOCAllGames", "Todos os jogos"),
        ("LOCQuickFilterRecentlyPlayed", "Jogados recentemente"),
        ("LOCNotifications", "Notificações"),
        ("LOCSettingsLabel", "Configurações"),
        ("LOCMoreAction", "Mais"),
        ("LOCCurrentlyPlaying", "Jogando agora"),
        ("LOCGridViewLabel", "Grade"),
        ("LOCListViewLabel", "Lista"),
        ("LOCUpdaterWindowTitle", "Atualização"),
    ];
}
