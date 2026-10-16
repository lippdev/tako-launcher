using System.ComponentModel;
using System.Diagnostics;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Emulation;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.Core.Session;
using Launcher.UI.Wpf.Services;
using Microsoft.Extensions.Logging;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Emulação (épico EMU, ADR-0024): abrir ROMs pelo emulador do sistema, sair do emulador pelo controle e as páginas de
/// Configurações › Emulação (pastas de ROMs, emuladores, emulador de cada sistema e de cada jogo).
/// </summary>
public sealed partial class ShellViewModel
{
    public const string EmulationOverline = "Emulação";

    /// <summary>Emulador do jogo aberto agora (para "sair do emulador" e o fim da sessão).</summary>
    private EmulatorLaunch? _runningEmulator;

    /// <summary>"Sair do emulador" em andamento (o atalho segurado de novo não começa outro).</summary>
    private bool _exitingEmulator;

    /// <summary>Emuladores, pastas de ROMs e a fonte "Emulação" da biblioteca.</summary>
    public EmulationService Emulation { get; }

    /// <summary>Um emulador está aberto pelo Tako e o atalho de sair pelo controle está ligado (EMU-006).</summary>
    public bool CanExitEmulatorFromController => Emulation.Config.ExitChord && _runningEmulator is not null && IsTrackingRealGame && !_exitingEmulator;

    // ---------------------------------------------------------------- Abrir (EMU-005/008)

    /// <summary>
    /// Antes da tela "Iniciando": sem emulador para o sistema, explica como configurar; faltando BIOS, avisa e deixa
    /// abrir mesmo assim. Verdadeiro = pode abrir agora.
    /// </summary>
    private bool ConfirmRomLaunch(GameItemViewModel item)
    {
        switch (Emulation.Resolve(item.Game))
        {
            case EmulatorUnavailable problem:
                ShowEmulationProblem(problem);
                return false;
            case EmulatorLaunch { MissingBios.Count: > 0 } launch when !_biosConfirmed.Remove(item.Id):
                DialogViewModel? dialog = null;
                dialog = new DialogViewModel(
                    $"{launch.Emulator.Name} pode precisar de BIOS",
                    "O Tako não baixa BIOS. Copie do seu console para a pasta indicada:\n" + string.Join('\n', launch.MissingBios.Select(b => "• " + b)),
                    [
                        new OverlayAction("Abrir mesmo assim", ActionStyle.Primary, new RelayCommand(() =>
                        {
                            CloseOverlay(dialog!);
                            _biosConfirmed.Add(item.Id);
                            Launch(item);
                        })),
                        new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: true),
                    ]);
                PushOverlay(dialog);
                return false;
            default:
                return true;
        }
    }

    private readonly HashSet<string> _biosConfirmed = new(StringComparer.Ordinal);

    /// <summary>Nenhum emulador serve: o motivo e um atalho para Configurações › Emulação.</summary>
    private void ShowEmulationProblem(EmulatorUnavailable problem)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(problem.Title, problem.Message,
        [
            new OverlayAction("Configurar emulação", ActionStyle.Primary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                OpenEmulationSettings();
            }), IsDefault: true),
            new OverlayAction("Fechar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))),
        ]);
        PushOverlay(dialog);
    }

    /// <summary>Como iniciar o jogo: o emulador com a linha de comando da ROM, ou o próprio jogo. Nulo = não dá (aviso já mostrado).</summary>
    private ProcessStartInfo? GameStartInfo(Game game, Core.Session.EffectiveSessionSettings effective)
    {
        _runningEmulator = null;
        if (!game.IsRom)
        {
            return new ProcessStartInfo(game.LaunchTarget!)
            {
                UseShellExecute = true,
                Arguments = effective.Arguments(game.LaunchArguments),
                WorkingDirectory = game.WorkingDirectory ?? string.Empty,
            };
        }

        if (Emulation.Resolve(game) is not EmulatorLaunch launch)
        {
            Toast($"Nenhum emulador abre {game.Title} agora · veja Configurações › Emulação");
            return null;
        }

        _runningEmulator = launch;
        LogEmulatorLaunch(_logger, game.Id, launch.Emulator.Id, launch.Option.Core?.Id);
        return new ProcessStartInfo(launch.Executable)
        {
            UseShellExecute = true,
            Arguments = effective.Arguments(launch.Arguments), // argumentos extras deste jogo (UX-007) no fim
            WorkingDirectory = launch.WorkingDirectory,
        };
    }

    /// <summary>ROM: o processo do emulador (e filhos); sem processo, qualquer programa da pasta do emulador.</summary>
    private TrackingPlan TrackingFor(Game game, int? root) =>
        _runningEmulator is { } emulator
            ? root is not null ? new TrackingPlan(GameTrackingStrategy.Process) : new TrackingPlan(GameTrackingStrategy.InstallFolder, emulator.WorkingDirectory)
            : GameTracking.Plan(game, root is not null);

    /// <summary>"Jogando X no RetroArch · segure View + Menu para sair" (nulo fora da emulação).</summary>
    private string? EmulatorStartedNote(Game game) => _runningEmulator is not { } emulator ? null
        : Emulation.Config.ExitChord
            ? $"Jogando {game.Title} no {emulator.Emulator.Name} · segure {ExitChordLabel} para sair"
            : $"Jogando {game.Title} no {emulator.Emulator.Name}";

    /// <summary>"View + Menu" ou "Create + Options", conforme o controle.</summary>
    public string ExitChordLabel => Core.Input.ControlCenterChord.Format(EmulatorExitSequence.Chord, IsPlayStationPad ? Core.Input.GamepadFamily.PlayStation5 : Core.Input.GamepadFamily.Xbox);

    // ---------------------------------------------------------------- Sair do emulador (EMU-006)

    /// <summary>
    /// Fecha o emulador com calma: pede para a janela fechar; se continuar aberto, aperta a tecla de sair dele; se
    /// ainda assim continuar, traz o Tako e pergunta antes de encerrar à força. O fim da sessão (tempo, telas, foco)
    /// segue o caminho normal quando o processo some.
    /// </summary>
    public async void ExitEmulator()
    {
        if (_runningEmulator is not { } emulator || _monitor?.Tracker is not { } tracker || _exitingEmulator)
        {
            return;
        }

        _exitingEmulator = true;
        try
        {
            var sequence = new EmulatorExitSequence(emulator.Emulator.Definition);
            int[] ids() => [.. (_monitor?.Tracker ?? tracker).GameProcessIds];
            LogEmulatorExit(_logger, emulator.Emulator.Id, sequence.Current);
            EmulatorWindows.Close(ids());
            Toast($"Fechando o {emulator.Emulator.Name}…");
            while (true)
            {
                await Task.Delay(EmulatorExitSequence.StepWait).ConfigureAwait(true);
                if (_runningEmulator != emulator)
                {
                    return; // a sessão já terminou
                }

                var step = sequence.Next(EmulatorWindows.AnyAlive(ids()));
                LogEmulatorExit(_logger, emulator.Emulator.Id, step);
                switch (step)
                {
                    case EmulatorExitStep.SendExitKey:
                        EmulatorWindows.PressKey(ids(), emulator.Emulator.Definition.ExitKey!, emulator.Emulator.Definition.ExitKeyPresses);
                        continue;
                    case EmulatorExitStep.AskToForceClose:
                        AskToForceCloseEmulator(emulator, ids());
                        return;
                    default:
                        return;
                }
            }
        }
        finally
        {
            _exitingEmulator = false;
        }
    }

    private void AskToForceCloseEmulator(EmulatorLaunch emulator, int[] processIds)
    {
        BringLauncherToFront();
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            $"O {emulator.Emulator.Name} não fechou",
            "Encerrar à força fecha o emulador agora; o que não foi salvo no jogo se perde.",
            [
                new OverlayAction("Encerrar o emulador", ActionStyle.Danger, new RelayCommand(() =>
                {
                    CloseOverlay(dialog!);
                    EmulatorWindows.Kill(processIds);
                    Toast($"{emulator.Emulator.Name} encerrado");
                })),
                new OverlayAction("Voltar ao jogo", ActionStyle.Secondary, new RelayCommand(() =>
                {
                    CloseOverlay(dialog!);
                    ResumeGame();
                }), IsDefault: true),
            ]);
        PushOverlay(dialog);
    }

    // ---------------------------------------------------------------- Configurações › Emulação

    /// <summary>"3 emuladores · 2 pastas", "Nenhum emulador encontrado".</summary>
    public string EmulationSummary
    {
        get
        {
            int emulators = Emulation.Inventory.Emulators.Count;
            int folders = Emulation.Config.RomFolders.Count;
            string found = emulators switch
            {
                0 => "Nenhum emulador encontrado",
                1 => "1 emulador",
                _ => $"{emulators} emuladores",
            };
            return folders switch
            {
                0 => found,
                1 => $"{found} · 1 pasta",
                _ => $"{found} · {folders} pastas",
            };
        }
    }

    /// <summary>Configurações já na categoria Emulação.</summary>
    public void OpenEmulationSettings()
    {
        OpenSettings();
        RequestFocus("settings:emu-setup");
    }

    [RelayCommand]
    public void StartEmulationSetup() => new EmulationSetupWizard(this).Start();

    /// <summary>Procura emuladores e ROMs de novo e resume o que achou.</summary>
    public void RescanEmulation()
    {
        Emulation.Redetect();
        Library.Refresh();
        var report = LibrarySources.Reports.GetValueOrDefault(GameStore.Emulation);
        Toast($"{EmulationSummary} · {RomCountText(report?.GameCount ?? 0)}{(report?.Notice is { } notice ? " · " + notice : string.Empty)}");
        NotifySettingsChanged();
    }

    public static string RomCountText(int count) => count switch
    {
        0 => "nenhum jogo de emulador",
        1 => "1 jogo de emulador",
        _ => $"{count} jogos de emulador",
    };

    public void ToggleEmulationFullscreen() => UpdateEmulation(c => c with { Fullscreen = !c.Fullscreen });

    public void ToggleEmulatorExitChord() => UpdateEmulation(c => c with { ExitChord = !c.ExitChord });

    public void ApplyRetroArchHotkeys()
    {
        Toast(Emulation.ApplyRetroArchHotkeys());
        NotifySettingsChanged();
    }

    /// <summary>BIOS que faltam nos emuladores em uso para os sistemas que têm jogos.</summary>
    public IReadOnlyList<string> MissingBios => Emulation.MissingBios(Library.Games.Where(g => g.IsRom).Select(g => g.Platform!).Distinct());

    public void ShowMissingBios()
    {
        var missing = MissingBios;
        ShowEmulationText("BIOS e firmware", missing.Count == 0
            ? "Nenhuma BIOS conhecida faltando nos emuladores em uso. O Tako não baixa BIOS: copie do seu console quando um emulador pedir."
            : "O Tako não baixa BIOS. Copie do seu console para a pasta indicada:\n" + string.Join('\n', missing.Select(m => "• " + m)));
    }

    private void UpdateEmulation(Func<EmulationConfig, EmulationConfig> change, bool rescan = false)
    {
        if (!Emulation.Update(change))
        {
            Toast("Não foi possível salvar as configurações de emulação; a mudança vale até fechar o Tako");
        }

        if (rescan)
        {
            Library.Refresh();
        }

        NotifySettingsChanged();
    }

    private SettingsDetailViewModel EmulationPage(string key, string title, string icon, Func<string?> description, Func<string?> value, IReadOnlyList<SettingRowViewModel> rows) =>
        new(this, new SettingRowViewModel($"emu:{key}", SettingKind.Action, title, description: description, value: value, icon: icon), rows, overline: EmulationOverline, routeKey: "settings");

    private void ShowEmulationPage(SettingsDetailViewModel page, bool replace = false) => Navigate(page, replace: replace);

    // ---------------------------------------------------------------- Pastas de ROMs (EMU-003)

    public void OpenRomFolders(bool replace = false)
    {
        var rows = new List<SettingRowViewModel>();
        var folders = Emulation.Config.RomFolders;
        for (int i = 0; i < folders.Count; i++)
        {
            var folder = folders[i];
            rows.Add(new SettingRowViewModel($"emu:folder:{i}", SettingKind.Action, folder.Path, new RelayCommand(() => FolderOptions(folder)),
                description: () => FolderDescription(folder),
                value: () => "Opções"));
        }

        rows.Add(new SettingRowViewModel("emu:folder:add", SettingKind.Action, "Adicionar pasta", new RelayCommand(AddRomFolder),
            description: () => "Uma pasta de um sistema, ou uma pasta com subpastas por sistema (snes, psx, ps2…, como no ES-DE)",
            value: () => "Escolher"));
        ShowEmulationPage(EmulationPage("folders", "Pastas de ROMs", "Icon.Folder",
            () => "Onde ficam os seus jogos de console. Só os nomes dos arquivos são lidos; nada é copiado nem baixado.",
            () => folders.Count == 1 ? "1 pasta" : $"{folders.Count} pastas", rows), replace);
    }

    private string FolderDescription(RomFolder folder)
    {
        if (folder.System is { } system)
        {
            return $"Pasta de {EmulationService.SystemName(system)}";
        }

        var systems = Emulation.SystemsInFolder(folder.Path);
        return systems.Count == 0 ? "Nenhuma subpasta de sistema reconhecida" : "Subpastas: " + string.Join(", ", systems.Select(EmulationService.SystemName));
    }

    /// <summary>Escolhe uma pasta (seletor do Windows) e pergunta se é de um sistema ou tem subpastas por sistema.</summary>
    public void AddRomFolder()
    {
        var dialog = new OpenFolderDialog { Title = "Pasta de ROMs" };
        if (dialog.ShowDialog(System.Windows.Application.Current?.MainWindow) == true)
        {
            AddRomFolder(dialog.FolderName);
        }
    }

    /// <summary>Exposto ao teste de fumaça: pasta já escolhida.</summary>
    internal void AddRomFolder(string path)
    {
        if (Emulation.Config.RomFolders.Any(f => WinPath.SamePath(f.Path, path)))
        {
            Toast("Esta pasta já está na lista");
            return;
        }

        var systems = Emulation.SystemsInFolder(path);
        void Add(string? system)
        {
            UpdateEmulation(c => c with { RomFolders = [.. c.RomFolders, new RomFolder(path, system)] }, rescan: true);
            var report = LibrarySources.Reports.GetValueOrDefault(GameStore.Emulation);
            Toast($"Pasta adicionada · {RomCountText(report?.GameCount ?? 0)}");
            if (CurrentPage is SettingsDetailViewModel { Feature.Key: "emu:folders" })
            {
                OpenRomFolders(replace: true);
            }
        }

        if (systems.Count > 0 && EmulationService.Catalog.SystemForFolder(WinPath.FileName(path)) is null)
        {
            DialogViewModel? dialog = null;
            dialog = new DialogViewModel("Como ler esta pasta?", $"{path}\nSubpastas reconhecidas: {string.Join(", ", systems.Select(EmulationService.SystemName))}.",
            [
                new OverlayAction("Usar as subpastas", ActionStyle.Primary, new RelayCommand(() => { CloseOverlay(dialog!); Add(null); }), IsDefault: true),
                new OverlayAction("É de um sistema só", ActionStyle.Secondary, new RelayCommand(() => { CloseOverlay(dialog!); ChooseSystem($"De qual sistema são os jogos de {WinPath.FileName(path)}?", ScreenScraperSystems.All, s => Add(s.Key)); })),
                new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))),
            ]);
            PushOverlay(dialog);
            return;
        }

        if (EmulationService.Catalog.SystemForFolder(WinPath.FileName(path)) is { } named)
        {
            Add(named); // a própria pasta tem nome de sistema ("snes", "psx")
            return;
        }

        ChooseSystem($"De qual sistema são os jogos de {WinPath.FileName(path)}?", ScreenScraperSystems.All, s => Add(s.Key));
    }

    private void FolderOptions(RomFolder folder)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(folder.Path, FolderDescription(folder),
        [
            new OverlayAction("Remover da lista", ActionStyle.Danger, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                UpdateEmulation(c => c with { RomFolders = [.. c.RomFolders.Where(f => f != folder)] }, rescan: true);
                Toast("Pasta removida · os arquivos continuam no disco");
                OpenRomFolders(replace: true);
            })),
            new OverlayAction("Fechar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: true),
        ]);
        PushOverlay(dialog);
    }

    private void ChooseSystem(string title, IReadOnlyList<EmulatedSystem> systems, Action<EmulatedSystem> chosen)
    {
        DialogViewModel? dialog = null;
        var actions = systems
            .Select((system, i) => new OverlayAction(system.Name, i == 0 ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                chosen(system);
            }), IsDefault: i == 0))
            .Append(new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))))
            .ToList();
        dialog = new DialogViewModel(title, "Escolha o sistema.", actions);
        PushOverlay(dialog);
    }

    // ---------------------------------------------------------------- Emuladores (EMU-001/002/004)

    public void OpenEmulators(bool replace = false)
    {
        var rows = Emulation.Inventory.Emulators.Select(emulator => new SettingRowViewModel(
            $"emu:emulator:{emulator.Id}", SettingKind.Action, emulator.Name, new RelayCommand(() => OpenEmulator(emulator.Id)),
            description: () => EmulatorDescription(emulator),
            value: () => emulator.Systems.Count == 1 ? EmulationService.SystemName(emulator.Systems[0]) : $"{emulator.Systems.Count} sistemas")).ToList();
        rows.Add(new SettingRowViewModel("emu:locate", SettingKind.Action, "Indicar onde está um emulador", new RelayCommand(LocateEmulator),
            description: () => "Para um emulador da lista que o Tako não achou sozinho",
            value: () => "Escolher"));
        rows.Add(new SettingRowViewModel("emu:custom", SettingKind.Action, "Cadastrar outro emulador", new RelayCommand(AddCustomEmulator),
            description: () => "Qualquer programa que abra uma ROM pela linha de comando",
            value: () => "Escolher"));
        ShowEmulationPage(EmulationPage("emulators", "Emuladores", "Icon.Gamepad",
            () => "Achados nas pastas padrão, no Scoop, no winget e nos aplicativos instalados do Windows.",
            () => EmulationSummary, rows), replace);
    }

    private static string EmulatorDescription(InstalledEmulator emulator)
    {
        string origin = emulator.Origin switch
        {
            EmulatorOrigin.Chosen => "Escolhido por você",
            EmulatorOrigin.Scoop => "Instalado pelo Scoop",
            EmulatorOrigin.Winget => "Instalado pelo winget",
            EmulatorOrigin.Registry => "Aplicativo instalado",
            EmulatorOrigin.Custom => "Cadastrado por você",
            _ => "Pasta padrão",
        };
        string cores = emulator.UsesCores ? $" · cores: {(emulator.Cores.Count == 0 ? "nenhum" : string.Join(", ", emulator.Cores.Select(c => c.Name)))}" : string.Empty;
        return $"{origin} · {emulator.ExePath}{cores}";
    }

    public void OpenEmulator(string id, bool replace = false)
    {
        if (Emulation.Inventory.Find(id) is not { } emulator)
        {
            Toast("Este emulador não foi encontrado; procure de novo");
            return;
        }

        bool custom = emulator.Origin == EmulatorOrigin.Custom;
        var rows = new List<SettingRowViewModel>
        {
            new($"emu:{id}:exe", SettingKind.Action, "Executável", new RelayCommand(() => ChooseEmulatorExe(emulator)),
                description: () => emulator.ExePath,
                value: () => "Trocar"),
            new($"emu:{id}:args", SettingKind.Action, "Argumentos", new RelayCommand(() => EditEmulatorArguments(emulator)),
                description: () => CurrentArguments(emulator),
                value: () => Emulation.Config.EmulatorArguments.ContainsKey(id) ? "Seus" : "Padrão"),
            new($"emu:{id}:systems", SettingKind.Info, "Sistemas", new RelayCommand(() => ShowEmulationText(emulator.Name, string.Join(", ", emulator.Systems.Select(EmulationService.SystemName)))),
                description: () => emulator.Systems.Count == 0 ? (emulator.UsesCores ? "Nenhum core instalado: instale cores pelo menu do RetroArch" : "Nenhum") : string.Join(", ", emulator.Systems.Select(EmulationService.SystemName)),
                value: () => emulator.Systems.Count.ToString(System.Globalization.CultureInfo.InvariantCulture)),
        };
        if (custom)
        {
            rows.Add(new($"emu:{id}:edit-systems", SettingKind.Action, "Escolher sistemas", new RelayCommand(() => OpenCustomSystems(id)),
                description: () => "Quais sistemas este emulador abre",
                value: () => "Escolher"));
            rows.Add(new($"emu:{id}:remove", SettingKind.Action, "Remover este emulador", new RelayCommand(() => RemoveCustomEmulator(id)),
                description: () => "Sai só da lista do Tako; nada é apagado do disco",
                value: () => "Remover"));
        }
        else if (emulator.Origin == EmulatorOrigin.Chosen)
        {
            rows.Add(new($"emu:{id}:forget", SettingKind.Action, "Voltar a procurar sozinho", new RelayCommand(() =>
                {
                    UpdateEmulation(c => c.WithEmulatorPath(id, null), rescan: true);
                    OpenEmulators(replace: true);
                }),
                description: () => "Esquece o executável escolhido e procura nas pastas padrão",
                value: () => "Esquecer"));
        }

        ShowEmulationPage(EmulationPage($"emulator-page:{id}", emulator.Name, "Icon.Gamepad", () => EmulatorDescription(emulator), () => null, rows), replace);
    }

    private string CurrentArguments(InstalledEmulator emulator) =>
        Emulation.Config.EmulatorArguments.GetValueOrDefault(emulator.Id) ?? emulator.Definition.Arguments;

    private void ChooseEmulatorExe(InstalledEmulator emulator)
    {
        if (PickExe($"Executável do {emulator.Name}") is not { } exe)
        {
            return;
        }

        if (emulator.Origin == EmulatorOrigin.Custom)
        {
            UpdateEmulation(c => c with { CustomEmulators = [.. c.CustomEmulators.Select(e => e.Id == emulator.Id ? e with { ExePath = exe } : e)] }, rescan: true);
        }
        else
        {
            UpdateEmulation(c => c.WithEmulatorPath(emulator.Id, exe), rescan: true);
        }

        Toast($"{emulator.Name}: {exe}");
        OpenEmulator(emulator.Id, replace: true);
    }

    private void LocateEmulator()
    {
        var missing = EmulationService.Catalog.Emulators.Where(d => Emulation.Inventory.Find(d.Id) is null).ToList();
        if (missing.Count == 0)
        {
            Toast("Todos os emuladores conhecidos já foram encontrados");
            return;
        }

        DialogViewModel? dialog = null;
        var actions = missing.Select((definition, i) => new OverlayAction(definition.Name, i == 0 ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                if (PickExe($"Executável do {definition.Name} ({string.Join(", ", definition.Executables)})") is { } exe)
                {
                    UpdateEmulation(c => c.WithEmulatorPath(definition.Id, exe), rescan: true);
                    Toast(Emulation.Inventory.Find(definition.Id) is null ? $"{definition.Name} não foi reconhecido nesse arquivo" : $"{definition.Name} encontrado");
                    OpenEmulators(replace: true);
                }
            }), IsDefault: i == 0))
            .Append(new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))))
            .ToList();
        dialog = new DialogViewModel("Qual emulador?", "Depois escolha o executável dele.", actions);
        PushOverlay(dialog);
    }

    private void AddCustomEmulator()
    {
        if (PickExe("Emulador personalizado") is { } exe)
        {
            AddCustomEmulator(exe, []);
        }
    }

    /// <summary>Exposto ao teste de fumaça: cadastra o executável e abre a escolha de sistemas.</summary>
    internal CustomEmulator AddCustomEmulator(string exe, IReadOnlyList<string> systems)
    {
        var custom = CustomEmulator.Create(exe, systems);
        UpdateEmulation(c => c with { CustomEmulators = [.. c.CustomEmulators, custom] }, rescan: true);
        Toast($"{custom.Name} cadastrado · escolha os sistemas que ele abre");
        OpenCustomSystems(custom.Id);
        return custom;
    }

    private void OpenCustomSystems(string id, bool replace = false)
    {
        var rows = ScreenScraperSystems.All.Select(system => new SettingRowViewModel(
            $"emu:{id}:system:{system.Key}", SettingKind.Toggle, system.Name, new RelayCommand(() => ToggleCustomSystem(id, system.Key)),
            isOn: () => Emulation.Config.CustomEmulators.FirstOrDefault(c => c.Id == id)?.Systems.Contains(system.Key) == true)).ToList();
        var custom = Emulation.Config.CustomEmulators.FirstOrDefault(c => c.Id == id);
        ShowEmulationPage(EmulationPage($"custom-systems:{id}", custom?.Name ?? "Emulador", "Icon.Gamepad",
            () => "Marque os sistemas que este emulador abre. Os argumentos ficam na página do emulador (padrão: só o arquivo do jogo).",
            () => $"{Emulation.Config.CustomEmulators.FirstOrDefault(c => c.Id == id)?.Systems.Count ?? 0} sistemas", rows), replace);
    }

    private void ToggleCustomSystem(string id, string system) => UpdateEmulation(c => c with
    {
        CustomEmulators = [.. c.CustomEmulators.Select(e => e.Id != id ? e : e with
        {
            Systems = e.Systems.Contains(system) ? [.. e.Systems.Where(s => s != system)] : [.. e.Systems, system],
        })],
    }, rescan: true);

    private void RemoveCustomEmulator(string id)
    {
        UpdateEmulation(c => c with
        {
            CustomEmulators = [.. c.CustomEmulators.Where(e => e.Id != id)],
            SystemChoices = c.SystemChoices.Where(p => p.Value.EmulatorId != id).ToDictionary(p => p.Key, p => p.Value),
        }, rescan: true);
        Toast("Emulador removido da lista");
        OpenEmulators(replace: true);
    }

    private void EditEmulatorArguments(InstalledEmulator emulator) => EditArguments(
        $"Argumentos do {emulator.Name}",
        "Valem para todos os jogos abertos por este emulador. O Tako põe aspas nos caminhos sozinho.",
        CurrentArguments(emulator),
        emulator.Definition.Arguments,
        emulator,
        text =>
        {
            if (emulator.Origin == EmulatorOrigin.Custom)
            {
                UpdateEmulation(c => c with { CustomEmulators = [.. c.CustomEmulators.Select(e => e.Id == emulator.Id ? e with { Arguments = text ?? "{rom}" } : e)] }, rescan: true);
            }
            else
            {
                UpdateEmulation(c => c.WithEmulatorArguments(emulator.Id, text));
            }

            Toast(text is null ? $"{emulator.Name}: argumentos padrão" : $"{emulator.Name}: argumentos salvos");
        });

    /// <summary>Abre o editor (EMU-004) com a prévia da linha de comando para uma ROM de exemplo.</summary>
    private void EditArguments(string title, string description, string current, string defaultText, InstalledEmulator? emulator, Action<string?> save)
    {
        ArgumentsEditorViewModel? editor = null;
        string Preview(string text)
        {
            string sample = @"D:\ROMs\Jogo de exemplo (USA).sfc";
            var core = emulator is { Cores.Count: > 0 } ? emulator.Cores[0] : null;
            var values = LaunchArguments.For(sample, core?.CorePath, null, Emulation.Config.Fullscreen ? emulator?.Definition.Fullscreen ?? string.Empty : string.Empty);
            string exe = emulator is null ? "emulador.exe" : WinPath.FileName(emulator.ExePath);
            return $"{exe} {LaunchArguments.Expand(text, values)}";
        }

        editor = new ArgumentsEditorViewModel(title, description, current, defaultText, Preview, ReadClipboard,
            text =>
            {
                CloseOverlay(editor!);
                save(text);
            },
            () => CloseOverlay(editor!));
        PushOverlay(editor);
    }

    private static string? PickExe(string title)
    {
        var dialog = new OpenFileDialog { Title = title, Filter = "Programas|*.exe|Todos os arquivos|*.*", CheckFileExists = true };
        return dialog.ShowDialog(System.Windows.Application.Current?.MainWindow) == true ? dialog.FileName : null;
    }

    // ---------------------------------------------------------------- Emulador de cada sistema (EMU-005)

    /// <summary>Sistemas que têm jogos na biblioteca ou um emulador, na ordem da tabela de sistemas.</summary>
    public IReadOnlyList<string> EmulationSystems
    {
        get
        {
            var keys = Library.Games.Where(g => g.IsRom).Select(g => g.Platform!).Concat(Emulation.Inventory.CoveredSystems).ToHashSet(StringComparer.OrdinalIgnoreCase);
            return [.. ScreenScraperSystems.All.Select(s => s.Key).Where(keys.Contains)];
        }
    }

    public void OpenEmulationSystems(bool replace = false)
    {
        var rows = EmulationSystems.Select(system => new SettingRowViewModel(
            $"emu:system:{system}", SettingKind.Action, EmulationService.SystemName(system), new RelayCommand(() => ChooseSystemEmulator(system)),
            description: () => SystemDescription(system),
            value: () => Emulation.CurrentFor(system)?.Label ?? "Nenhum emulador")).ToList();
        if (rows.Count == 0)
        {
            rows.Add(new SettingRowViewModel("emu:system:none", SettingKind.Info, "Nenhum sistema ainda", new RelayCommand(StartEmulationSetup),
                description: () => "Instale um emulador ou adicione uma pasta de ROMs; o assistente ajuda",
                value: () => "Configurar"));
        }

        ShowEmulationPage(EmulationPage("systems", "Emulador de cada sistema", "Icon.Grid",
            () => "O primeiro da lista é sugerido; troque quando quiser. Cada jogo também pode ter o seu (Opções do jogo).",
            () => $"{EmulationSystems.Count} sistemas", rows), replace);
    }

    private string SystemDescription(string system)
    {
        int games = Library.Games.Count(g => string.Equals(g.Platform, system, StringComparison.OrdinalIgnoreCase));
        string count = games switch { 0 => "Nenhum jogo", 1 => "1 jogo", _ => $"{games} jogos" };
        if (Emulation.CurrentFor(system) is null && ScreenScraperSystems.ForKey(system) is { } known)
        {
            return $"{count} · {EmulatorResolver.NoEmulatorMessage(known, EmulationService.Catalog)}";
        }

        var bios = Emulation.MissingBios([system]);
        return bios.Count == 0 ? count : $"{count} · falta BIOS: {bios[0]}";
    }

    public void ChooseSystemEmulator(string system)
    {
        var options = Emulation.Inventory.OptionsFor(system);
        var current = Emulation.CurrentFor(system);
        DialogViewModel? dialog = null;
        var actions = options.Select(option => new OverlayAction(
                option.Label + (option == current ? " ✓" : string.Empty),
                option == current ? ActionStyle.Primary : ActionStyle.Secondary,
                new RelayCommand(() =>
                {
                    CloseOverlay(dialog!);
                    UpdateEmulation(c => c.WithSystemChoice(system, option.Choice));
                    Toast($"{EmulationService.SystemName(system)}: {option.Label}");
                }),
                IsDefault: option == current))
            .ToList();
        actions.Add(new OverlayAction("Argumentos extras do sistema", ActionStyle.Secondary, new RelayCommand(() =>
        {
            CloseOverlay(dialog!);
            EditArguments(
                $"Argumentos extras de {EmulationService.SystemName(system)}",
                "Entram depois dos argumentos do emulador em todos os jogos deste sistema.",
                Emulation.Config.SystemArguments.GetValueOrDefault(system) ?? string.Empty,
                string.Empty,
                current?.Emulator,
                text => UpdateEmulation(c => c with { SystemArguments = Set(c.SystemArguments, system, text) }));
        })));
        actions.Add(new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: options.Count == 0));
        string text = options.Count == 0 && ScreenScraperSystems.ForKey(system) is { } known
            ? EmulatorResolver.NoEmulatorMessage(known, EmulationService.Catalog)
            : "Com qual emulador abrir os jogos deste sistema?";
        dialog = new DialogViewModel(EmulationService.SystemName(system), text, actions);
        PushOverlay(dialog);
    }

    private static Dictionary<string, string> Set(IReadOnlyDictionary<string, string> source, string key, string? value)
    {
        var copy = new Dictionary<string, string>(source, StringComparer.OrdinalIgnoreCase);
        if (string.IsNullOrWhiteSpace(value))
        {
            copy.Remove(key);
        }
        else
        {
            copy[key] = value;
        }

        return copy;
    }

    // ---------------------------------------------------------------- Por jogo (EMU-004/005)

    /// <summary>Ações de emulação no "Opções" do jogo (ROMs).</summary>
    private IEnumerable<OverlayAction> GameEmulationActions(GameItemViewModel item, Action close)
    {
        if (!item.Game.IsRom)
        {
            yield break;
        }

        yield return new OverlayAction("Emulador deste jogo", ActionStyle.Secondary, new RelayCommand(() => { close(); ChooseGameEmulator(item); }));
        yield return new OverlayAction("Argumentos deste jogo", ActionStyle.Secondary, new RelayCommand(() => { close(); EditGameArguments(item); }));
        if (Emulation.Resolve(item.Game) is EmulatorLaunch { Emulator.Definition.ConfigArgument.Length: > 0 })
        {
            yield return new OverlayAction("Configuração do RetroArch deste jogo", ActionStyle.Secondary, new RelayCommand(() => { close(); ChooseGameConfig(item); }));
        }
    }

    public void ChooseGameEmulator(GameItemViewModel item)
    {
        var game = item.Game;
        var options = Emulation.OptionsFor(game);
        var own = Emulation.Config.Games.GetValueOrDefault(game.Id) ?? new GameEmulation();
        var chosen = EmulatorResolver.Pick(options, own.Choice);
        DialogViewModel? dialog = null;
        var actions = new List<OverlayAction>
        {
            new($"Usar o do sistema ({(game.Platform is { } p ? Emulation.CurrentFor(p)?.Label ?? "nenhum" : "nenhum")}){(chosen is null ? " ✓" : string.Empty)}", chosen is null ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(() =>
            {
                CloseOverlay(dialog!);
                UpdateEmulation(c => c.WithGame(game.Id, own with { Choice = null }));
                RefreshDetails();
            }), IsDefault: chosen is null),
        };
        actions.AddRange(options.Select(option => new OverlayAction(option.Label + (option == chosen ? " ✓" : string.Empty), option == chosen ? ActionStyle.Primary : ActionStyle.Secondary, new RelayCommand(() =>
        {
            CloseOverlay(dialog!);
            UpdateEmulation(c => c.WithGame(game.Id, own with { Choice = option.Choice }));
            Toast($"{game.Title}: {option.Label}");
            RefreshDetails();
        }), IsDefault: option == chosen)));
        actions.Add(new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))));
        dialog = new DialogViewModel($"Emulador de {game.Title}", options.Count == 0 ? "Nenhum emulador instalado abre este sistema." : "Só para este jogo; os outros seguem o do sistema.", actions);
        PushOverlay(dialog);
    }

    private void EditGameArguments(GameItemViewModel item)
    {
        var game = item.Game;
        var own = Emulation.Config.Games.GetValueOrDefault(game.Id) ?? new GameEmulation();
        var launch = Emulation.Resolve(game) as EmulatorLaunch;
        string systemDefault = launch is null ? "{rom}" : CurrentArguments(launch.Emulator);
        EditArguments(
            $"Argumentos de {game.Title}",
            "Só para este jogo, no lugar dos argumentos do emulador. \"Padrão\" volta aos do emulador.",
            own.Arguments ?? systemDefault,
            systemDefault,
            launch?.Emulator,
            text =>
            {
                UpdateEmulation(c => c.WithGame(game.Id, own with { Arguments = text }));
                Toast(text is null ? $"{game.Title}: argumentos do emulador" : $"{game.Title}: argumentos salvos");
            });
    }

    private void ChooseGameConfig(GameItemViewModel item)
    {
        var game = item.Game;
        var own = Emulation.Config.Games.GetValueOrDefault(game.Id) ?? new GameEmulation();
        var dialog = new OpenFileDialog { Title = $"Configuração do RetroArch para {game.Title} (.cfg)", Filter = "Configurações do RetroArch|*.cfg|Todos os arquivos|*.*", CheckFileExists = true };
        if (dialog.ShowDialog(System.Windows.Application.Current?.MainWindow) == true)
        {
            UpdateEmulation(c => c.WithGame(game.Id, own with { ConfigFile = dialog.FileName }));
            Toast($"{game.Title}: o RetroArch soma {System.IO.Path.GetFileName(dialog.FileName)} à configuração");
        }
        else if (own.ConfigFile is not null)
        {
            UpdateEmulation(c => c.WithGame(game.Id, own with { ConfigFile = null }));
            Toast($"{game.Title}: sem configuração própria do RetroArch");
        }
    }

    private void RefreshDetails()
    {
        if (CurrentPage is DetailsViewModel details)
        {
            details.Refresh();
        }
    }

    private void ShowEmulationText(string title, string text)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(title, text, [new OverlayAction("Fechar", ActionStyle.Primary, new RelayCommand(() => CloseOverlay(dialog!)), IsDefault: true)]);
        PushOverlay(dialog);
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "ROM {GameId} aberta no emulador {Emulator} (core {Core})")]
    private static partial void LogEmulatorLaunch(ILogger logger, string gameId, string emulator, string? core);

    [LoggerMessage(Level = LogLevel.Information, Message = "Saindo do emulador {Emulator}: {Step}")]
    private static partial void LogEmulatorExit(ILogger logger, string emulator, EmulatorExitStep step);
}

/// <summary>Janelas e processos do emulador (EMU-006): fechar, apertar a tecla de sair e encerrar.</summary>
internal static partial class EmulatorWindows
{
    private const uint WmKeyDown = 0x0100;
    private const uint WmKeyUp = 0x0101;

    /// <summary>WM_CLOSE na janela principal de cada processo, como o X da janela.</summary>
    public static void Close(IEnumerable<int> processIds)
    {
        foreach (int id in processIds)
        {
            try
            {
                using var process = Process.GetProcessById(id);
                process.CloseMainWindow();
            }
            catch (Exception ex) when (ex is ArgumentException or InvalidOperationException or Win32Exception)
            {
                // Já fechou ou não pode ser lido.
            }
        }
    }

    public static bool AnyAlive(IEnumerable<int> processIds)
    {
        foreach (int id in processIds)
        {
            try
            {
                using var process = Process.GetProcessById(id);
                if (!process.HasExited)
                {
                    return true;
                }
            }
            catch (Exception ex) when (ex is ArgumentException or InvalidOperationException or Win32Exception)
            {
                // Esse já saiu.
            }
        }

        return false;
    }

    /// <summary>Manda a tecla (nome de <see cref="System.Windows.Input.Key"/>, ex.: "Escape") para a janela principal.</summary>
    public static void PressKey(IEnumerable<int> processIds, string key, int presses)
    {
        if (!Enum.TryParse(key, ignoreCase: true, out System.Windows.Input.Key parsed))
        {
            return;
        }

        uint virtualKey = (uint)System.Windows.Input.KeyInterop.VirtualKeyFromKey(parsed);
        foreach (int id in processIds)
        {
            try
            {
                using var process = Process.GetProcessById(id);
                nint window = process.MainWindowHandle;
                if (window == 0)
                {
                    continue;
                }

                for (int i = 0; i < Math.Max(1, presses); i++)
                {
                    PostMessageW(window, WmKeyDown, (nint)virtualKey, 1);
                    PostMessageW(window, WmKeyUp, (nint)virtualKey, unchecked((nint)0xC0000001));
                }
            }
            catch (Exception ex) when (ex is ArgumentException or InvalidOperationException or Win32Exception)
            {
                // Já fechou.
            }
        }
    }

    public static void Kill(IEnumerable<int> processIds)
    {
        foreach (int id in processIds)
        {
            try
            {
                using var process = Process.GetProcessById(id);
                process.Kill(entireProcessTree: true);
            }
            catch (Exception ex) when (ex is ArgumentException or InvalidOperationException or Win32Exception or NotSupportedException)
            {
                // Já fechou ou não pode ser encerrado.
            }
        }
    }

    [System.Runtime.InteropServices.LibraryImport("user32.dll")]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static partial bool PostMessageW(nint window, uint message, nint wParam, nint lParam);
}
