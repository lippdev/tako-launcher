using Launcher.Core.Library;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Configurações › Emulação (épico EMU, ADR-0024): categoria própria (ADR-0020), com o assistente no topo.</summary>
public sealed partial class SettingsViewModel
{
    private List<SettingRowViewModel> EmulationRows() =>
    [
        Row("emu-setup", SettingKind.Action, "Configurar emulação", Shell.StartEmulationSetup,
            description: () => "Acha os emuladores, escolhe as pastas de ROMs e diz o que falta, passo a passo",
            value: () => "Começar"),
        Header("emu-games", "Jogos"),
        StoreRow("emulation", GameStore.Emulation, "Mostrar jogos de emulador", "Icon.Gamepad"),
        Row("emu-folders", SettingKind.Action, "Pastas de ROMs", () => Shell.OpenRomFolders(),
            description: () => "Pastas de um sistema ou com subpastas por sistema (snes, psx, ps2…)",
            value: () => Shell.Emulation.Config.RomFolders.Count switch
            {
                0 => "Adicionar",
                1 => "1 pasta",
                int n => $"{n} pastas",
            }),
        Row("emu-rescan", SettingKind.Action, "Procurar emuladores e ROMs", Shell.RescanEmulation,
            description: () => Shell.EmulationSummary,
            value: () => "Procurar"),
        Header("emu-emulators", "Emuladores"),
        Row("emu-list", SettingKind.Action, "Emuladores", () => Shell.OpenEmulators(),
            description: () => Shell.Emulation.Inventory.Emulators.Count == 0
                ? "Nenhum encontrado: instale o RetroArch ou o emulador do seu console, ou indique onde está"
                : string.Join(", ", Shell.Emulation.Inventory.Emulators.Select(e => e.Name)),
            value: () => Shell.Emulation.Inventory.Emulators.Count.ToString(System.Globalization.CultureInfo.InvariantCulture)),
        Row("emu-systems", SettingKind.Action, "Emulador de cada sistema", () => Shell.OpenEmulationSystems(),
            description: () => "Escolha o emulador (e o core do RetroArch) de cada console; o sugerido já vem marcado",
            value: () => $"{Shell.EmulationSystems.Count} sistemas"),
        Row("emu-bios", SettingKind.Info, "BIOS e firmware", Shell.ShowMissingBios,
            description: () => Shell.MissingBios is { Count: > 0 } missing ? missing[0] : "Nada faltando nos emuladores em uso; o Tako nunca baixa BIOS",
            value: () => Shell.MissingBios.Count switch
            {
                0 => "Tudo certo",
                1 => "1 faltando",
                int n => $"{n} faltando",
            }),
        Header("emu-play", "Durante o jogo"),
        Row("emu-fullscreen", SettingKind.Toggle, "Abrir em tela cheia", Shell.ToggleEmulationFullscreen,
            description: () => "Pede tela cheia ao emulador quando ele aceita pela linha de comando",
            isOn: () => Shell.Emulation.Config.Fullscreen),
        Row("emu-exit", SettingKind.Toggle, "Sair do emulador pelo controle", Shell.ToggleEmulatorExitChord,
            description: () => $"Segure {Shell.ExitChordLabel} (Select + Start) para fechar o emulador e voltar ao Tako",
            isOn: () => Shell.Emulation.Config.ExitChord),
        Row("emu-retroarch", SettingKind.Action, "Atalhos do RetroArch", Shell.ApplyRetroArchHotkeys,
            description: () => Shell.Emulation.Inventory.Find("retroarch") is null
                ? "Para quando o RetroArch estiver instalado"
                : "Select + Start também fecha pelo próprio RetroArch; só muda o que você não mudou e guarda uma cópia do retroarch.cfg",
            value: () => "Aplicar"),
    ];
}
