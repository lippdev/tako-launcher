using System.IO;
using System.Net.Http;
using System.Windows;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Steam;
using Launcher.Themes.Xaml;
using Launcher.UI.Wpf.Input;
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

        // Descrição, gêneros, notas e artes dos jogos chegam em segundo plano (cache primeiro).
        shell.StartMetadataUpdates();

        // Procura e baixa atualizações em segundo plano (GitHub Releases, como no Console Mode).
        shell.StartAutoUpdate(_services.GetRequiredService<UpdateService>());
    }

    /// <summary>Composição de dependências. O teste de fumaça da UI passa uma biblioteca fictícia em <paramref name="library"/>.</summary>
    internal static ServiceProvider BuildServices(IGameLibrary? library = null)
    {
        var services = new ServiceCollection();
        services.AddLogging(logging => logging.AddDebug().SetMinimumLevel(LogLevel.Information));
        services.AddSingleton(TimeProvider.System);
        services.AddSingleton(_ =>
        {
            var http = new HttpClient { Timeout = TimeSpan.FromSeconds(20) };
            // CDNs e APIs exigem User-Agent identificável.
            http.DefaultRequestHeaders.UserAgent.ParseAdd("Tako/0.1 (+https://github.com/lippdev/ultralauncher)");
            return http;
        });
        services.AddSingleton<SteamArtResolver>();
        services.AddSingleton<IMetadataProvider, SteamStoreProvider>();
        services.AddSingleton<MetadataService>();
        services.AddSingleton<UpdateService>();
        services.AddSingleton<IGameLibrary>(sp => library ?? CreateLocalLibrary(sp));
        services.AddSingleton(XamlSecurityPolicy.CreateDefault());
        services.AddSingleton<XamlValidator>();
        services.AddSingleton<RestrictedXamlLoader>();
        services.AddSingleton<ThemeService>();
        services.AddSingleton<PlayniteThemeService>();
        services.AddSingleton<ArtService>();
        services.AddSingleton<SonyHidGamepad>();
        services.AddSingleton<SettingsService>();
        services.AddSingleton<PerformanceSimulator>();
        services.AddSingleton<ShellViewModel>();
        services.AddSingleton<MainWindow>();
        return services.BuildServiceProvider();
    }

    /// <summary>Biblioteca real: jogos instalados na Steam + adicionados pelo usuário (<c>%LOCALAPPDATA%\Tako\library.json</c>).</summary>
    private static LocalGameLibrary CreateLocalLibrary(IServiceProvider services)
    {
        string? steamRoot = SteamLocator.FindRoot();
        services.GetRequiredService<SteamArtResolver>().SteamRoot = steamRoot;
        var logger = services.GetRequiredService<ILogger<App>>();
        IReadOnlyList<Game> DiscoverSteam()
        {
            if (steamRoot is null)
            {
                return [];
            }

            try
            {
                return [.. SteamLibraryScanner.Scan(steamRoot).Select(SteamGames.ToGame)];
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                LogSteamScanFailed(logger, ex, steamRoot);
                return [];
            }
        }

        var library = new LocalGameLibrary(DiscoverSteam, new LibraryStore(AppPaths.LibraryFile), services.GetRequiredService<TimeProvider>(), services.GetRequiredService<MetadataService>().ApplyCached);
        library.Load();
        return library;
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Falha ao ler as bibliotecas da Steam em {SteamRoot}")]
    private static partial void LogSteamScanFailed(ILogger logger, Exception exception, string steamRoot);

    protected override void OnExit(ExitEventArgs e)
    {
        // Atualização baixada e não aplicada: instala agora, sem reabrir.
        _services?.GetService<ShellViewModel>()?.ApplyPendingUpdateOnExit();
        _services?.Dispose();
        base.OnExit(e);
    }
}
