using System.IO;
using System.Net.Http;
using System.Windows;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Steam;
using ConsoleMode.Runtime.Tv;
using ConsoleMode.Runtime.Windows;
using ConsoleMode.Runtime.Windows.SessionTools;
using Launcher.Core.Diagnostics;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Steam;
using Launcher.Core.Library.Xbox;
using Launcher.Core.Persistence;
using Launcher.Core.Resilience;
using Launcher.Core.Session.GameScreen;
using Launcher.Core.Session.Devices;
using Launcher.Core.Session.Restore;
using Launcher.Themes.Xaml;
using Launcher.UI.Wpf.Input;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf;

public partial class App : Application
{
    /// <summary>Força o modo seguro nesta abertura (atalho para suporte: <c>Tako.exe --safe-mode</c>).</summary>
    public const string SafeModeArgument = "--safe-mode";

    private ServiceProvider? _services;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        _services = BuildServices();
        LogUnhandledExceptions(_services);
        // Aberta pelo supervisor (--ui --supervisor-pipe): heartbeats pelo IPC (MVP-003/006, ADR-0012).
        _services.GetRequiredService<SupervisorLink>().Start(e.Args, this);

        // Proteção contra loop de travamento (MVP-036): duas aberturas seguidas sem fechar limpo abrem em modo seguro.
        var journal = _services.GetRequiredService<SessionJournal>();
        var startup = journal.Begin(safeModeRequested: e.Args.Contains(SafeModeArgument, StringComparer.OrdinalIgnoreCase));

        // O tema padrão passa pelo mesmo carregador restrito que temas da comunidade (MVP-013/016); no modo seguro, só a
        // cópia embutida no executável.
        var themes = _services.GetRequiredService<ThemeService>();
        ThemeLoadResult startupTheme;
        if (startup.SafeMode)
        {
            themes.ApplyEmbeddedDefault();
            startupTheme = ThemeLoadResult.Ok(ThemeService.DefaultTheme);
        }
        else
        {
            startupTheme = themes.Apply(ThemeService.DefaultTheme);
        }

        var shell = _services.GetRequiredService<ShellViewModel>();
        var window = _services.GetRequiredService<MainWindow>();
        MainWindow = window;
        _services.GetRequiredService<CrashGuard>().Install(this, window, shell);
        shell.ConfigureStartupEntry(StartupRegistration.IsSupported ? new StartupRegistration() : null, Environment.ProcessPath);
        window.Show();
        if (!startupTheme.Succeeded)
        {
            shell.ShowRecovery(startupTheme);
        }

        // O Tako caiu durante um jogo com o ponteiro escondido, o PresentMon medindo ou um limite de FPS aplicado: desfaz.
        _services.GetRequiredService<SessionToolsService>().RecoverAfterCrash();

        // O Tako caiu durante um jogo e monitores ou áudio ficaram diferentes: oferece restaurar (MVP-012).
        shell.OfferEnvironmentRestore();

        // Descrição, gêneros, notas e artes dos jogos chegam em segundo plano (cache primeiro).
        shell.StartMetadataUpdates();

        // Procura e baixa atualizações em segundo plano (GitHub Releases, como no Console Mode); no modo seguro, só
        // depois de "Voltar ao normal".
        var updates = _services.GetRequiredService<UpdateService>();
        if (startup.SafeMode)
        {
            shell.EnterSafeMode(startup, journal, updates);
        }
        else
        {
            shell.StartAutoUpdate(updates);
        }

        // Tema Playnite escolhido na tela inicial (MVP-018/019): só depois da decisão do modo seguro, que o bloqueia.
        shell.ApplyPlayniteUi();
    }

    /// <summary>
    /// Composição de dependências. O teste de fumaça da UI passa uma biblioteca fictícia em <paramref name="library"/> e
    /// troca serviços que mexem no Windows (ex.: monitores e áudio) por simulados em <paramref name="testServices"/>.
    /// </summary>
    internal static ServiceProvider BuildServices(IGameLibrary? library = null, Action<IServiceCollection>? testServices = null)
    {
        var services = new ServiceCollection();
        services.AddLogging(logging => logging.AddDebug().SetMinimumLevel(LogLevel.Information));
        // Logs de diagnóstico em arquivo, sem dados sensíveis (MVP-033).
        services.AddSingleton(_ => new FileLoggerProvider(AppPaths.LogsDirectory, TimeProvider.System));
        services.AddSingleton<ILoggerProvider>(sp => sp.GetRequiredService<FileLoggerProvider>());
        // Biblioteca e configurações no SQLite (MVP-005).
        services.AddSingleton(_ => new TakoDatabase(AppPaths.DatabaseFile));
        services.AddSingleton<SettingsStore>();
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
        services.AddSingleton<SupervisorLink>();
        services.AddSingleton<UpdateService>();
        services.AddSingleton(sp => new SessionJournal(AppPaths.SessionFile, sp.GetRequiredService<TimeProvider>()));
        services.AddSingleton<CrashGuard>();
        services.AddSingleton<IGameLibrary>(sp => library ?? CreateLocalLibrary(sp));
        services.AddSingleton(XamlSecurityPolicy.CreateDefault());
        services.AddSingleton<XamlValidator>();
        services.AddSingleton<RestrictedXamlLoader>();
        services.AddSingleton<ThemeService>();
        services.AddSingleton<PlayniteThemeService>();
        services.AddSingleton<ArtService>();
        services.AddSingleton<SonyHidGamepad>();
        services.AddSingleton<SettingsService>();
        // Jogos do app Xbox / Microsoft Store (STORE-004): fonte isolada, ligada nas Configurações.
        services.AddSingleton<IXboxSystem, WindowsXboxSystem>();
        services.AddSingleton<XboxLibraryService>();
        // Central de Controle durante o jogo: janelas abertas, contador de FPS (PresentMon, ADR-0017), limite de FPS,
        // ponteiro, ControlFS e desempenho (MVP-002/030).
        services.AddSingleton<IWindowSwitcher, WindowsWindowSwitcher>();
        services.AddSingleton<IFpsCounter>(sp => new PresentMonFpsCounter(AppPaths.PresentMonExe, AppPaths.FpsCaptureMarker, sp.GetRequiredService<ILogger<PresentMonFpsCounter>>()));
        services.AddSingleton<IPerfLogAccess>(sp => new WindowsPerfLogAccess(Environment.ProcessPath ?? "Tako.exe", sp.GetRequiredService<ILogger<WindowsPerfLogAccess>>()));
        // Limite de FPS pelo driver de vídeo (NVAPI, ADLX ou IGCL), sem RTSS nem injeção (ADR-0018).
        services.AddSingleton<IFrameRateLimiter>(sp => WindowsFrameRateLimiter.Create(AppPaths.DriverFpsLimitBackupFile, sp.GetRequiredService<ILogger<SessionToolsService>>()));
        services.AddSingleton<PointerPadRelay>();
        services.AddSingleton<IPointerService>(sp => new WindowsPointer(AppPaths.CursorHiddenMarker, sp.GetRequiredService<PointerPadRelay>().Read, sp.GetRequiredService<ILogger<WindowsPointer>>()));
        services.AddSingleton<IControlFsService, WindowsControlFs>();
        services.AddSingleton<IHardwareMonitor, WindowsHardwareMonitor>();
        services.AddSingleton<SessionToolsService>();
        // Monitores e áudio da sessão (MVP-012): registrados antes do jogo e restaurados depois.
        services.AddSingleton<IDisplayConfigurationService, WindowsDisplayConfiguration>();
        services.AddSingleton<IAudioEndpointService, WindowsAudioEndpoints>();
        services.AddSingleton(_ => new PendingEnvironmentStore(AppPaths.EnvironmentSnapshotFile));
        services.AddSingleton<SessionEnvironmentGuard>();
        // Tela de jogo (MVP-002): monitores pela pilha portada do Console Mode.
        services.AddSingleton<IDisplayControlService, WindowsDisplayControl>();
        services.AddSingleton<IScreenCurtain, WindowsScreenCurtain>();
        services.AddSingleton(sp => new GameScreenSession(sp.GetRequiredService<IDisplayControlService>(), sp.GetRequiredService<IDisplayConfigurationService>(), sp.GetRequiredService<IScreenCurtain>(), sp.GetRequiredService<ILogger<GameScreenSession>>()));
        // Áudio, Steam e TV durante o jogo (MVP-002, ADR-0016).
        services.AddSingleton<ISteamClient, WindowsSteamClient>();
        services.AddSingleton<ITvControl>(sp => CreateTvControl(sp.GetRequiredService<ILogger<GameSessionDevices>>()));
        services.AddSingleton<GameSessionDevices>();
        services.AddSingleton<ShellViewModel>();
        services.AddSingleton<MainWindow>();
        testServices?.Invoke(services);
        return services.BuildServiceProvider();
    }

    /// <summary>Biblioteca real: jogos instalados na Steam e no app Xbox + adicionados pelo usuário (<c>tako.db</c>; importa o antigo <c>library.json</c>).</summary>
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

        var xbox = services.GetRequiredService<XboxLibraryService>();
        IReadOnlyList<Game> Discover() => [.. DiscoverSteam(), .. xbox.Discover()];

        var database = services.GetRequiredService<TakoDatabase>();
        var library = new LocalGameLibrary(Discover, new SqliteLibraryStore(database, AppPaths.LibraryFile), services.GetRequiredService<TimeProvider>(), services.GetRequiredService<MetadataService>().ApplyCached);
        library.Load();
        if (database.RecoveredFromCorruption)
        {
            LogDatabaseRecovered(logger, database.FilePath);
        }

        return library;
    }

    /// <summary>Google TV / Android TV pela rede (ADB); a chave deste PC fica cifrada com DPAPI na pasta de dados.</summary>
    private static AndroidTvControl CreateTvControl(ILogger logger) =>
        new(new AdbKeyStore(AppPaths.AdbKeyFile, new DpapiSecretProtector()), $"Tako@{Environment.MachineName}", e => GameSessionDevices.LogTvEvent(logger, e));

    /// <summary>Exceções que derrubariam o processo ficam registradas no log antes (MVP-033).</summary>
    private static void LogUnhandledExceptions(ServiceProvider services)
    {
        var logger = services.GetRequiredService<ILogger<App>>();
        var fileLog = services.GetRequiredService<FileLoggerProvider>();
        AppDomain.CurrentDomain.UnhandledException += (_, e) =>
        {
            LogFatalException(logger, e.ExceptionObject as Exception);
            fileLog.Flush(TimeSpan.FromSeconds(2));
        };
        TaskScheduler.UnobservedTaskException += (_, e) => LogUnobservedTaskException(logger, e.Exception);
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Banco corrompido renomeado para .corrompido; recomeçando vazio: {Path}")]
    private static partial void LogDatabaseRecovered(ILogger logger, string path);

    [LoggerMessage(Level = LogLevel.Critical, Message = "Exceção não tratada encerrando o Tako")]
    private static partial void LogFatalException(ILogger logger, Exception? exception);

    [LoggerMessage(Level = LogLevel.Error, Message = "Exceção não observada em tarefa")]
    private static partial void LogUnobservedTaskException(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Falha ao ler as bibliotecas da Steam em {SteamRoot}")]
    private static partial void LogSteamScanFailed(ILogger logger, Exception exception, string steamRoot);

    protected override void OnExit(ExitEventArgs e)
    {
        // Fechamento limpo apaga a "sessão em andamento"; saída por falha (CrashGuard) conta para o modo seguro.
        if (_services?.GetService<CrashGuard>() is { IsFatal: false })
        {
            _services.GetService<SessionJournal>()?.EndClean();
        }

        // Atualização baixada e não aplicada: instala agora, sem reabrir.
        _services?.GetService<ShellViewModel>()?.ApplyPendingUpdateOnExit();
        _services?.Dispose();
        base.OnExit(e);
    }
}
