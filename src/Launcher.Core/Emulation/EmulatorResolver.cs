using Launcher.Core.Library;
using Launcher.Core.Library.Metadata.ScreenScraper;

namespace Launcher.Core.Emulation;

/// <summary>Resultado de <see cref="EmulatorResolver.Resolve"/>: como abrir a ROM, ou por que não dá.</summary>
public abstract record EmulatorResolution;

/// <summary>Linha de comando pronta (EMU-005/008). <paramref name="MissingBios"/> = avisos de BIOS que faltam (o jogo pode não abrir).</summary>
public sealed record EmulatorLaunch(EmulatorOption Option, string Executable, string Arguments, string WorkingDirectory, IReadOnlyList<string> MissingBios) : EmulatorResolution
{
    public InstalledEmulator Emulator => Option.Emulator;
}

/// <summary>Nenhum emulador serve: título e explicação de como configurar, em linguagem de usuário.</summary>
public sealed record EmulatorUnavailable(string Title, string Message) : EmulatorResolution;

/// <summary>
/// Decide com que emulador abrir uma ROM (ADR-0024): a escolha do jogo, senão a do sistema, senão a primeira opção
/// sugerida; um emulador escolhido que sumiu cai para a próxima opção. Monta os argumentos e confere as BIOS. Puro.
/// </summary>
public static class EmulatorResolver
{
    public static EmulatorResolution Resolve(Game game, EmulationConfig config, EmulatorInventory inventory, IEmulationFileSystem disk, EmulationPaths paths)
    {
        ArgumentNullException.ThrowIfNull(game);
        ArgumentNullException.ThrowIfNull(config);
        ArgumentNullException.ThrowIfNull(inventory);
        string? rom = game.RomPath ?? game.LaunchTarget;
        var system = ScreenScraperSystems.ForKey(game.Platform);
        if (rom is null || system is null)
        {
            return new EmulatorUnavailable("Sistema desconhecido", $"O Tako não sabe de que sistema é {game.Title}. Use Opções › Editar para escolher o sistema.");
        }

        if (!disk.FileExists(rom) && !disk.DirectoryExists(rom))
        {
            return new EmulatorUnavailable("Arquivo não encontrado", $"{rom} não existe mais. Confira se o disco está conectado ou procure as ROMs de novo em Configurações › Emulação.");
        }

        var options = inventory.OptionsFor(system.Key, WinPath.Extension(rom));
        if (options.Count == 0)
        {
            return new EmulatorUnavailable($"Nenhum emulador para {system.Name}", NoEmulatorMessage(system, inventory.Catalog));
        }

        var perGame = config.Games.GetValueOrDefault(game.Id);
        var option = Pick(options, perGame?.Choice) ?? Pick(options, config.SystemChoices.GetValueOrDefault(system.Key)) ?? options[0];
        var definition = option.Emulator.Definition;
        string template = !string.IsNullOrWhiteSpace(perGame?.Arguments) ? perGame.Arguments
            : config.EmulatorArguments.GetValueOrDefault(definition.Id) is { Length: > 0 } own ? own
            : definition.Arguments;
        if (config.SystemArguments.GetValueOrDefault(system.Key) is { Length: > 0 } extra)
        {
            template += " " + extra;
        }

        string? configFile = string.IsNullOrWhiteSpace(perGame?.ConfigFile) ? null : perGame.ConfigFile;
        if (configFile is not null && definition.ConfigArgument.Length > 0 && !template.Contains(LaunchArguments.Config, StringComparison.OrdinalIgnoreCase))
        {
            template += " " + definition.ConfigArgument;
        }

        var values = LaunchArguments.For(rom, option.Core?.CorePath, configFile, config.Fullscreen ? definition.Fullscreen : string.Empty);
        string arguments = LaunchArguments.Expand(template, values);
        var bios = BiosCheck.Missing(option, system.Key, disk, paths, inventory.Catalog);
        return new EmulatorLaunch(option, option.Emulator.ExePath, arguments, option.Emulator.Folder, bios);
    }

    /// <summary>A opção escolhida, se ainda existe; core que sumiu = outro core do mesmo emulador.</summary>
    public static EmulatorOption? Pick(IReadOnlyList<EmulatorOption> options, EmulatorChoice? choice)
    {
        ArgumentNullException.ThrowIfNull(options);
        if (choice is null)
        {
            return null;
        }

        return options.FirstOrDefault(o => string.Equals(o.Emulator.Id, choice.EmulatorId, StringComparison.OrdinalIgnoreCase) && string.Equals(o.Core?.Id, choice.Core, StringComparison.OrdinalIgnoreCase))
            ?? options.FirstOrDefault(o => o.Matches(choice))
            ?? options.FirstOrDefault(o => string.Equals(o.Emulator.Id, choice.EmulatorId, StringComparison.OrdinalIgnoreCase));
    }

    /// <summary>"Nenhum emulador instalado abre jogos de PS2. Instale o PCSX2 ou o RetroArch (core LRPS2)…".</summary>
    public static string NoEmulatorMessage(EmulatedSystem system, EmulatorCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(system);
        ArgumentNullException.ThrowIfNull(catalog);
        var names = catalog.Emulators.Where(e => !e.UsesCores && e.Systems.Contains(system.Key, StringComparer.OrdinalIgnoreCase)).Select(e => e.Name).ToList();
        var core = catalog.RetroArchCores.FirstOrDefault(c => c.Systems.Contains(system.Key, StringComparer.OrdinalIgnoreCase));
        if (core is not null)
        {
            names.Add($"RetroArch com o core {core.Name}");
        }

        string suggestion = names.Count switch
        {
            0 => "um emulador",
            1 => names[0],
            _ => string.Join(", ", names.Take(names.Count - 1)) + " ou " + names[^1],
        };
        return $"Nenhum emulador instalado abre jogos de {system.Name}. Instale {suggestion} e use \"Procurar emuladores\" em Configurações › Emulação, ou cadastre o seu emulador lá.";
    }
}

/// <summary>
/// BIOS e firmware conhecidos (EMU-008): o Tako nunca baixa; só confere se estão onde o emulador procura e diz onde
/// colocar. RetroArch pelo <c>.info</c> do core (arquivos obrigatórios na pasta <c>system</c>); os outros pela tabela.
/// </summary>
public static class BiosCheck
{
    public static IReadOnlyList<string> Missing(EmulatorOption option, string system, IEmulationFileSystem disk, EmulationPaths paths, EmulatorCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(option);
        ArgumentNullException.ThrowIfNull(disk);
        ArgumentNullException.ThrowIfNull(paths);
        ArgumentNullException.ThrowIfNull(catalog);
        var missing = new List<string>();
        if (option.Core is { } core)
        {
            string? cfg = RetroArch.ConfigFile(disk, option.Emulator.Folder, paths);
            var settings = cfg is null ? null : disk.ReadText(cfg) is { } text ? RetroArch.ParseKeyValues(text) : null;
            string folder = RetroArch.SystemFolder(option.Emulator.Folder, settings);
            foreach (var firmware in core.Firmware.Where(f => !f.Optional))
            {
                string file = WinPath.Resolve(folder, firmware.Path);
                if (!disk.FileExists(file))
                {
                    missing.Add($"{firmware.Description}: coloque {WinPath.FileName(file)} em {WinPath.Directory(file)}");
                }
            }

            return missing;
        }

        foreach (var requirement in catalog.Bios.Where(b => string.Equals(b.Emulator, option.Emulator.Id, StringComparison.OrdinalIgnoreCase) && string.Equals(b.System, system, StringComparison.OrdinalIgnoreCase)))
        {
            var folders = requirement.Folders.Select(f => paths.Expand(f, option.Emulator.Folder)).OfType<string>().ToList();
            bool found = folders.Any(folder => disk.Files(folder).Any(file => Matches(file, requirement.Patterns)));
            if (!found && folders.Count > 0)
            {
                string where = folders.FirstOrDefault(disk.DirectoryExists) ?? folders[0];
                missing.Add($"{requirement.Label}: coloque em {where}");
            }
        }

        return missing;
    }

    private static bool Matches(string file, IReadOnlyList<string> patterns) => patterns.Any(p => p.StartsWith('.')
        ? string.Equals(WinPath.Extension(file), p, StringComparison.OrdinalIgnoreCase)
        : string.Equals(WinPath.FileName(file), p, StringComparison.OrdinalIgnoreCase));
}
