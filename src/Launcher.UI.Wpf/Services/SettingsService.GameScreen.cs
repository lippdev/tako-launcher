using CommunityToolkit.Mvvm.ComponentModel;
using ConsoleMode.Runtime.Display;

namespace Launcher.UI.Wpf.Services;

/// <summary>"Tela de jogo" (MVP-002): desligada por padrão; a tela é guardada pelo id estável do monitor.</summary>
public sealed partial class SettingsService
{
    [ObservableProperty]
    private bool _gameScreenEnabled;

    /// <summary>Id do monitor (caminho do dispositivo); vazio até o usuário escolher.</summary>
    [ObservableProperty]
    private string _gameScreenId = string.Empty;

    /// <summary>Nome da tela escolhida, para mostrar mesmo com ela desligada ou desconectada.</summary>
    [ObservableProperty]
    private string _gameScreenName = string.Empty;

    [ObservableProperty]
    private HideOthersMode _gameScreenHideOthers = HideOthersMode.Disconnect;

    /// <summary>Resolução e taxa (<c>3840x2160@120</c>); vazio mantém o modo atual da tela.</summary>
    [ObservableProperty]
    private string _gameScreenMode = string.Empty;

    [ObservableProperty]
    private bool _gameScreenHdr;

    /// <summary>Última combinação tela + modo confirmada em "Você consegue ver esta tela?".</summary>
    [ObservableProperty]
    private string _gameScreenConfirmedSetup = string.Empty;

    /// <summary>"Ao abrir o Tako, ir para a tela de jogo" (modo console); desligado por padrão.</summary>
    [ObservableProperty]
    private bool _consoleModeOnLaunch;

    /// <summary>Preferências prontas para a sessão; nulo se a opção estiver desligada ou sem tela escolhida.</summary>
    public GameScreenSettings? GameScreen => GameScreenEnabled && GameScreenId.Length > 0
        ? new GameScreenSettings(GameScreenId, GameScreenHideOthers, DisplayMode.TryParse(GameScreenMode), GameScreenHdr)
        : null;

    partial void OnGameScreenEnabledChanged(bool value) => Save(nameof(GameScreenEnabled), value);

    partial void OnGameScreenIdChanged(string value) => Save(nameof(GameScreenId), value);

    partial void OnGameScreenNameChanged(string value) => Save(nameof(GameScreenName), value);

    partial void OnGameScreenHideOthersChanged(HideOthersMode value) => Save(nameof(GameScreenHideOthers), value.ToString());

    partial void OnGameScreenModeChanged(string value) => Save(nameof(GameScreenMode), value);

    partial void OnGameScreenHdrChanged(bool value) => Save(nameof(GameScreenHdr), value);

    partial void OnGameScreenConfirmedSetupChanged(string value) => Save(nameof(GameScreenConfirmedSetup), value);

    partial void OnConsoleModeOnLaunchChanged(bool value) => Save(nameof(ConsoleModeOnLaunch), value);

    private void LoadGameScreen(IReadOnlyDictionary<string, string> saved)
    {
        GameScreenEnabled = Read(saved, nameof(GameScreenEnabled), GameScreenEnabled);
        GameScreenId = saved.GetValueOrDefault(nameof(GameScreenId)) ?? string.Empty;
        GameScreenName = saved.GetValueOrDefault(nameof(GameScreenName)) ?? string.Empty;
        GameScreenHideOthers = Enum.TryParse(saved.GetValueOrDefault(nameof(GameScreenHideOthers)), out HideOthersMode hide) && Enum.IsDefined(hide) ? hide : HideOthersMode.Disconnect;
        GameScreenMode = DisplayMode.TryParse(saved.GetValueOrDefault(nameof(GameScreenMode)))?.Key ?? string.Empty;
        GameScreenHdr = Read(saved, nameof(GameScreenHdr), GameScreenHdr);
        GameScreenConfirmedSetup = saved.GetValueOrDefault(nameof(GameScreenConfirmedSetup)) ?? string.Empty;
        ConsoleModeOnLaunch = Read(saved, nameof(ConsoleModeOnLaunch), ConsoleModeOnLaunch);
    }
}
