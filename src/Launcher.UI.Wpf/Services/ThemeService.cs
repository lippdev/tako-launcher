using System.IO;
using System.Windows;
using Launcher.Themes.Xaml;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>Resultado de aplicar um tema: sucesso ou o motivo da rejeição (exibido na tela de recuperação).</summary>
public sealed record ThemeLoadResult(string ThemeFolder, bool Succeeded, string? Error)
{
    public static ThemeLoadResult Ok(string folder) => new(folder, true, null);
}

/// <summary>
/// Aplica os tokens do tema (Tokens.xaml) passando pelo carregador XAML restrito (MVP-016). Se o tema falhar,
/// volta para a cópia embutida do tema padrão (MVP-021), e a UI mostra a tela de recuperação.
/// </summary>
public sealed partial class ThemeService(RestrictedXamlLoader loader, ILogger<ThemeService> logger)
{
    public const string DefaultTheme = "Default";
    public const string InvalidSampleTheme = "Exemplo-Invalido";
    private const string TokensFile = "Tokens.xaml";

    private ResourceDictionary? _applied;

    public string ThemesRoot { get; } = Path.Combine(AppContext.BaseDirectory, "Themes");

    public string CurrentTheme { get; private set; } = DefaultTheme;

    public ThemeLoadResult Apply(string themeFolder)
    {
        try
        {
            var tokens = loader.LoadResourceDictionary(Path.Combine(ThemesRoot, themeFolder), TokensFile);
            Swap(tokens);
            CurrentTheme = themeFolder;
            return ThemeLoadResult.Ok(themeFolder);
        }
        catch (Exception ex) when (ex is ThemeXamlRejectedException or IOException or UnauthorizedAccessException)
        {
            string reason = ex is ThemeXamlRejectedException { Violations.Count: > 0 } rejected
                ? $"{themeFolder}/{TokensFile} {rejected.Violations[0]}"
                : ex.Message;
            LogThemeRejected(logger, themeFolder, reason);
            ApplyEmbeddedDefault();
            return new ThemeLoadResult(themeFolder, false, reason);
        }
    }

    /// <summary>Tema padrão embutido no executável: funciona mesmo se a pasta Themes for apagada ou corrompida.</summary>
    public void ApplyEmbeddedDefault()
    {
        var uri = new Uri($"pack://application:,,,/Tako;component/Themes/{DefaultTheme}/{TokensFile}");
        var info = Application.GetResourceStream(uri) ?? throw new InvalidOperationException("Tema padrão embutido ausente.");
        using var reader = new StreamReader(info.Stream);
        Swap(loader.Parse(reader.ReadToEnd(), uri.AbsoluteUri, TokensFile));
        CurrentTheme = DefaultTheme;
    }

    private void Swap(ResourceDictionary tokens)
    {
        var merged = Application.Current.Resources.MergedDictionaries;
        if (_applied is not null)
        {
            merged.Remove(_applied);
        }

        // Tokens primeiro: estilos do app os referenciam via DynamicResource.
        merged.Insert(0, tokens);
        _applied = tokens;
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tema '{Theme}' rejeitado: {Reason}")]
    private static partial void LogThemeRejected(ILogger logger, string theme, string reason);
}
