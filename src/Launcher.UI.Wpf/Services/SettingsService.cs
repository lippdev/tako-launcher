using System.IO;
using CommunityToolkit.Mvvm.ComponentModel;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Persistence;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Preferências do usuário, guardadas no <c>tako.db</c> (MVP-005): carregadas na abertura e salvas a cada mudança.
/// O que o usuário nunca mudou segue o padrão (ex.: "reduzir animações" acompanha o Windows).
/// </summary>
public sealed partial class SettingsService : ObservableObject
{
    private const string GameAudioIdKey = "GameAudioDeviceId";
    private const string GameAudioNameKey = "GameAudioDeviceName";

    private readonly SettingsStore _store;
    private readonly ILogger<SettingsService> _logger;
    private readonly bool _loading;

    [ObservableProperty]
    private bool _dynamicColor = true;

    [ObservableProperty]
    private bool _reducedMotion = SystemParametersReducedMotion();

    [ObservableProperty]
    private bool _startFullscreenOnLogin = true;

    /// <summary>Restaurar monitores e áudio ao sair do jogo (MVP-012).</summary>
    [ObservableProperty]
    private bool _restoreEnvironmentAfterGame = true;

    /// <summary>Saída de áudio durante o jogo (MVP-002); nula = não alterar.</summary>
    [ObservableProperty]
    private AudioOutputChoice? _gameAudioOutput;

    /// <summary>Mandar a Steam para a bandeja quando um jogo da Steam termina (MVP-002).</summary>
    [ObservableProperty]
    private bool _steamToTrayAfterGame;

    /// <summary>Ligar a TV e trocar a entrada (MVP-002); desligado por padrão.</summary>
    [ObservableProperty]
    private TvControlConfig _tvControl = TvControlConfig.Default;

    /// <summary>Configurações › Sistema › Opções de desenvolvedor (ADR-0020): ferramentas de tema; desligado por padrão.</summary>
    [ObservableProperty]
    private bool _developerOptions;

    public SettingsService(SettingsStore store, ILogger<SettingsService> logger)
    {
        _store = store;
        _logger = logger;
        _loading = true;
        try
        {
            var saved = store.LoadAll();
            DynamicColor = Read(saved, nameof(DynamicColor), DynamicColor);
            ReducedMotion = Read(saved, nameof(ReducedMotion), ReducedMotion);
            StartFullscreenOnLogin = Read(saved, nameof(StartFullscreenOnLogin), StartFullscreenOnLogin);
            RestoreEnvironmentAfterGame = Read(saved, nameof(RestoreEnvironmentAfterGame), RestoreEnvironmentAfterGame);
            LoadGameScreen(saved);
            LoadStores(saved);
            LoadShortcuts(saved);
            LoadCovers(saved);
            SteamToTrayAfterGame = Read(saved, nameof(SteamToTrayAfterGame), SteamToTrayAfterGame);
            GameAudioOutput = saved.TryGetValue(GameAudioIdKey, out string? audioId) && audioId.Length > 0
                ? new AudioOutputChoice(audioId, saved.GetValueOrDefault(GameAudioNameKey, audioId))
                : null;
            TvControl = TvControlConfigJson.Parse(saved.GetValueOrDefault(nameof(TvControl)));
            DeveloperOptions = Read(saved, nameof(DeveloperOptions), DeveloperOptions);
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            // Sem banco legível: segue com os padrões; o launcher abre do mesmo jeito.
            LogSettingsFailed(logger, ex, "ler");
        }
        finally
        {
            _loading = false;
        }
    }

    public TimeSpan FocusDuration => ReducedMotion ? TimeSpan.Zero : TimeSpan.FromMilliseconds(160);

    public TimeSpan PageDuration => ReducedMotion ? TimeSpan.Zero : TimeSpan.FromMilliseconds(280);

    public TimeSpan ColorDuration => ReducedMotion ? TimeSpan.Zero : TimeSpan.FromMilliseconds(600);

    partial void OnDynamicColorChanged(bool value) => Save(nameof(DynamicColor), value);

    partial void OnReducedMotionChanged(bool value) => Save(nameof(ReducedMotion), value);

    partial void OnStartFullscreenOnLoginChanged(bool value) => Save(nameof(StartFullscreenOnLogin), value);

    partial void OnRestoreEnvironmentAfterGameChanged(bool value) => Save(nameof(RestoreEnvironmentAfterGame), value);

    partial void OnSteamToTrayAfterGameChanged(bool value) => Save(nameof(SteamToTrayAfterGame), value);

    partial void OnGameAudioOutputChanged(AudioOutputChoice? value)
    {
        Save(GameAudioIdKey, value?.DeviceId ?? string.Empty);
        Save(GameAudioNameKey, value?.Name ?? string.Empty);
    }

    partial void OnTvControlChanged(TvControlConfig value) => Save(nameof(TvControl), TvControlConfigJson.Serialize(value));

    partial void OnDeveloperOptionsChanged(bool value) => Save(nameof(DeveloperOptions), value);

    private void Save(string key, bool value) => Save(key, value ? "true" : "false");

    private void Save(string key, string value)
    {
        if (_loading)
        {
            return;
        }

        try
        {
            _store.Set(key, value);
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            // A mudança vale nesta sessão mesmo que o disco falhe.
            LogSettingsFailed(_logger, ex, "salvar");
        }
    }

    private static bool Read(IReadOnlyDictionary<string, string> saved, string key, bool fallback) =>
        saved.TryGetValue(key, out string? text) && bool.TryParse(text, out bool value) ? value : fallback;

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível {Action} as configurações")]
    private static partial void LogSettingsFailed(ILogger logger, Exception exception, string action);

    // O Windows expõe "mostrar animações" em SystemParameters.ClientAreaAnimation.
    private static bool SystemParametersReducedMotion() => !System.Windows.SystemParameters.ClientAreaAnimation;
}

/// <summary>Opções da sessão de jogo ajustadas pela Central de Controle.</summary>
public sealed partial class SessionOptions : ObservableObject
{
    public static readonly string[] Outputs = ["TV (HDMI)", "Fone Bluetooth", "Alto-falantes"];
    public static readonly string[] Resolutions = ["3840 × 2160", "2560 × 1440", "1920 × 1080"];

    [ObservableProperty]
    private int _volume = 70;

    [ObservableProperty]
    private int _output;

    [ObservableProperty]
    private int _resolution;

    [ObservableProperty]
    private bool _hdr = true;

    [ObservableProperty]
    private int _captures;
}
