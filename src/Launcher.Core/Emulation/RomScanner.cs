using System.Globalization;
using System.Text.Json;
using System.Text.RegularExpressions;
using Launcher.Core.Library.Metadata.ScreenScraper;

namespace Launcher.Core.Emulation;

/// <summary>Um jogo achado nas pastas de ROMs: o arquivo que abre, o sistema, o título e os discos (multi-disco).</summary>
/// <param name="Path">Arquivo passado ao emulador (.m3u, .cue, o primeiro disco, a ROM).</param>
/// <param name="Discs">Todos os discos do jogo, na ordem (um só para jogos de um disco/cartucho).</param>
/// <param name="FoundAt">Quando a pasta mudou pela última vez (vira "adicionado em").</param>
public sealed record RomEntry(string Path, string System, string Title, IReadOnlyList<string> Discs, DateTimeOffset? FoundAt = null);

/// <summary>
/// Lê as pastas de ROMs (EMU-003, ADR-0024): pasta de um sistema, ou pasta raiz com subpastas reconhecidas ("snes",
/// "psx", nomes do ES-DE). Junta jogos de vários discos (.m3u; .cue/.gdi com as trilhas; "(Disc 1)", "(Disc 2)") e
/// aceita .zip de arcade. Só lê nomes de arquivos (nada é aberto além de .m3u/.cue/.gdi); testado com pastas em memória.
/// </summary>
public sealed partial class RomScanner(IEmulationFileSystem disk, EmulatorCatalog catalog, RomScanCache? cache = null)
{
    /// <summary>Profundidade máxima dentro da pasta do sistema (jogos em subpastas próprias).</summary>
    public const int MaxDepth = 3;

    // Pastas de mídia e BIOS dentro das pastas de ROMs (ES-DE, RetroBat, Batocera...).
    private static readonly HashSet<string> IgnoredFolders = new(StringComparer.OrdinalIgnoreCase)
    {
        "bios", "media", "images", "videos", "manuals", "downloaded_media", "screenshots", "snap", "snaps", "boxart", "covers", "saves", "states",
    };

    // Imagens de disco que listam outros arquivos (as trilhas não viram jogos à parte).
    private static readonly HashSet<string> Playlists = new(StringComparer.OrdinalIgnoreCase) { ".m3u", ".cue", ".gdi" };

    public IReadOnlyList<RomEntry> Scan(IEnumerable<RomFolder> folders)
    {
        ArgumentNullException.ThrowIfNull(folders);
        cache?.BeginScan();
        var entries = new List<RomEntry>();
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var folder in folders)
        {
            foreach (var (path, system) in SystemFolders(folder))
            {
                foreach (var entry in ScanSystem(path, system))
                {
                    if (seen.Add(WinPath.Normalize(entry.Path)))
                    {
                        entries.Add(entry);
                    }
                }
            }
        }

        cache?.Prune();
        return entries;
    }

    /// <summary>Pastas de sistema dentro de uma pasta configurada: ela mesma (sistema escolhido ou nome reconhecido) ou as subpastas.</summary>
    public IReadOnlyList<(string Path, EmulatedSystem System)> SystemFolders(RomFolder folder)
    {
        ArgumentNullException.ThrowIfNull(folder);
        if (ScreenScraperSystems.ForKey(folder.System) is { } chosen)
        {
            return [(folder.Path, chosen)];
        }

        if (ScreenScraperSystems.ForKey(catalog.SystemForFolder(WinPath.FileName(folder.Path))) is { } named)
        {
            return [(folder.Path, named)];
        }

        var found = new List<(string, EmulatedSystem)>();
        foreach (string sub in disk.Directories(folder.Path))
        {
            string name = WinPath.FileName(sub);
            if (ScreenScraperSystems.ForKey(catalog.SystemForFolder(name)) is { } system)
            {
                found.Add((sub, system));
            }
            else if (string.Equals(name, "roms", StringComparison.OrdinalIgnoreCase))
            {
                found.AddRange(SystemFolders(new RomFolder(sub)));
            }
        }

        return found;
    }

    /// <summary>Extensões aceitas no sistema: as dele, .m3u e, em sistemas de cartucho e arcade, .zip/.7z.</summary>
    public static IReadOnlySet<string> Extensions(EmulatedSystem system)
    {
        ArgumentNullException.ThrowIfNull(system);
        var extensions = new HashSet<string>(system.Extensions, StringComparer.OrdinalIgnoreCase) { ".m3u" };
        if (ScreenScraperSystems.CandidatesFor("x.zip").Contains(system))
        {
            extensions.UnionWith(ScreenScraperSystems.ArchiveExtensions);
        }

        return extensions;
    }

    private List<RomEntry> ScanSystem(string folder, EmulatedSystem system)
    {
        var accepted = Extensions(system);
        var files = new List<(string Path, DateTimeOffset? Stamp)>();
        Collect(folder, 0, files);

        // Arquivos citados por .m3u, .cue e .gdi são partes de outro jogo.
        var consumed = new HashSet<string>(StringComparer.Ordinal);
        foreach (var (path, _) in files.Where(f => Playlists.Contains(WinPath.Extension(f.Path)) && accepted.Contains(WinPath.Extension(f.Path))))
        {
            foreach (string part in ReferencedFiles(path))
            {
                consumed.Add(WinPath.Normalize(part));
            }
        }

        var candidates = files
            .Where(f => accepted.Contains(WinPath.Extension(f.Path)) && !consumed.Contains(WinPath.Normalize(f.Path)))
            .ToList();

        // "Jogo (Disc 1).chd" + "Jogo (Disc 2).chd" na mesma pasta = um jogo com dois discos.
        var entries = new List<RomEntry>();
        foreach (var group in candidates.GroupBy(f => (WinPath.Normalize(WinPath.Directory(f.Path)), DiscBase(f.Path), WinPath.Extension(f.Path))))
        {
            var discs = group.OrderBy(f => DiscNumber(f.Path)).ThenBy(f => f.Path, StringComparer.OrdinalIgnoreCase).ToList();
            string first = discs[0].Path;
            entries.Add(new RomEntry(first, system.Key, RomTitle.FromFileName(WinPath.FileName(first)), [.. discs.Select(d => d.Path)], discs[0].Stamp));
        }

        return [.. entries.OrderBy(e => e.Title, StringComparer.OrdinalIgnoreCase)];
    }

    private void Collect(string folder, int depth, List<(string, DateTimeOffset?)> files)
    {
        DateTimeOffset? stamp = disk.LastWrite(folder);
        foreach (string file in cache?.FilesOf(disk, folder, stamp) ?? disk.Files(folder))
        {
            files.Add((file, stamp));
        }

        if (depth >= MaxDepth)
        {
            return;
        }

        foreach (string sub in disk.Directories(folder))
        {
            string name = WinPath.FileName(sub);
            if (!IgnoredFolders.Contains(name) && !name.StartsWith('.'))
            {
                Collect(sub, depth + 1, files);
            }
        }
    }

    /// <summary>Arquivos citados por um .m3u (uma linha por disco), .cue (<c>FILE "x.bin" BINARY</c>) ou .gdi (trilhas).</summary>
    public IReadOnlyList<string> ReferencedFiles(string playlist)
    {
        ArgumentNullException.ThrowIfNull(playlist);
        string? text = disk.ReadText(playlist);
        if (text is null)
        {
            return [];
        }

        string folder = WinPath.Directory(playlist);
        string extension = WinPath.Extension(playlist);
        var lines = text.Split('\n').Select(l => l.Trim().TrimEnd('\r')).Where(l => l.Length > 0);
        return extension switch
        {
            ".m3u" => [.. lines.Where(l => !l.StartsWith('#')).Select(l => WinPath.Resolve(folder, l))],
            ".cue" => [.. lines.Select(l => CueFile().Match(l)).Where(m => m.Success).Select(m => WinPath.Resolve(folder, m.Groups["file"].Value))],
            ".gdi" => [.. lines.Skip(1).Select(GdiTrack).OfType<string>().Select(t => WinPath.Resolve(folder, t))],
            _ => [],
        };
    }

    /// <summary>Nome sem a marcação de disco: "Jogo (USA) (Disc 2)" → "Jogo (USA)".</summary>
    public static string DiscBase(string path) => DiscTag().Replace(WinPath.NameWithoutExtension(path), string.Empty).Trim().ToUpperInvariant();

    /// <summary>Número do disco ("(Disc 2)" → 2); sem marcação = 0.</summary>
    public static int DiscNumber(string path) =>
        DiscTag().Match(WinPath.NameWithoutExtension(path)) is { Success: true } match ? int.Parse(match.Groups["n"].Value, CultureInfo.InvariantCulture) : 0;

    // Linha do .gdi: "1 0 4 2352 track01.bin 0" ou com o nome entre aspas.
    private static string? GdiTrack(string line)
    {
        var quoted = GdiQuoted().Match(line);
        if (quoted.Success)
        {
            return quoted.Groups["f"].Value;
        }

        string[] parts = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        return parts.Length >= 5 ? parts[4] : null;
    }

    [GeneratedRegex("\"(?<f>[^\"]+)\"")]
    private static partial Regex GdiQuoted();

    [GeneratedRegex(@"^FILE\s+(?:""(?<file>[^""]+)""|(?<file>\S+))", RegexOptions.IgnoreCase)]
    private static partial Regex CueFile();

    [GeneratedRegex(@"\s*\((?:Disc|Disk|CD)\s*(?<n>\d+)(?:\s*of\s*\d+)?\)", RegexOptions.IgnoreCase)]
    private static partial Regex DiscTag();
}

/// <summary>
/// Leitura incremental das pastas de ROMs (EMU-003): guarda a lista de arquivos de cada pasta com a data de
/// modificação dela; pasta sem mudança não é listada de novo. Fica em <c>cache\rom-scan.json</c>.
/// </summary>
public sealed class RomScanCache
{
    private readonly Dictionary<string, Entry> _entries = new(StringComparer.Ordinal);
    private readonly HashSet<string> _touched = new(StringComparer.Ordinal);

    /// <summary>Pastas reaproveitadas na última leitura (para testes e diagnóstico).</summary>
    public int Hits { get; private set; }

    /// <summary>Pastas listadas de novo na última leitura.</summary>
    public int Misses { get; private set; }

    public IReadOnlyList<string> FilesOf(IEmulationFileSystem disk, string folder, DateTimeOffset? stamp)
    {
        ArgumentNullException.ThrowIfNull(disk);
        string key = WinPath.Normalize(folder);
        _touched.Add(key);
        if (stamp is { } when && _entries.TryGetValue(key, out var entry) && entry.Stamp == when.UtcTicks)
        {
            Hits++;
            return entry.Files;
        }

        Misses++;
        var files = disk.Files(folder);
        if (stamp is { } now)
        {
            _entries[key] = new Entry(now.UtcTicks, [.. files]);
        }

        return files;
    }

    /// <summary>Esquece as pastas que a última leitura não visitou (removidas da configuração ou apagadas).</summary>
    public void Prune()
    {
        foreach (string key in _entries.Keys.Where(k => !_touched.Contains(k)).ToList())
        {
            _entries.Remove(key);
        }

        _touched.Clear();
    }

    /// <summary>Começa uma leitura nova (zera os contadores).</summary>
    public void BeginScan()
    {
        Hits = 0;
        Misses = 0;
        _touched.Clear();
    }

    public static RomScanCache Load(string path)
    {
        var cache = new RomScanCache();
        try
        {
            if (File.Exists(path) && JsonSerializer.Deserialize<Dictionary<string, Entry>>(File.ReadAllText(path)) is { } saved)
            {
                foreach (var (key, entry) in saved)
                {
                    cache._entries[key] = entry;
                }
            }
        }
        catch (Exception ex) when (ex is JsonException or IOException or UnauthorizedAccessException)
        {
            // Cache estragado: lê tudo de novo.
        }

        return cache;
    }

    public void Save(string path)
    {
        try
        {
            string? folder = System.IO.Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(folder))
            {
                Directory.CreateDirectory(folder);
            }

            File.WriteAllText(path, JsonSerializer.Serialize(_entries));
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Sem cache, a próxima leitura só demora mais.
        }
    }

    public sealed record Entry(long Stamp, IReadOnlyList<string> Files);
}
