using System.IO;
using Launcher.Core.Themes;
using Launcher.PlayniteCompat.P10;
using Launcher.Themes.Xaml;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Temas Fullscreen do Playnite 10 (H2): carregador restrito com a política do adaptador P10, os temas embutidos e os
/// importados pelo usuário (MVP-014/020). "Usar este tema" grava a escolha: com um tema Playnite escolhido, a tela
/// inicial do Tako passa a ser a do tema (MVP-018/019); sem escolha, fica a UI do Tako. A prévia usa o escolhido, ou o
/// Fullscreen padrão do Playnite quando não há.
/// </summary>
public sealed class PlayniteThemeService(XamlSecurityPolicy basePolicy, string? userThemesRoot = null)
{
    public const string SampleTheme = "Playnite-Default";
    private const string SelectionFile = ".selected";

    private readonly RestrictedXamlLoader _loader = new(new XamlValidator(P10Theme.Policy(basePolicy)));

    /// <summary>Temas embutidos (pasta <c>Themes</c> ao lado do executável).</summary>
    public string ThemesRoot { get; } = Path.Combine(AppContext.BaseDirectory, "Themes");

    /// <summary>Temas importados de pacotes <c>.pthm</c> (<c>%LOCALAPPDATA%\Tako\themes</c> ou <c>Tako_Data\themes</c>).</summary>
    public string UserThemesRoot { get; } = userThemesRoot ?? AppPaths.ThemesRoot;

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
}
