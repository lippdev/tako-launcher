using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Menu Modo Console (ADR-0020): tudo que veio do Console Mode num lugar só, fora das Configurações. Um cartão por
/// recurso com o estado claro (ligado, desligado, falta configurar, com problema); A abre o detalhe do recurso. No topo,
/// o assistente "Configurar para jogar na TV" guia do zero: tela, imagem, TV, áudio e pronto.
/// </summary>
public sealed partial class ConsoleModeViewModel : RowsPageViewModel
{
    public const string Route = "console";

    /// <summary>Tela de jogo conectada agora (nulo = não deu para ler); lida ao abrir a página e depois de mudanças.</summary>
    private bool? _screenConnected;

    /// <summary>Saída de áudio do jogo conectada agora (nulo = não deu para ler).</summary>
    private bool? _audioConnected;

    public ConsoleModeViewModel(ShellViewModel shell)
        : base(shell)
    {
        Probe();
        Cards =
        [
            Card("gamescreen", "Tela de jogo", "Icon.Monitor", () => GameScreenStatus, OpenGameScreen),
            Card("tv", "TV", "Icon.Tv", () => ConsoleFeatures.Tv(Settings.TvControl), () => Shell.OpenTvSettingsCommand.Execute(null)),
            Card("audio", "Áudio do jogo", "Icon.Volume", () => GameAudioStatus, () => Shell.ChooseGameAudioCommand.Execute(null)),
            Card("steam", "Steam na bandeja", "Store.Steam", () => ConsoleFeatures.SteamTray(Settings.SteamToTrayAfterGame), OpenSteamTray),
            Card("fps", "Contador de FPS", "Icon.Gauge", () => FpsCounterStatus, OpenFpsCounter),
            Card("fpslimit", "Limite de FPS", "Icon.Limit", () => FpsLimitStatus, OpenFpsLimit),
        ];
    }

    public override string Title => "Modo Console";

    public override string RouteKey => Route;

    public override string? DefaultFocusKey => "console:wizard";

    /// <summary>Um cartão por recurso, na ordem do assistente.</summary>
    public IReadOnlyList<SettingRowViewModel> Cards { get; }

    public SettingsService Settings => Shell.Settings;

    public FeatureStatus GameAudioStatus => ConsoleFeatures.GameAudio(Settings.GameAudioOutput?.Name, _audioConnected);

    protected override IEnumerable<SettingRowViewModel> AllRows => Cards;

    public SettingRowViewModel CardFor(string id) => Cards.First(c => c.Key == $"console:{id}");

    public override void Refresh()
    {
        Probe();
        base.Refresh();
    }

    /// <summary>Lê se a tela de jogo e a saída de áudio escolhidas estão conectadas (barato, mas não a cada toque).</summary>
    public void Probe()
    {
        try
        {
            _screenConnected = Settings.GameScreenId.Length == 0
                ? null
                : Shell.GameScreen.ListDisplays().Any(d => string.Equals(d.Id, Settings.GameScreenId, StringComparison.OrdinalIgnoreCase));
        }
        catch (RuntimeEnvironmentException)
        {
            _screenConnected = null;
        }

        try
        {
            _audioConnected = Settings.GameAudioOutput is not { } output
                ? null
                : Shell.AudioVolume.Outputs().Any(d => string.Equals(d.Id, output.DeviceId, StringComparison.OrdinalIgnoreCase));
        }
        catch (RuntimeEnvironmentException)
        {
            _audioConnected = null;
        }
    }

    [RelayCommand]
    private void StartWizard() => new TvSetupWizard(this, Shell).Start();

    /// <summary>Assistente › Tela: liga a tela de jogo com a tela escolhida.</summary>
    public void ChooseScreenForWizard(ConsoleMode.Runtime.Display.ConnectedDisplay display)
    {
        ArgumentNullException.ThrowIfNull(display);
        ChooseGameScreen(display);
        Settings.GameScreenEnabled = true;
    }

    public void OpenGameScreen() => OpenDetail("gamescreen", GameScreenRows());

    private void OpenSteamTray() => OpenDetail("steam",
    [
        new("console:steam:on", SettingKind.Toggle, "Mandar a Steam para a bandeja ao sair do jogo", new RelayCommand(() => Shell.ToggleSteamTrayCommand.Execute(null)),
            description: () => "Fecha só a janela da Steam (ela continua aberta na bandeja), e só se nenhum outro jogo estiver rodando",
            isOn: () => Settings.SteamToTrayAfterGame),
    ]);

    public void OpenFpsCounter()
    {
        Tools.RefreshPermission(); // o usuário pode ter saído e entrado no Windows desde a última vez
        OpenDetail("fps", FpsCounterRows());
    }

    private void OpenFpsLimit() => OpenDetail("fpslimit", FpsLimitRows());

    private void OpenDetail(string id, IReadOnlyList<SettingRowViewModel> rows) =>
        Shell.OpenConsoleFeature(new SettingsDetailViewModel(Shell, CardFor(id), rows, Refresh));

    private static SettingRowViewModel Card(string id, string title, string icon, Func<FeatureStatus> status, Action open) =>
        new($"console:{id}", SettingKind.Action, title, new RelayCommand(open),
            description: () => status().Summary,
            value: () => status().Badge,
            state: () => status().State,
            icon: icon);
}

/// <summary>
/// Detalhe de um recurso do Modo Console: o cartão do recurso no topo (com o estado) e as linhas dele. B volta aos
/// cartões, com o foco no cartão de origem.
/// </summary>
public sealed class SettingsDetailViewModel(ShellViewModel shell, SettingRowViewModel feature, IReadOnlyList<SettingRowViewModel> rows, Action? beforeRefresh = null) : RowsPageViewModel(shell)
{
    public SettingRowViewModel Feature { get; } = feature;

    public IReadOnlyList<SettingRowViewModel> Rows { get; } = rows;

    public override string Title => Feature.Title;

    public override string RouteKey => ConsoleModeViewModel.Route;

    public override string? DefaultFocusKey => Rows.FirstOrDefault(r => r.IsShown && !r.IsHeader)?.Key;

    protected override IEnumerable<SettingRowViewModel> AllRows => Rows.Prepend(Feature);

    public override void Refresh()
    {
        beforeRefresh?.Invoke();
        base.Refresh();
    }
}
