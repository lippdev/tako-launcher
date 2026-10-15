using System.IO;
using Launcher.PlayniteCompat.P10;
using Launcher.Themes.Xaml;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Temas Fullscreen do Playnite 10 (H2): carregador restrito com a política do adaptador P10. No Marco 0 o tema é
/// aplicado numa prévia isolada; trocar o visual inteiro do launcher por um tema P10 é o MVP-018/019.
/// </summary>
public sealed class PlayniteThemeService(XamlSecurityPolicy basePolicy)
{
    public const string SampleTheme = "Playnite-Default";

    private readonly RestrictedXamlLoader _loader = new(new XamlValidator(P10Theme.Policy(basePolicy)));

    public string ThemesRoot { get; } = Path.Combine(AppContext.BaseDirectory, "Themes");

    public P10ThemeReport Load(string themeFolder) => P10Theme.Load(Path.Combine(ThemesRoot, themeFolder), _loader);
}
