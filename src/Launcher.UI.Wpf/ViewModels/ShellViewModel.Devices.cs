using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Steam;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Library;
using Launcher.Core.Session;
using Launcher.Core.Session.Devices;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Áudio, Steam e TV da sessão de jogo (MVP-002, parte 2): opções nas Configurações e a ligação com o início e o fim da
/// sessão real. A lógica fica no <see cref="GameSessionDevices"/> (Core) e no ConsoleMode.Runtime.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Volume e saída de áudio para a Central de Controle (lê e ajusta o Windows na hora).</summary>
    public AudioVolumeControl AudioVolume => _devices.Volume;

    /// <summary>Resumo da TV na linha das Configurações.</summary>
    public string TvSummary => Settings.TvControl switch
    {
        { IsActive: true, StandbyAfterGame: true } tv => $"Ligada · {tv.Host} · {TvKinds.Label(tv.Kind)} · espera ao sair",
        { IsActive: true } tv => $"Ligada · {tv.Host} · {TvKinds.Label(tv.Kind)}",
        _ => "Desligado",
    };

    /// <summary>Saída de áudio do jogo vem do efetivo (UX-007: o jogo pode ter a própria); Steam e TV são do Tako.</summary>
    private SessionDeviceOptions DeviceOptions(EffectiveSessionSettings effective) =>
        new(effective.GameAudio, Settings.SteamToTrayAfterGame, Settings.TvControl);

    /// <summary>Jogo aberto e acompanhado: TV (se ainda não foi ligada) e saída de áudio em segundo plano.</summary>
    private void BeginSessionDevices(Game game, bool tvReady, EffectiveSessionSettings effective) =>
        _ = _devices.BeginSession(game.Id, DeviceOptions(effective), tvReady);

    /// <summary>
    /// Com a "Tela de jogo" (ADR-0015) e a TV ligadas, a TV precisa estar ligada na entrada do PC antes de a tela de
    /// jogo ser focada: liga aqui e espera (com prazo). Depois disso a própria tela de jogo espera a imagem aparecer;
    /// não há segunda espera. Sem a tela de jogo, a TV liga em segundo plano junto com o jogo. Devolve se a TV já foi
    /// tratada aqui.
    /// </summary>
    private async Task<bool> WakeTvForGameScreenAsync(Game game, EffectiveSessionSettings effective)
    {
        if (effective.GameScreen is null || !Settings.TvControl.IsActive)
        {
            return false;
        }

        Toast("Ligando a TV…");
        await _devices.WakeTvAsync(game.Id, Settings.TvControl).ConfigureAwait(true);
        return true;
    }

    /// <summary>
    /// Antes de restaurar monitores e áudio: devolve a saída de áudio, Steam para a bandeja e TV em espera. O texto
    /// volta para o aviso de "bem-vindo".
    /// </summary>
    private string EndSessionDevices(GameItemViewModel? item, bool gameEnded)
    {
        var game = item?.Game;
        var report = _devices.EndSession(game?.Id ?? "-", game?.Store == GameStore.Steam, game?.SteamAppId, gameEnded);
        return report.SteamSentToTray ? " · Steam na bandeja" : string.Empty;
    }

    /// <summary>Configurações › Saída de áudio durante o jogo: lista as saídas ativas e "Não alterar".</summary>
    [RelayCommand]
    private void ChooseGameAudio()
    {
        IReadOnlyList<AudioDevice> outputs;
        try
        {
            if (!AudioVolume.CanChangeOutput)
            {
                Toast("Este Windows não deixa o Tako trocar a saída de áudio");
                return;
            }

            outputs = AudioVolume.Outputs();
        }
        catch (RuntimeEnvironmentException)
        {
            Toast("Não foi possível ler as saídas de áudio do Windows");
            return;
        }

        var current = Settings.GameAudioOutput;
        var choices = outputs.Select(d => new AudioOutputChoice(d.Id, d.Name)).ToList();
        if (current is not null && !choices.Any(c => string.Equals(c.DeviceId, current.DeviceId, StringComparison.OrdinalIgnoreCase)))
        {
            choices.Insert(0, current); // desconectada agora (ex.: TV desligada): continua escolhível
        }

        DialogViewModel? dialog = null;
        void Pick(AudioOutputChoice? choice)
        {
            CloseOverlay(dialog!);
            Settings.GameAudioOutput = choice;
            Toast(choice is null ? "A saída de áudio não muda durante o jogo" : $"Durante o jogo, o som sai por {choice.Name}");
        }

        string Label(AudioOutputChoice choice)
        {
            string name = outputs.Any(d => string.Equals(d.Id, choice.DeviceId, StringComparison.OrdinalIgnoreCase)) ? choice.Name : $"{choice.Name} (desconectada)";
            return Same(choice, current) ? $"{name} ✓" : name;
        }

        var actions = new List<OverlayAction>
        {
            new(current is null ? "Não alterar ✓" : "Não alterar", ActionStyle.Secondary, new RelayCommand(() => Pick(null)), IsDefault: current is null),
        };
        actions.AddRange(choices.Select(c => new OverlayAction(Label(c), Same(c, current) ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(() => Pick(c)), IsDefault: Same(c, current))));
        dialog = new DialogViewModel(
            "Saída de áudio durante o jogo",
            "Ao abrir um jogo, o som passa a sair por aqui; ao fechar, volta para a saída de antes.",
            actions);
        PushOverlay(dialog);

        static bool Same(AudioOutputChoice a, AudioOutputChoice? b) => b is not null && string.Equals(a.DeviceId, b.DeviceId, StringComparison.OrdinalIgnoreCase);
    }

    [RelayCommand]
    private void ToggleSteamTray() => Settings.SteamToTrayAfterGame = !Settings.SteamToTrayAfterGame;

    /// <summary>Configurações › TV: tipo (Android, LG, Samsung ou Detectar), endereço, entrada, espera ao sair e Testar (que pareia na primeira vez).</summary>
    [RelayCommand]
    private void OpenTvSettings()
    {
        TvSettingsViewModel? editor = null;
        editor = new TvSettingsViewModel(
            Settings.TvControl,
            config => _devices.TestTvAsync(config, CancellationToken.None),
            host => _devices.DetectTvAsync(host, CancellationToken.None),
            saved =>
            {
                CloseOverlay(editor!);
                Settings.TvControl = saved;
                OnPropertyChanged(nameof(TvSummary));
                Toast(saved.IsActive ? "TV configurada" : "Controle da TV desligado");
            },
            () => CloseOverlay(editor!));
        PushOverlay(editor);
    }
}
