using System.Collections.Concurrent;
using System.IO;
using Launcher.Core.Diagnostics;
using Launcher.Core.Themes;
using Launcher.PlayniteCompat.P10;
using Launcher.Themes.Security;
using Launcher.Themes.Xaml;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Temas Fullscreen do Playnite 10 (H2): carregador restrito com a política do adaptador P10, os temas embutidos e os
/// importados pelo usuário (MVP-014/020). "Usar este tema" grava a escolha: com um tema Playnite escolhido, a tela
/// inicial do Tako passa a ser a do tema (MVP-018/019); sem escolha, fica a UI do Tako. A prévia usa o escolhido, ou o
/// Fullscreen padrão do Playnite quando não há.
/// </summary>
public sealed partial class PlayniteThemeService
{
    public const string SampleTheme = "Playnite-Default";
    private const string SelectionFile = ".selected";
    private const int MaxLoggedOrigins = 256;

    private static readonly ConcurrentDictionary<string, bool> LoggedOrigins = new(StringComparer.Ordinal);
    private static ILogger? _blockedLog;

    private readonly RestrictedXamlLoader _loader;

    public PlayniteThemeService(XamlSecurityPolicy basePolicy, string? userThemesRoot = null, ILogger<PlayniteThemeService>? logger = null)
    {
        _loader = new(new XamlValidator(P10Theme.Policy(basePolicy)));
        UserThemesRoot = userThemesRoot ?? AppPaths.ThemesRoot;

        // Frente H: além das pastas dos temas (registradas ao carregar), um tema pode exibir as artes do cache do Tako.
        ThemeSourcePolicy.Shared.AllowDirectory(AppPaths.ArtCache);
        if (logger is not null && Interlocked.CompareExchange(ref _blockedLog, logger, null) is null)
        {
            ThemeMediaGuard.Blocked += OnMediaBlocked;
        }
    }

    /// <summary>Temas embutidos (pasta <c>Themes</c> ao lado do executável).</summary>
    public string ThemesRoot { get; } = Path.Combine(AppContext.BaseDirectory, "Themes");

    /// <summary>Temas importados de pacotes <c>.pthm</c> (<c>%LOCALAPPDATA%\Tako\themes</c> ou <c>Tako_Data\themes</c>).</summary>
    public string UserThemesRoot { get; }

    public P10ThemeReport Load(string themeFolder) => LoadDirectory(Path.Combine(ThemesRoot, themeFolder));

    /// <summary>
    /// Carrega qualquer pasta de tema P10 (embutido ou importado) sobre o Fullscreen padrão do Playnite, como o
    /// Playnite faz: arquivos que o tema não tem (ou que o carregador recusa) vêm do padrão. Roda na thread de UI.
    /// </summary>
    public P10ThemeReport LoadDirectory(string directory) => P10Theme.Load(directory, _loader, Path.Combine(ThemesRoot, SampleTheme));

    /// <summary>Tema escolhido pelo usuário, ou o Fullscreen padrão do Playnite quando não há escolha (ou ela sumiu).</summary>
    public P10ThemeReport LoadSelected() => LoadDirectory(Selected().Directory);

    public InstalledTheme BuiltIn()
    {
        string directory = Path.Combine(ThemesRoot, SampleTheme);
        return new InstalledTheme(directory, ThemeManifest.Read(directory));
    }

    public IReadOnlyList<InstalledTheme> Imported() => InstalledTheme.Scan(UserThemesRoot);

    /// <summary>Instala um pacote <c>.pthm</c>; os Ids dos temas embutidos não podem ser substituídos.</summary>
    public ThemeInstallResult Import(string packagePath) =>
        ThemePackage.Install(packagePath, UserThemesRoot, reservedIds: [BuiltIn().Id]);

    public void Remove(InstalledTheme theme)
    {
        ArgumentNullException.ThrowIfNull(theme);
        if (string.Equals(SelectedId, theme.Id, StringComparison.OrdinalIgnoreCase))
        {
            Select(null);
        }

        ThemePackage.Remove(UserThemesRoot, theme.Directory);
    }

    /// <summary>Id do tema escolhido; nulo = padrão.</summary>
    public string? SelectedId
    {
        get
        {
            try
            {
                string path = Path.Combine(UserThemesRoot, SelectionFile);
                string id = File.Exists(path) ? File.ReadAllText(path).Trim() : string.Empty;
                return id.Length > 0 ? id : null;
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                return null;
            }
        }
    }

    /// <summary>Tema da prévia: o escolhido, ou o Fullscreen padrão do Playnite.</summary>
    public InstalledTheme Selected() => SelectedForUi() ?? BuiltIn();

    /// <summary>
    /// Tema Playnite aplicado à interface; nulo = UI do Tako (sem escolha, ou o tema escolhido sumiu ou ficou
    /// incompatível).
    /// </summary>
    public InstalledTheme? SelectedForUi()
    {
        if (SelectedId is not { } id)
        {
            return null;
        }

        var builtIn = BuiltIn();
        if (string.Equals(id, builtIn.Id, StringComparison.OrdinalIgnoreCase))
        {
            return builtIn;
        }

        return Imported().FirstOrDefault(t => string.Equals(t.Id, id, StringComparison.OrdinalIgnoreCase) && t.Manifest.IsCompatible);
    }

    /// <summary>Escolhe o tema Playnite da interface (e da prévia); nulo volta à UI do Tako.</summary>
    public void Select(InstalledTheme? theme)
    {
        string path = Path.Combine(UserThemesRoot, SelectionFile);
        if (theme is null)
        {
            if (File.Exists(path))
            {
                File.Delete(path);
            }

            return;
        }

        Directory.CreateDirectory(UserThemesRoot);
        File.WriteAllText(path, theme.Id);
    }

    /// <summary>
    /// Origem bloqueada pela guarda de mídia: vai para o log uma vez por origem, só com esquema/host ou nome do arquivo
    /// (o <see cref="Launcher.Core.Diagnostics.LogRedactor"/> do arquivo de log ainda passa por cima). O usuário não vê nada.
    /// </summary>
    private static void OnMediaBlocked(object? sender, ThemeMediaBlockedEventArgs e)
    {
        if (_blockedLog is { } logger && LoggedOrigins.Count < MaxLoggedOrigins && LoggedOrigins.TryAdd(e.Origin, true))
        {
            LogMediaBlocked(logger, e.Reason, e.TargetType.Name, LogRedactor.Default.Redact(e.Origin));
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tema: origem de mídia bloqueada ({Reason}, {Target}): {Origin}")]
    private static partial void LogMediaBlocked(ILogger logger, ThemeSourceBlock reason, string target, string origin);
}
