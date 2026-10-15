using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Platform;
using Launcher.Core.Resilience;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Resiliência (frente D): modo seguro depois de travamentos seguidos (MVP-036/021), tela de recuperação para erros
/// inesperados (MVP-011), "abrir ao iniciar o Windows" ligado ao registro (MVP-010) e o estado do Xbox mode (MVP-008).
/// </summary>
public sealed partial class ShellViewModel
{
    private readonly Lazy<FseDetector> _fse = new(() => new FseDetector());
    private SessionJournal? _journal;
    private UpdateService? _deferredUpdates;
    private StartupRegistration? _startupEntry;
    private string? _executablePath;
    private bool _syncingStartup;

    /// <summary>
    /// Modo seguro nesta abertura: tema padrão embutido (sem tema do disco), sem prévia automática de tema Playnite e
    /// sem atualizações automáticas. Outras telas devem consultar esta flag antes de carregar algo do disco sozinhas.
    /// </summary>
    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(SafeModeLabel), nameof(SafeModeDescription))]
    private bool _isSafeMode;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(SafeModeDescription))]
    private StartupReason _safeModeReason;

    public string SafeModeLabel => IsSafeMode ? "Ativo · voltar ao normal" : "Desligado";

    public string SafeModeDescription => (IsSafeMode, SafeModeReason) switch
    {
        (true, StartupReason.Requested) => "Aberto com --safe-mode: tema padrão embutido, sem atualizações automáticas",
        (true, _) => "O Tako não fechou corretamente duas vezes seguidas: tema padrão embutido, sem atualizações automáticas",
        _ => "Liga sozinho se o Tako não fechar corretamente duas vezes seguidas",
    };

    /// <summary>Xbox mode / Full Screen Experience neste PC (MVP-008), lido uma vez do registro.</summary>
    public FseAssessment FseStatus => _fse.Value.Assessment;

    public string FseDescription => $"{FseStatus.Reason} · o Tako abre em tela cheia convencional";

    // ---------------------------------------------------------------- Modo seguro

    /// <summary>Abertura em modo seguro: aviso com a opção de voltar ao normal; atualizações ficam para depois.</summary>
    public void EnterSafeMode(StartupDecision decision, SessionJournal journal, UpdateService updates)
    {
        ArgumentNullException.ThrowIfNull(decision);
        _journal = journal;
        _deferredUpdates = updates;
        SafeModeReason = decision.Reason;
        IsSafeMode = true;
        UpdateStatus = $"Versão {UpdateService.CurrentVersion} · modo seguro: atualizações automáticas pausadas nesta abertura";

        RecoveryViewModel? notice = null;
        notice = new RecoveryViewModel(
            decision.Reason == StartupReason.CrashLoop ? $"{decision.UncleanExits} aberturas seguidas sem fechar corretamente" : "Pedido na linha de comando (--safe-mode)",
            [
                new OverlayAction("Continuar no modo seguro", ActionStyle.Primary, new RelayCommand(() => CloseOverlay(notice!)), IsDefault: true),
                new OverlayAction("Voltar ao normal", ActionStyle.Secondary, new RelayCommand(() =>
                {
                    CloseOverlay(notice!);
                    ExitSafeMode();
                })),
                new OverlayAction("Sair para o desktop", ActionStyle.Secondary, new RelayCommand(ConfirmExit)),
            ])
        {
            Title = "Modo seguro",
            Message = "O Tako abriu com o tema padrão embutido e sem atualizações automáticas. Seus jogos e configurações não foram alterados.",
        };
        PushOverlay(notice);
    }

    /// <summary>Volta ao normal sem reiniciar: tema do disco, atualizações automáticas e falhas anteriores perdoadas.</summary>
    [RelayCommand]
    public void ExitSafeMode()
    {
        if (!IsSafeMode)
        {
            Toast("O modo seguro liga sozinho se o Tako não fechar corretamente duas vezes seguidas");
            return;
        }

        IsSafeMode = false;
        _journal?.Forgive();
        var theme = _themes.Apply(ThemeService.DefaultTheme);
        if (_deferredUpdates is { } updates)
        {
            _deferredUpdates = null;
            StartAutoUpdate(updates);
        }

        if (theme.Succeeded)
        {
            Toast("Modo normal: tema e atualizações automáticas de volta");
        }
        else
        {
            ShowRecovery(theme);
        }
    }

    // ---------------------------------------------------------------- Erros inesperados

    /// <summary>
    /// Erro inesperado na UI (MVP-011): a tela de recuperação sempre oferece um caminho de volta ao desktop. Erros
    /// repetidos são contados pelo <see cref="CrashGuard"/>, que fecha a tela cheia se a recuperação não adiantar.
    /// </summary>
    public void ShowErrorRecovery(Exception error)
    {
        ArgumentNullException.ThrowIfNull(error);
        if (Overlays.OfType<RecoveryViewModel>().Any(r => r.IsError))
        {
            return; // já na tela de recuperação
        }

        RecoveryViewModel? recovery = null;
        recovery = new RecoveryViewModel(
            $"{error.GetType().Name}: {error.Message}",
            [
                new OverlayAction("Continuar", ActionStyle.Primary, new RelayCommand(() =>
                {
                    CloseOverlay(recovery!);
                    if (!IsInGame)
                    {
                        GoHome();
                    }
                }), IsDefault: true),
                // Sem confirmação: com a UI instável, o caminho para o desktop precisa ser de um passo só.
                new OverlayAction("Sair para o desktop", ActionStyle.Secondary, new RelayCommand(() => System.Windows.Application.Current?.Shutdown())),
            ])
        {
            Title = "Algo deu errado",
            Message = "O Tako encontrou um erro e continua aberto. Se voltar a acontecer, saia para o desktop; o erro fica registrado nos logs de diagnóstico.",
            IsError = true,
        };
        PushOverlay(recovery);
    }

    // ---------------------------------------------------------------- Abrir ao iniciar o Windows

    /// <summary>
    /// Liga a opção "Abrir em tela cheia ao iniciar o Windows" ao registro. Sem <paramref name="registration"/>
    /// (build de desenvolvimento), a opção fica desligada e avisa que só funciona nas versões publicadas.
    /// </summary>
    public void ConfigureStartupEntry(StartupRegistration? registration, string? executablePath)
    {
        _startupEntry = registration;
        _executablePath = executablePath;
        SyncStartupSetting(registration is not null && executablePath is not null && registration.IsEnabled(executablePath));
        Settings.PropertyChanged += OnSettingsChangedForStartup;
    }

    private void OnSettingsChangedForStartup(object? sender, PropertyChangedEventArgs e)
    {
        if (_syncingStartup || e.PropertyName != nameof(SettingsService.StartFullscreenOnLogin))
        {
            return;
        }

        bool wanted = Settings.StartFullscreenOnLogin;
        if (_startupEntry is null || _executablePath is null)
        {
            SyncStartupSetting(false);
            Toast("Abrir ao iniciar o Windows só funciona nas versões instalada e portátil");
            return;
        }

        try
        {
            _startupEntry.Set(wanted, _executablePath);
            Toast(wanted ? "O Tako abre em tela cheia ao entrar no Windows" : "O Tako não abre mais ao entrar no Windows");
        }
        catch (Exception ex) when (StartupRegistration.IsExpectedFailure(ex))
        {
            Toast($"Não foi possível alterar a inicialização: {ex.Message}");
        }

        // Mostra o estado real do registro, mesmo se a gravação falhou.
        SyncStartupSetting(_startupEntry.IsEnabled(_executablePath));
    }

    private void SyncStartupSetting(bool enabled)
    {
        _syncingStartup = true;
        try
        {
            Settings.StartFullscreenOnLogin = enabled;
        }
        finally
        {
            _syncingStartup = false;
        }
    }

    [RelayCommand]
    private void ShowFseStatus() => Toast($"Xbox mode: {FseStatus.Label} · {FseStatus.Reason}");
}
