using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Emulation;
using Launcher.Core.Library;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// "Configurar emulação" (ADR-0024): emuladores → pastas de ROMs → sistemas → pronto. Cada passo já salva o que foi
/// feito (sair no meio não desfaz nada) e diz o que falta em linguagem de usuário; tudo pode ser mudado depois em
/// Configurações › Emulação. Usa a mesma tela do assistente da TV (<see cref="SetupWizardViewModel"/>).
/// </summary>
public sealed class EmulationSetupWizard(ShellViewModel shell)
{
    private const int EmulatorsStep = 0;
    private const int FoldersStep = 1;
    private const int SystemsStep = 2;
    private const int DoneStep = 3;

    private SetupWizardViewModel? _view;
    private int _step;

    public SetupWizardViewModel? View => _view;

    public void Start()
    {
        _view = new SetupWizardViewModel("Configurar emulação", ["Emuladores", "ROMs", "Sistemas", "Pronto"], Back);
        shell.Emulation.Redetect();
        shell.Emulation.Changed += OnEmulationChanged;
        Go(EmulatorsStep);
        shell.PushOverlay(_view);
    }

    // Pasta adicionada pelo diálogo por cima do assistente: o passo mostra a lista nova.
    private void OnEmulationChanged(object? sender, EventArgs e)
    {
        if (_step == FoldersStep && _view is not null && shell.Overlays.Contains(_view))
        {
            ShowFolders();
        }
    }

    private bool Back()
    {
        if (_step is EmulatorsStep or DoneStep)
        {
            Close();
            return true;
        }

        Go(_step - 1);
        return true;
    }

    /// <summary>Exposto ao teste de fumaça: vai para um passo.</summary>
    internal void Go(int step)
    {
        _step = step;
        switch (step)
        {
            case EmulatorsStep:
                ShowEmulators();
                break;
            case FoldersStep:
                ShowFolders();
                break;
            case SystemsStep:
                ShowSystems();
                break;
            default:
                ShowDone();
                break;
        }

        shell.RequestFocus(null);
    }

    private void ShowEmulators()
    {
        var emulators = shell.Emulation.Inventory.Emulators;
        string text = emulators.Count == 0
            ? "Nenhum emulador encontrado neste PC. Instale o RetroArch (abre a maioria dos consoles antigos) ou o emulador do seu console, por exemplo DuckStation, PCSX2 ou Dolphin, e toque em \"Procurar de novo\". O Tako não baixa emuladores, BIOS nem jogos."
            : "Encontrados:\n" + string.Join('\n', emulators.Select(e => $"• {e.Name}{(e.UsesCores ? $" ({(e.Cores.Count == 0 ? "sem cores: instale pelo menu do RetroArch" : string.Join(", ", e.Cores.Select(c => c.Name)))})" : string.Empty)}"));
        _view!.Show(EmulatorsStep, emulators.Count == 0 ? "Vamos achar seus emuladores" : "Emuladores neste PC", text,
        [
            Action("Continuar", ActionStyle.Primary, () => Go(FoldersStep), isDefault: true),
            Action("Procurar de novo", ActionStyle.Secondary, () =>
            {
                shell.Emulation.Redetect();
                Go(EmulatorsStep);
            }),
            Action("Indicar onde está", ActionStyle.Secondary, () =>
            {
                Close();
                shell.OpenEmulators();
            }),
        ]);
    }

    private void ShowFolders()
    {
        var folders = shell.Emulation.Config.RomFolders;
        string text = folders.Count == 0
            ? "Escolha a pasta onde estão os seus jogos de console. Pode ser a pasta de um sistema ou uma pasta com subpastas por sistema (snes, psx, ps2…). Só use jogos que você tem o direito de jogar."
            : "Pastas:\n" + string.Join('\n', folders.Select(f => $"• {f.Path}{(f.System is { } s ? $" ({EmulationService.SystemName(s)})" : string.Empty)}"))
                + $"\n{ShellViewModel.RomCountText(shell.LibrarySources.Reports.GetValueOrDefault(GameStore.Emulation)?.GameCount ?? 0)} na biblioteca.";
        _view!.Show(FoldersStep, "Onde estão os seus jogos?", text,
        [
            Action(folders.Count == 0 ? "Escolher pasta" : "Adicionar outra pasta", ActionStyle.Primary, shell.AddRomFolder, isDefault: folders.Count == 0),
            Action(folders.Count == 0 ? "Pular" : "Continuar", folders.Count == 0 ? ActionStyle.Secondary : ActionStyle.Primary, () => Go(SystemsStep), isDefault: folders.Count > 0),
        ]);
    }

    private void ShowSystems()
    {
        var systems = shell.EmulationSystems;
        var lines = systems.Select(s => $"• {EmulationService.SystemName(s)}: {shell.Emulation.CurrentFor(s)?.Label ?? "falta emulador"}").ToList();
        var missing = shell.MissingBios;
        string text = systems.Count == 0
            ? "Nenhum sistema ainda: falta um emulador ou uma pasta com jogos."
            : string.Join('\n', lines);
        if (missing.Count > 0)
        {
            text += "\nBIOS faltando (o Tako não baixa; copie do seu console):\n" + string.Join('\n', missing.Select(m => "• " + m));
        }

        var actions = new List<OverlayAction>
        {
            Action("Continuar", ActionStyle.Primary, () => Go(DoneStep), isDefault: true),
            Action("Trocar emuladores", ActionStyle.Secondary, () =>
            {
                Close();
                shell.OpenEmulationSystems();
            }),
        };
        if (shell.Emulation.Inventory.Find("retroarch") is not null)
        {
            actions.Add(Action("Atalhos do RetroArch", ActionStyle.Secondary, shell.ApplyRetroArchHotkeys));
        }

        _view!.Show(SystemsStep, "Quem abre cada sistema", text, actions);
    }

    private void ShowDone()
    {
        int games = shell.LibrarySources.Reports.GetValueOrDefault(GameStore.Emulation)?.GameCount ?? 0;
        string exit = shell.Emulation.Config.ExitChord
            ? $"Durante o jogo, segure {shell.ExitChordLabel} para fechar o emulador e voltar ao Tako."
            : "O atalho para sair do emulador pelo controle está desligado.";
        _view!.Show(DoneStep, "Tudo pronto", $"{char.ToUpperInvariant(ShellViewModel.RomCountText(games)[0])}{ShellViewModel.RomCountText(games)[1..]} na biblioteca (fonte Emulação).\n{exit}\nVocê pode mudar tudo em Configurações › Emulação.",
            [Action("Concluir", ActionStyle.Primary, Close, isDefault: true)]);
    }

    private void Close()
    {
        shell.Emulation.Changed -= OnEmulationChanged;
        if (_view is not null && shell.Overlays.Contains(_view))
        {
            shell.CloseOverlay(_view);
        }

        shell.NotifySettingsChanged();
    }

    private static OverlayAction Action(string label, ActionStyle style, Action execute, bool isDefault = false) =>
        new(label, style, new RelayCommand(execute), isDefault);
}
