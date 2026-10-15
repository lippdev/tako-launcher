using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.SessionTools;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Input;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Modo Console › Contador de FPS e Limite de FPS (ADR-0017/0018). Ligar o contador pede ao Windows, uma vez, para pôr
/// o usuário no grupo "Usuários de log de desempenho" (UAC). A tela do UAC não aceita o controle, então ligar e remover
/// a permissão só funcionam com teclado e mouse; com o controle as linhas ficam esmaecidas e explicam isso.
/// </summary>
public sealed partial class ConsoleModeViewModel
{
    public const string UseKeyboardMessage = "Use teclado e mouse para ativar";
    public const string SignOutMessage = "Saia e entre no Windows de novo para o contador funcionar.";
    public const string WhyPermissionMessage = "Para medir o FPS sem tocar no jogo, o Windows precisa liberar o Tako uma única vez.";
    private const string NotBundledMessage = "O contador de FPS não veio nesta versão do Tako";
    private const string EnabledMessage = "Contador de FPS ligado · estilo e posição aqui e na Central de Controle";

    private bool _askingPermission;

    public bool FpsCounterOn => Tools.FpsCounterEnabled;

    /// <summary>Com o controle a opção fica esmaecida: o UAC que ela abre não aceita o controle.</summary>
    public bool FpsCounterDimmed => Shell.InputKind == InputKind.Gamepad;

    public string FpsCounterDetail => FpsCounterDimmed ? UseKeyboardMessage : Tools.CounterState switch
    {
        FpsCounterState.NotBundled => NotBundledMessage,
        FpsCounterState.NeedsSignOut => SignOutMessage,
        FpsCounterState.NeedsPermission => "Ligado, mas falta a permissão do Windows: desligue e ligue de novo para pedir",
        FpsCounterState.Ready => "Medido pelo Windows, sem tocar no jogo",
        _ => "Mede o FPS do jogo sem tocar nele; o Windows pede permissão uma única vez",
    };

    /// <summary>"Remover permissão" aparece para quem já está no grupo.</summary>
    public bool CanRevokeFpsPermission => Tools.PermissionState != PerfLogStatus.NotGranted && Tools.Fps.IsAvailable;

    public FeatureStatus FpsCounterStatus => ConsoleFeatures.FpsCounter(
        Tools.Fps.IsAvailable, Tools.FpsCounterEnabled, Tools.PermissionState, ControlCenterViewModel.StyleLabel(Tools.FpsStyle));

    public FeatureStatus FpsLimitStatus => ConsoleFeatures.FpsLimit(Tools.LimitCapabilities, Tools.SavedFpsLimit, Tools.DisplayRefreshHz);

    private SessionToolsService Tools => Shell.SessionTools;

    private IReadOnlyList<SettingRowViewModel> FpsCounterRows() =>
    [
        new("console:fps:on", SettingKind.Toggle, "Mostrar FPS", new RelayCommand(ToggleFpsCounter),
            description: () => FpsCounterDetail,
            isOn: () => FpsCounterOn,
            dimmed: () => FpsCounterDimmed),
        new("console:fps:style", SettingKind.Choice, "Estilo", new RelayCommand(() => ChangeTools(() => Tools.SetFpsStyle(ControlCenterViewModel.Next(FpsOverlay.Styles, Tools.FpsStyle, 0)))),
            description: () => "Só FPS, compacto (com CPU, GPU e memória) ou detalhado",
            value: () => ControlCenterViewModel.StyleLabel(Tools.FpsStyle),
            visible: () => FpsCounterOn),
        new("console:fps:position", SettingKind.Choice, "Posição", new RelayCommand(() => ChangeTools(() => Tools.SetLayout(Tools.Layout with { Position = ControlCenterViewModel.Next(FpsOverlay.Positions, Tools.Layout.Position, 0) }))),
            description: () => "Canto da tela do jogo onde o contador aparece",
            value: () => ControlCenterViewModel.PositionLabel(Tools.Layout.Position),
            visible: () => FpsCounterOn && Tools.FpsStyle != FpsOverlay.Off),
        new("console:fps:size", SettingKind.Choice, "Tamanho", new RelayCommand(() => ChangeTools(() => Tools.SetLayout(Tools.Layout with { Size = ControlCenterViewModel.Next(FpsOverlay.Sizes, Tools.Layout.Size, 0) }))),
            value: () => ControlCenterViewModel.SizeLabel(Tools.Layout.Size),
            visible: () => FpsCounterOn && Tools.FpsStyle != FpsOverlay.Off),
        new("console:fps:card", SettingKind.Toggle, "Fundo escuro", new RelayCommand(() => ChangeTools(() => Tools.SetLayout(Tools.Layout with { Card = !Tools.Layout.Card }))),
            description: () => "Deixa o contador legível sobre qualquer jogo",
            isOn: () => Tools.Layout.Card,
            visible: () => FpsCounterOn && Tools.FpsStyle != FpsOverlay.Off),
        new("console:fps:revoke", SettingKind.Action, "Remover a permissão do Windows", new AsyncRelayCommand(RevokeFpsPermissionAsync),
            description: () => FpsCounterDimmed ? UseKeyboardMessage : "Tira sua conta do grupo \"Usuários de log de desempenho\" do Windows (pede confirmação)",
            value: () => "Remover",
            visible: () => CanRevokeFpsPermission,
            dimmed: () => FpsCounterDimmed),
    ];

    private IReadOnlyList<SettingRowViewModel> FpsLimitRows() =>
    [
        new("console:fpslimit:value", SettingKind.Choice, "Limite em todos os jogos", new RelayCommand(() => ChangeTools(() => Tools.SetFpsLimit(FpsLimits.Cycle(Tools.SavedFpsLimit)))),
            description: () => "Um jogo pode ter um limite próprio em Configurações deste jogo",
            value: () => FpsLimits.Label(Tools.SavedFpsLimit, Tools.DisplayRefreshHz)),
        new("console:fpslimit:driver", SettingKind.Info, "Placa de vídeo", new RelayCommand(() => Shell.Toast(Tools.LimitCapabilities?.Explanation ?? "O driver é verificado ao abrir o primeiro jogo")),
            description: () => "Feito pelo driver da NVIDIA, AMD ou Intel, sem programas extras; o valor que você tinha volta ao sair do jogo",
            value: () => Tools.LimitCapabilities switch
            {
                null => "Verificada no primeiro jogo",
                { IsAvailable: true } caps => GpuVendors.Name(caps.Vendor),
                _ => "Sem suporte",
            }),
    ];

    /// <summary>Mudança nas ferramentas da sessão (estilo, posição, limite): as linhas e os cartões releem.</summary>
    private void ChangeTools(Action change)
    {
        change();
        Shell.NotifySettingsChanged();
    }

    private void ToggleFpsCounter()
    {
        if (FpsCounterDimmed)
        {
            Shell.Toast(UseKeyboardMessage);
            return;
        }

        if (Tools.CounterState == FpsCounterState.NotBundled)
        {
            Shell.Toast(NotBundledMessage);
            return;
        }

        if (Tools.FpsCounterEnabled)
        {
            Tools.SetFpsCounterEnabled(false);
            Shell.NotifySettingsChanged();
            Shell.Toast("Contador de FPS desligado");
            return;
        }

        switch (Tools.RefreshPermission())
        {
            case PerfLogStatus.Ready:
                Tools.SetFpsCounterEnabled(true);
                Shell.NotifySettingsChanged();
                Shell.Toast(EnabledMessage);
                break;
            case PerfLogStatus.NeedsSignOut:
                Tools.SetFpsCounterEnabled(true);
                Shell.NotifySettingsChanged();
                Shell.Toast(SignOutMessage);
                break;
            default:
                AskPermission();
                break;
        }
    }

    private async Task RevokeFpsPermissionAsync()
    {
        if (FpsCounterDimmed)
        {
            Shell.Toast("Use teclado e mouse para remover a permissão");
            return;
        }

        Tools.SetFpsCounterEnabled(false);
        var result = await Tools.Permission.RevokeAsync().ConfigureAwait(true);
        Tools.RefreshPermission();
        Shell.NotifySettingsChanged();
        Shell.Toast(result switch
        {
            PerfLogChange.Done => "Permissão removida · vale depois de sair e entrar no Windows",
            PerfLogChange.Cancelled => "Contador desligado · a permissão continua",
            _ => "Não foi possível remover a permissão · veja os registros",
        });
    }

    /// <summary>Explica antes do UAC; "Continuar" abre o pedido do Windows.</summary>
    private void AskPermission()
    {
        DialogViewModel? dialog = null;
        void Close() => Shell.CloseOverlay(dialog!);
        dialog = new DialogViewModel(
            "Mostrar FPS",
            WhyPermissionMessage + " Confirme na próxima tela do Windows. Depois, saia e entre no Windows de novo.",
            [
                new OverlayAction("Continuar", ActionStyle.Primary, new RelayCommand(() =>
                {
                    Close();
                    _ = GrantAsync();
                }), IsDefault: true),
                new OverlayAction("Agora não", ActionStyle.Secondary, new RelayCommand(Close)),
            ]);
        Shell.PushOverlay(dialog);
    }

    private async Task GrantAsync()
    {
        if (_askingPermission)
        {
            return;
        }

        _askingPermission = true;
        try
        {
            var result = await Tools.Permission.GrantAsync().ConfigureAwait(true);
            if (result != PerfLogChange.Done)
            {
                Shell.Toast(result == PerfLogChange.Cancelled ? "O Windows não liberou o contador · nada mudou" : "Não foi possível liberar o contador · veja os registros");
                return;
            }

            Tools.SetFpsCounterEnabled(true);
            Shell.Toast(Tools.RefreshPermission() == PerfLogStatus.Ready ? EnabledMessage : SignOutMessage);
        }
        finally
        {
            _askingPermission = false;
            Shell.NotifySettingsChanged();
        }
    }
}
