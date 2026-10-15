using System.Windows;
using Launcher.Core.Library;
using Launcher.Themes.Xaml;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf;

public partial class App : Application
{
    private ServiceProvider? _services;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        _services = BuildServices();

        // O tema padrão passa pelo mesmo carregador restrito que temas da comunidade (MVP-013/016).
        var startupTheme = _services.GetRequiredService<ThemeService>().Apply(ThemeService.DefaultTheme);
        var shell = _services.GetRequiredService<ShellViewModel>();
        var window = _services.GetRequiredService<MainWindow>();
        MainWindow = window;
        window.Show();
        if (!startupTheme.Succeeded)
        {
            shell.ShowRecovery(startupTheme);
        }
    }

    /// <summary>Composição de dependências (também usada pelo teste de fumaça da UI).</summary>
    internal static ServiceProvider BuildServices()
    {
        var services = new ServiceCollection();
        services.AddLogging(logging => logging.AddDebug().SetMinimumLevel(LogLevel.Information));
        services.AddSingleton(TimeProvider.System);
        services.AddSingleton<IGameLibrary>(sp =>
        {
            var time = sp.GetRequiredService<TimeProvider>();
            return new InMemoryGameLibrary(SampleLibrary.Load(time), time);
        });
        services.AddSingleton(XamlSecurityPolicy.CreateDefault());
        services.AddSingleton<XamlValidator>();
        services.AddSingleton<RestrictedXamlLoader>();
        services.AddSingleton<ThemeService>();
        services.AddSingleton<PlayniteThemeService>();
        services.AddSingleton<ArtService>();
        services.AddSingleton<SettingsService>();
        services.AddSingleton<PerformanceSimulator>();
        services.AddSingleton<ShellViewModel>();
        services.AddSingleton<MainWindow>();
        return services.BuildServiceProvider();
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _services?.Dispose();
        base.OnExit(e);
    }
}
