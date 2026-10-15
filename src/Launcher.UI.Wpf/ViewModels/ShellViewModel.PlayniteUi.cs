using System.IO;
using System.Windows;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Input;
using Launcher.PlayniteCompat.P10;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Tema Playnite na interface inteira (frente F, MVP-018/019/021): com um tema Playnite escolhido em Configurações ›
/// Temas, a tela inicial do Tako passa a ser o <c>Views/Main.xaml</c> do tema. Outras telas (biblioteca, detalhes do
/// Tako, Configurações) e as sobreposições continuam do Tako, e Menu/☰ sempre abre as Configurações. Qualquer falha
/// volta à UI padrão com a tela de recuperação; no modo seguro o tema não é aplicado.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Tamanho do palco usado no ensaio do tema (o layout do Playnite é desenhado em 1920×1080).</summary>
    private static readonly Size PlayniteStage = new(1920, 1080);

    /// <summary>Tema Playnite aplicado à tela inicial; nulo = UI padrão do Tako.</summary>
    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsPlayniteUiVisible))]
    private PlayniteUiViewModel? _playniteUi;

    /// <summary>A tela do tema está à vista: tela inicial, sem sobreposição de página do Tako por cima.</summary>
    public bool IsPlayniteUiVisible => PlayniteUi is not null && CurrentPage is HomeViewModel;

    /// <summary>
    /// Aplica o tema Playnite escolhido (ou volta à UI do Tako sem escolha). Chamado na abertura (depois da decisão do
    /// modo seguro), ao escolher um tema e ao sair do modo seguro. Nunca lança: falha vira a tela de recuperação.
    /// </summary>
    public void ApplyPlayniteUi()
    {
        var theme = _playniteThemes.SelectedForUi();
        if (theme is null || IsSafeMode)
        {
            ClosePlayniteUi();
            return;
        }

        P10ThemeReport report;
        try
        {
            report = _playniteThemes.LoadDirectory(theme.Directory);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            FailPlayniteUi($"Não foi possível ler o tema: {ex.Message}", theme.Manifest.Manifest.DisplayName);
            return;
        }

        var ui = new PlayniteUiViewModel(this, report, theme);
        if (P10Theme.Probe(report, ui.Model, ui.Api, PlayniteStage) is { } problem)
        {
            ui.Dispose();
            FailPlayniteUi(problem, report.Name);
            return;
        }

        PlayniteUi?.Dispose();
        PlayniteUi = ui;
        LogPlayniteUi(_logger, report.Name, report.MainFromTheme);
        AfterPlayniteUiChanged();
    }

    /// <summary>Volta à UI padrão do Tako (a escolha salva não muda).</summary>
    public void ClosePlayniteUi()
    {
        if (PlayniteUi is null)
        {
            return;
        }

        PlayniteUi.Dispose();
        PlayniteUi = null;
        AfterPlayniteUiChanged();
    }

    /// <summary>
    /// O tema falhou (ensaio, carga ou erro de layout/XAML em uso): UI padrão e tela de recuperação. Não reaplica
    /// sozinho, então não há loop; "Tentar de novo" é escolha do usuário.
    /// </summary>
    public void FailPlayniteUi(string reason, string? themeName = null)
    {
        string name = themeName ?? PlayniteUi?.Name ?? "Tema Playnite";
        LogPlayniteUiFailed(_logger, name, reason);
        ClosePlayniteUi();
        if (Overlays.OfType<RecoveryViewModel>().Any(r => r.Title == PlayniteRecoveryTitle))
        {
            return;
        }

        RecoveryViewModel? recovery = null;
        recovery = new RecoveryViewModel(
            $"{name}: {reason}",
            [
                new OverlayAction("Usar a interface do Tako", ActionStyle.Primary, new RelayCommand(() =>
                {
                    CloseOverlay(recovery!);
                    if (TryChangeSelection(null))
                    {
                        RefreshThemesPage();
                        Toast("Interface do Tako em uso");
                    }
                }), IsDefault: true),
                new OverlayAction("Tentar de novo", ActionStyle.Secondary, new RelayCommand(() =>
                {
                    CloseOverlay(recovery!);
                    ApplyPlayniteUi();
                })),
                new OverlayAction("Sair para o desktop", ActionStyle.Secondary, new RelayCommand(ConfirmExit)),
            ])
        {
            Title = PlayniteRecoveryTitle,
            Message = "O Tako voltou para a interface padrão. Seus jogos e configurações não foram alterados.",
        };
        PushOverlay(recovery);
    }

    /// <summary>A lista do tema foi reconstruída (biblioteca mudou): o foco volta ao jogo selecionado.</summary>
    internal void OnPlayniteUiListChanged()
    {
        if (IsPlayniteUiVisible && TopOverlay is null && PlayniteUi is { IsDetailsOpen: false } ui)
        {
            FocusPlayniteLater(ui.SelectedFocusKey);
        }
    }

    /// <summary>Detalhes do jogo abriram ou fecharam dentro do tema: foco no botão principal ou de volta na capa.</summary>
    internal void OnPlayniteDetailsChanged()
    {
        UpdateHints();
        if (IsPlayniteUiVisible && TopOverlay is null && PlayniteUi is { } ui)
        {
            FocusPlayniteLater(ui.IsDetailsOpen ? "p10-details:play" : ui.SelectedFocusKey);
        }
    }

    /// <summary>B/Esc com o tema à vista: fecha os detalhes do tema antes de voltar páginas.</summary>
    private bool TryBackInPlayniteUi() => IsPlayniteUiVisible && TopOverlay is null && PlayniteUi!.HandleBack();

    partial void OnClockChanged(string value) => PlayniteUi?.UpdateClock(value);

    private const string PlayniteRecoveryTitle = "O tema não pôde ser aplicado";

    private void AfterPlayniteUiChanged()
    {
        OnPropertyChanged(nameof(IsPlayniteUiVisible));
        UpdateHints();
        if (TopOverlay is null)
        {
            // Com o tema: capa selecionada. Sem ele (tema fechado na tela inicial): o foco volta à página do Tako.
            FocusPlayniteLater(IsPlayniteUiVisible ? PlayniteUi?.SelectedFocusKey : CurrentPage.DefaultFocusKey);
        }
    }

    /// <summary>Página mudou: a tela do tema aparece (tela inicial) ou some (outras páginas do Tako).</summary>
    private void OnPageChangedForPlaynite()
    {
        OnPropertyChanged(nameof(IsPlayniteUiVisible));
        if (IsPlayniteUiVisible && TopOverlay is null)
        {
            FocusPlayniteLater(PlayniteUi?.SelectedFocusKey);
        }
    }

    /// <summary>
    /// Pede o foco depois que os elementos do tema entraram na árvore (os <c>PART_</c> se anunciam no Loaded), senão a
    /// navegação ainda não os enxerga.
    /// </summary>
    private void FocusPlayniteLater(string? key) =>
        Dispatcher.CurrentDispatcher.BeginInvoke(() => RequestFocus(key), DispatcherPriority.ContextIdle);

    /// <summary>Dicas com o tema à vista: o tema tem os próprios botões; ☰ sempre leva às Configurações do Tako.</summary>
    private void AddPlayniteHints(List<(string Buttons, string Label, AppCommand Command)> list)
    {
        bool details = PlayniteUi?.IsDetailsOpen == true;
        list.Add(("a", details ? "Selecionar" : "Detalhes", AppCommand.Accept));
        if (details)
        {
            list.Add(("b", "Voltar", AppCommand.Back));
        }

        if (_backgroundSession is not null)
        {
            list.Add(("guide", "Voltar ao jogo", AppCommand.Guide));
        }

        list.Add(("lb+rb", "Seção", AppCommand.NextSection));
        list.Add(("menu", "Configurações do Tako", AppCommand.Menu));
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Tema Playnite {Theme} aplicado à interface (Main.xaml do tema: {MainFromTheme})")]
    private static partial void LogPlayniteUi(ILogger logger, string theme, bool mainFromTheme);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Tema Playnite {Theme} não aplicado à interface: {Reason}")]
    private static partial void LogPlayniteUiFailed(ILogger logger, string theme, string reason);
}
