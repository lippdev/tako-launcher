using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.SessionTools;
using Launcher.UI.Wpf.Input;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › Mostrar FPS (ADR-0017). Ligar pede ao Windows, uma vez, para pôr o usuário no grupo "Usuários de
/// log de desempenho" (UAC). A tela do UAC não aceita o controle, então a opção só é acionável com teclado e mouse;
/// com o controle ela fica esmaecida e explica isso.
/// </summary>
public sealed partial class SettingsViewModel
{
    public const string UseKeyboardMessage = "Use teclado e mouse para ativar";
    public const string SignOutMessage = "Saia e entre no Windows de novo para o contador funcionar.";
    public const string WhyPermissionMessage = "Para medir o FPS sem tocar no jogo, o Windows precisa liberar o Tako uma única vez.";
    private const string NotBundledMessage = "O contador de FPS não veio nesta versão do Tako";
    private const string EnabledMessage = "Contador de FPS ligado · estilo e posição na Central de Controle";

    private bool _askingPermission;

    public bool FpsCounterOn => Tools.FpsCounterEnabled;

    /// <summary>Com o controle a opção fica esmaecida: o UAC que ela abre não aceita o controle.</summary>
    public bool FpsCounterDimmed => Shell.InputKind == InputKind.Gamepad;

    public string FpsCounterDetail => FpsCounterDimmed ? UseKeyboardMessage : Tools.CounterState switch
    {
        FpsCounterState.NotBundled => NotBundledMessage,
        FpsCounterState.NeedsSignOut => SignOutMessage,
        FpsCounterState.NeedsPermission => "Ligado, mas falta a permissão do Windows: desligue e ligue de novo para pedir",
        FpsCounterState.Ready => "Medido pelo Windows, sem tocar no jogo; estilo e posição na Central de Controle",
        _ => "Mede o FPS do jogo sem tocar nele; o Windows pede permissão uma única vez",
    };

    /// <summary>"Remover permissão" aparece para quem já está no grupo.</summary>
    public bool CanRevokeFpsPermission => Tools.PermissionState != PerfLogStatus.NotGranted && Tools.Fps.IsAvailable;

    private SessionToolsService Tools => Shell.SessionTools;

    /// <summary>Modo de entrada trocado: a opção esmaece (controle) ou volta (teclado e mouse).</summary>
    public void OnInputKindChanged() => NotifyFpsCounter();

    [RelayCommand]
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
            NotifyFpsCounter();
            Shell.Toast("Contador de FPS desligado");
            return;
        }

        switch (Tools.RefreshPermission())
        {
            case PerfLogStatus.Ready:
                Tools.SetFpsCounterEnabled(true);
                NotifyFpsCounter();
                Shell.Toast(EnabledMessage);
                break;
            case PerfLogStatus.NeedsSignOut:
                Tools.SetFpsCounterEnabled(true);
                NotifyFpsCounter();
                Shell.Toast(SignOutMessage);
                break;
            default:
                AskPermission();
                break;
        }
    }

    [RelayCommand]
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
        NotifyFpsCounter();
        Shell.Toast(result switch
        {
            PerfLogChange.Done => "Permissão removida · vale depois de sair e entrar no Windows",
            PerfLogChange.Cancelled => "Contador desligado · a permissão continua",
            _ => "Não foi possível remover a permissão · veja os logs",
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
                Shell.Toast(result == PerfLogChange.Cancelled ? "O Windows não liberou o contador · nada mudou" : "Não foi possível liberar o contador · veja os logs");
                return;
            }

            Tools.SetFpsCounterEnabled(true);
            Shell.Toast(Tools.RefreshPermission() == PerfLogStatus.Ready ? EnabledMessage : SignOutMessage);
        }
        finally
        {
            _askingPermission = false;
            NotifyFpsCounter();
        }
    }

    private void NotifyFpsCounter()
    {
        OnPropertyChanged(nameof(FpsCounterOn));
        OnPropertyChanged(nameof(FpsCounterDimmed));
        OnPropertyChanged(nameof(FpsCounterDetail));
        OnPropertyChanged(nameof(CanRevokeFpsPermission));
    }
}
