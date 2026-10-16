using Launcher.Core.Library.Sources;

namespace Launcher.Core.Emulation;

/// <summary>Onde o emulador foi achado (mostrado nas Configurações).</summary>
public enum EmulatorOrigin
{
    /// <summary>Executável escolhido pelo usuário.</summary>
    Chosen,

    /// <summary>Pasta padrão do emulador.</summary>
    Folder,

    Scoop,

    Winget,

    /// <summary>"Aplicativos instalados" do Windows (registro Uninstall).</summary>
    Registry,

    /// <summary>Emulador personalizado (EMU-002).</summary>
    Custom,
}

/// <summary>Emulador encontrado neste PC (ou cadastrado), pronto para abrir jogos.</summary>
/// <param name="Systems">Sistemas atendidos; no RetroArch, a união dos cores instalados.</param>
/// <param name="Cores">Cores instalados (só RetroArch).</param>
public sealed record InstalledEmulator(EmulatorDefinition Definition, string ExePath, EmulatorOrigin Origin, IReadOnlyList<string> Systems, IReadOnlyList<RetroArchCore> Cores)
{
    public string Id => Definition.Id;

    public string Name => Definition.Name;

    public string Folder => WinPath.Directory(ExePath);

    public bool UsesCores => Definition.UsesCores;

    public bool Serves(string system) => Systems.Contains(system, StringComparer.OrdinalIgnoreCase);
}

/// <summary>Uma forma de abrir jogos de um sistema: um emulador (e, no RetroArch, o core).</summary>
public sealed record EmulatorOption(InstalledEmulator Emulator, RetroArchCore? Core)
{
    /// <summary>"RetroArch · Snes9x" ou "PCSX2".</summary>
    public string Label => Core is null ? Emulator.Name : $"{Emulator.Name} · {Core.Name}";

    public EmulatorChoice Choice => new(Emulator.Id, Core?.Id);

    public bool Matches(EmulatorChoice? choice) =>
        choice is not null
        && string.Equals(choice.EmulatorId, Emulator.Id, StringComparison.OrdinalIgnoreCase)
        && (Core is null || choice.Core is null || string.Equals(choice.Core, Core.Id, StringComparison.OrdinalIgnoreCase));
}

/// <summary>Emuladores achados numa leitura; responde quem abre cada sistema e em que ordem.</summary>
public sealed class EmulatorInventory(IReadOnlyList<InstalledEmulator> emulators, EmulatorCatalog catalog)
{
    public static EmulatorInventory Empty { get; } = new([], EmulatorCatalog.Default);

    public IReadOnlyList<InstalledEmulator> Emulators { get; } = emulators;

    public EmulatorCatalog Catalog { get; } = catalog;

    public InstalledEmulator? Find(string? id) =>
        id is null ? null : Emulators.FirstOrDefault(e => string.Equals(e.Id, id, StringComparison.OrdinalIgnoreCase));

    /// <summary>Sistemas que algum emulador instalado abre.</summary>
    public IReadOnlySet<string> CoveredSystems => Emulators.SelectMany(e => e.Systems).ToHashSet(StringComparer.OrdinalIgnoreCase);

    /// <summary>
    /// Opções para o sistema, na ordem de sugestão: a preferência da tabela (ex.: PS2 → PCSX2 antes do RetroArch), os
    /// personalizados logo depois, e no RetroArch os cores sugeridos primeiro. Com a extensão, cores que a aceitam vêm antes.
    /// </summary>
    public IReadOnlyList<EmulatorOption> OptionsFor(string system, string? extension = null)
    {
        ArgumentNullException.ThrowIfNull(system);
        var preferred = Catalog.Preferred.GetValueOrDefault(system) ?? [];
        int Rank(InstalledEmulator e)
        {
            int index = preferred.ToList().FindIndex(p => string.Equals(p, e.Id, StringComparison.OrdinalIgnoreCase));
            return index >= 0 ? index : e.Origin == EmulatorOrigin.Custom ? preferred.Count : preferred.Count + 1 + IndexOf(e);
        }

        var options = new List<EmulatorOption>();
        foreach (var emulator in Emulators.Where(e => e.Serves(system)).OrderBy(Rank))
        {
            if (!emulator.UsesCores)
            {
                options.Add(new EmulatorOption(emulator, null));
                continue;
            }

            var suggested = Catalog.RetroArchCores.Select(c => c.Core).ToList();
            options.AddRange(emulator.Cores
                .Where(c => c.Systems.Contains(system, StringComparer.OrdinalIgnoreCase))
                .OrderBy(c => extension is null || c.Extensions.Count == 0 || c.Extensions.Contains(extension) || Library.Metadata.ScreenScraper.ScreenScraperSystems.ArchiveExtensions.Contains(extension) ? 0 : 1)
                .ThenBy(c => suggested.FindIndex(s => string.Equals(s, c.Id, StringComparison.OrdinalIgnoreCase)) is int i and >= 0 ? i : int.MaxValue)
                .Select(c => new EmulatorOption(emulator, c)));
        }

        return options;
    }

    private int IndexOf(InstalledEmulator emulator) => Emulators.ToList().IndexOf(emulator);
}

/// <summary>
/// Detecta os emuladores da tabela (EMU-001): executável escolhido pelo usuário, pastas padrão, Scoop, winget e
/// "Aplicativos instalados" (registro), nessa ordem; mais os personalizados (EMU-002). Só lê; testado com pastas e
/// registro em memória.
/// </summary>
public sealed class EmulatorDetector(IEmulationFileSystem disk, IRegistryReader registry, EmulationPaths paths, EmulatorCatalog catalog)
{
    public static readonly IReadOnlyList<string> UninstallRoots =
    [
        @"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        @"HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    public EmulatorInventory Detect(EmulationConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);
        var uninstall = ReadUninstallEntries();
        var found = new List<InstalledEmulator>();
        foreach (var definition in catalog.Emulators)
        {
            if (Locate(definition, config.EmulatorPaths.GetValueOrDefault(definition.Id), uninstall) is not { } hit)
            {
                continue;
            }

            if (definition.UsesCores)
            {
                var cores = RetroArch.InstalledCores(disk, WinPath.Directory(hit.Exe), catalog);
                found.Add(new InstalledEmulator(definition, hit.Exe, hit.Origin, cores.SelectMany(c => c.Systems).Distinct(StringComparer.OrdinalIgnoreCase).ToList(), cores));
            }
            else
            {
                found.Add(new InstalledEmulator(definition, hit.Exe, hit.Origin, definition.Systems, []));
            }
        }

        foreach (var custom in config.CustomEmulators)
        {
            if (!string.IsNullOrWhiteSpace(custom.ExePath) && disk.FileExists(custom.ExePath))
            {
                found.Add(new InstalledEmulator(custom.ToDefinition(), custom.ExePath, EmulatorOrigin.Custom, custom.Systems, []));
            }
        }

        return new EmulatorInventory(found, catalog);
    }

    /// <summary>Onde o emulador está; nulo se não foi achado.</summary>
    public (string Exe, EmulatorOrigin Origin)? Locate(EmulatorDefinition definition, string? chosenExe, IReadOnlyList<(string Name, string Folder)>? uninstall = null)
    {
        ArgumentNullException.ThrowIfNull(definition);
        if (!string.IsNullOrWhiteSpace(chosenExe) && disk.FileExists(chosenExe))
        {
            return (chosenExe, EmulatorOrigin.Chosen);
        }

        foreach (string template in definition.Folders)
        {
            if (paths.Expand(template) is { } folder && FindExe(definition, folder, depth: 1) is { } exe)
            {
                return (exe, EmulatorOrigin.Folder);
            }
        }

        foreach (string? root in (string?[])[paths.Scoop, paths.ScoopGlobal])
        {
            if (string.IsNullOrWhiteSpace(root))
            {
                continue;
            }

            foreach (string app in definition.Scoop)
            {
                string folder = WinPath.Combine(WinPath.Combine(WinPath.Combine(root, "apps"), app), "current");
                if (FindExe(definition, folder, depth: 0) is { } exe)
                {
                    return (exe, EmulatorOrigin.Scoop);
                }
            }
        }

        string packages = WinPath.Combine(paths.LocalAppData, @"Microsoft\WinGet\Packages");
        foreach (string package in definition.Winget)
        {
            foreach (string folder in disk.Directories(packages).Where(d => WinPath.FileName(d).StartsWith(package + "_", StringComparison.OrdinalIgnoreCase)))
            {
                if (FindExe(definition, folder, depth: 1) is { } exe)
                {
                    return (exe, EmulatorOrigin.Winget);
                }
            }
        }

        foreach (var (name, folder) in uninstall ?? ReadUninstallEntries())
        {
            if (definition.UninstallNames.Any(n => name.StartsWith(n, StringComparison.OrdinalIgnoreCase)) && FindExe(definition, folder, depth: 1) is { } exe)
            {
                return (exe, EmulatorOrigin.Registry);
            }
        }

        return null;
    }

    /// <summary>Nome e pasta de cada programa em "Aplicativos instalados" (<c>InstallLocation</c>, senão a pasta do <c>DisplayIcon</c>).</summary>
    public IReadOnlyList<(string Name, string Folder)> ReadUninstallEntries()
    {
        var entries = new List<(string, string)>();
        foreach (string root in UninstallRoots)
        {
            foreach (string sub in registry.SubKeyNames(root))
            {
                string key = root + "\\" + sub;
                if (registry.GetString(key, "DisplayName") is not { Length: > 0 } name)
                {
                    continue;
                }

                string? folder = registry.GetString(key, "InstallLocation");
                if (string.IsNullOrWhiteSpace(folder) && registry.GetString(key, "DisplayIcon") is { Length: > 0 } icon)
                {
                    string iconPath = icon.Split(',')[0].Trim().Trim('"');
                    folder = WinPath.Extension(iconPath) == ".exe" ? WinPath.Directory(iconPath) : null;
                }

                if (!string.IsNullOrWhiteSpace(folder))
                {
                    entries.Add((name, folder.Trim().Trim('"')));
                }
            }
        }

        return entries;
    }

    private string? FindExe(EmulatorDefinition definition, string folder, int depth)
    {
        if (!disk.DirectoryExists(folder))
        {
            return null;
        }

        foreach (string exe in definition.Executables)
        {
            string path = WinPath.Combine(folder, exe);
            if (disk.FileExists(path))
            {
                return path;
            }
        }

        // Zip extraído com uma pasta a mais (ex.: "RetroArch-Win64\RetroArch-Win64\retroarch.exe").
        return depth <= 0 ? null : disk.Directories(folder).Select(sub => FindExe(definition, sub, depth - 1)).FirstOrDefault(e => e is not null);
    }
}
