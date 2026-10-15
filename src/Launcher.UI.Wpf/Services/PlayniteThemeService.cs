using System.IO;
using Launcher.Core.Themes;
using Launcher.PlayniteCompat.P10;
using Launcher.Themes.Xaml;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Temas Fullscreen do Playnite 10 (H2): carregador restrito com a política do adaptador P10, os temas embutidos e os
/// importados pelo usuário (MVP-014/020). No Marco 1 um tema P10 é aplicado numa prévia isolada; trocar o visual
/// inteiro do launcher por um tema P10 é o MVP-018/019. "Usar tema" escolhe qual tema a prévia (e, depois, o modo
/// Fullscreen) usa.
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

    public InstalledTheme Selected() =>
        SelectedId is { } id && Imported().FirstOrDefault(t => string.Equals(t.Id, id, StringComparison.OrdinalIgnoreCase) && t.Manifest.IsCompatible) is { } theme
            ? theme
            : BuiltIn();

    /// <summary>Escolhe o tema usado pela prévia; nulo volta ao padrão.</summary>
    public void Select(InstalledTheme? theme)
    {
        string path = Path.Combine(UserThemesRoot, SelectionFile);
        if (theme is null || string.Equals(theme.Id, BuiltIn().Id, StringComparison.OrdinalIgnoreCase))
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
