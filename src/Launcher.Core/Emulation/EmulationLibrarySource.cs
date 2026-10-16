using System.Security.Cryptography;
using System.Text;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.Core.Library.Sources;

namespace Launcher.Core.Emulation;

/// <summary>
/// Fonte "Emulação" da biblioteca (EMU-003, ADR-0019/0024): os jogos das pastas de ROMs, como qualquer loja. Liga e
/// desliga em Configurações › Emulação; o aviso diz o que falta (pasta, emulador para um sistema).
/// </summary>
public sealed class EmulationLibrarySource(Func<EmulationConfig> config, RomScanner scanner, Func<EmulatorInventory> inventory) : ILibrarySource
{
    public const string Accent = "#4B3F72";

    public GameStore Store => GameStore.Emulation;

    public LibraryScan Scan()
    {
        var folders = config().RomFolders;
        var emulators = inventory();
        if (folders.Count == 0)
        {
            return new LibraryScan([], emulators.Emulators.Count > 0, "Nenhuma pasta de ROMs: adicione em Configurações › Emulação");
        }

        var entries = scanner.Scan(folders);
        var games = entries.Select(ToGame).ToList();
        var covered = emulators.CoveredSystems;
        var uncovered = entries.Select(e => e.System).Distinct(StringComparer.OrdinalIgnoreCase).Where(s => !covered.Contains(s)).ToList();
        string? notice = uncovered.Count == 0 ? null : $"Sem emulador para {Join(uncovered.Select(s => ScreenScraperSystems.ForKey(s)?.Name ?? s).ToList())}";
        return new LibraryScan(games, emulators.Emulators.Count > 0, notice);
    }

    /// <summary>Id estável por arquivo ("emu:snes:3f2a…"): favoritos, tempo e metadados sobrevivem a novas leituras.</summary>
    public static string GameId(string system, string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        byte[] hash = SHA256.HashData(Encoding.UTF8.GetBytes(WinPath.Normalize(path)));
        return $"emu:{system}:{Convert.ToHexStringLower(hash.AsSpan(0, 8))}";
    }

    public static Game ToGame(RomEntry entry)
    {
        ArgumentNullException.ThrowIfNull(entry);
        return new Game
        {
            Id = GameId(entry.System, entry.Path),
            Title = entry.Title,
            Store = GameStore.Emulation,
            Accent = Accent,
            IsInstalled = true,
            LaunchTarget = entry.Path,
            WorkingDirectory = WinPath.Directory(entry.Path),
            InstallPath = WinPath.Directory(entry.Path),
            Platform = entry.System,
            RomPath = entry.Path,
            SizeLabel = entry.Discs.Count > 1 ? $"{entry.Discs.Count} discos" : null,
            AddedAt = entry.FoundAt ?? DateTimeOffset.UnixEpoch,
        };
    }

    private static string Join(List<string> names) => names.Count switch
    {
        1 => names[0],
        _ => string.Join(", ", names.Take(names.Count - 1)) + " e " + names[^1],
    };
}
