using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › Dados de ROMs (ScreenScraper), ADR-0022, e o mínimo para ROMs no "Adicionar jogo" (escolher o
/// sistema, sem abrir o jogo). Bloco isolado (uma linha nas Configurações e uma sobreposição) para a reorganização das
/// Configurações movê-lo para "Biblioteca e lojas" sem mexer na lógica.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>ROMs ainda não abrem pelo Tako: o épico EMU traz os emuladores.</summary>
    public const string RomLaunchNotice = "Abrir jogos de emulador chega numa próxima versão. Por enquanto o Tako mostra os dados e as artes da ROM.";

    /// <summary>"Indisponível nesta versão", "Desligado", "Ligado · sua conta", "Ligado · falta a conta" ou "Pausado · cota do dia".</summary>
    public string ScreenScraperSummary => (_metadata.HasScreenScraperAccess, _metadata.ScreenScraperSettings, _metadata.ScreenScraperPause) switch
    {
        (false, _, _) => "Indisponível nesta versão",
        (_, { IsActive: true }, ScreenScraperFailure.QuotaExceeded) => "Pausado · cota do dia",
        (_, { IsActive: true }, not null) => "Pausado · tenta mais tarde",
        (_, { IsActive: true }, _) => "Ligado · sua conta",
        (_, { Enabled: true }, _) => "Ligado · falta a conta",
        _ => "Desligado",
    };

    [RelayCommand]
    private void OpenScreenScraperSettings()
    {
        ScreenScraperSettingsViewModel? editor = null;
        editor = new ScreenScraperSettingsViewModel(
            _metadata.ScreenScraperSettings,
            _metadata.HasScreenScraperAccess,
            settings => _metadata.TestScreenScraperAsync(settings, CancellationToken.None),
            saved =>
            {
                CloseOverlay(editor!);
                _metadata.SaveScreenScraper(saved);
                OnPropertyChanged(nameof(ScreenScraperSummary));
                CurrentPage.Refresh();
                if (!saved.IsActive)
                {
                    Toast("ScreenScraper desligado · ROMs usam só a IGDB");
                }
                else if (!_metadata.HasScreenScraperAccess)
                {
                    Toast("Conta salva · o ScreenScraper chega numa próxima versão do Tako");
                }
                else
                {
                    // Só as ROMs que ainda não passaram pelo ScreenScraper são consultadas, aos poucos.
                    _ = _metadata.RefreshAsync(() => Library.Games);
                    Toast("ScreenScraper ligado · buscando dados das ROMs em segundo plano");
                }
            },
            () => OpenUri(ScreenScraperSettingsViewModel.SignUpUrl),
            ReadClipboard,
            () => CloseOverlay(editor!));
        PushOverlay(editor);
    }

    /// <summary>
    /// "Adicionar jogo" com uma ROM (ADR-0022): pergunta o sistema (os prováveis pela extensão primeiro; "Outro sistema"
    /// lista todos), grava <see cref="Game.Platform"/> e mostra os detalhes. O jogo não abre: falta o emulador (épico EMU).
    /// </summary>
    internal void AddRom(string path)
    {
        var candidates = ScreenScraperSystems.CandidatesFor(path);
        ChooseRomSystem(path, candidates.Count == 0 ? ScreenScraperSystems.All : candidates, showAllOption: candidates.Count > 0);
    }

    /// <summary>Para o teste de fumaça: adiciona a ROM já com o sistema escolhido.</summary>
    internal Game AddRomWithSystem(string path, EmulatedSystem system)
    {
        ArgumentNullException.ThrowIfNull(system);
        var game = Library.AddManual(ManualGameRecord.CreateRom(path, system.Key, _time.GetUtcNow()));
        Toast($"{game.Title} ({system.Name}) adicionado · buscando capa e informações");
        OpenDetails(ItemFor(game));
        return game;
    }

    private void ChooseRomSystem(string path, IReadOnlyList<EmulatedSystem> systems, bool showAllOption)
    {
        DialogViewModel? dialog = null;
        var actions = systems
            .Select((system, i) => new OverlayAction(system.Name, i == 0 ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                AddRomWithSystem(path, system);
            }), IsDefault: i == 0))
            .ToList();
        if (showAllOption)
        {
            actions.Add(new OverlayAction("Outro sistema", ActionStyle.Secondary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                ChooseRomSystem(path, ScreenScraperSystems.All, showAllOption: false);
            })));
        }

        actions.Add(new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))));
        dialog = new DialogViewModel(
            "De qual sistema é este jogo?",
            $"{System.IO.Path.GetFileName(path)}\nO Tako usa o sistema para achar os dados e as artes. Abrir pelo emulador chega numa próxima versão.",
            actions);
        PushOverlay(dialog);
    }

    /// <summary>Filtro do diálogo "Adicionar jogo": jogos de PC, atalhos e ROMs.</summary>
    private static OpenFileDialog AddGameDialog() => new()
    {
        Title = "Adicionar jogo",
        Filter = $"Jogos, atalhos e ROMs|*.exe;*.lnk;*.url;*.bat;*.cmd;{ScreenScraperSystems.FileDialogPattern()}|Jogos de PC e atalhos|*.exe;*.lnk;*.url;*.bat;*.cmd|ROMs e imagens de disco|{ScreenScraperSystems.FileDialogPattern()}|Todos os arquivos|*.*",
        CheckFileExists = true,
    };
}
