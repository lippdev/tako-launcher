using System.Globalization;
using Launcher.Core.Library.Sources;

namespace Launcher.Core.Library.Playnite;

/// <summary>Jogo do Playnite que entra no Tako como "Adicionado", com as coleções (categorias do Playnite) dele.</summary>
public sealed record PlayniteNewGame(PlayniteGame Source, ManualGameRecord Record, string? PlatformName, IReadOnlyList<string> Collections);

/// <summary>Jogo do Playnite que o Tako já tem (pela loja ou importado antes): só completa favorito, tempo e coleções.</summary>
public sealed record PlayniteMatchedGame(PlayniteGame Source, string TakoId, IReadOnlyList<string> Collections);

/// <summary>Por que um jogo do Playnite fica de fora da importação.</summary>
public enum PlayniteSkipReason
{
    /// <summary>Jogo de uma loja que o Tako lê sozinho (aparece quando instalado e com a loja ligada).</summary>
    StoreGame,

    /// <summary>Não instalado no Playnite ou sem ação de jogar.</summary>
    NotPlayable,

    /// <summary>Abre por script do Playnite ou usa variáveis que só o Playnite resolve.</summary>
    PlayniteOnly,
}

public sealed record PlayniteSkippedGame(PlayniteGame Source, PlayniteSkipReason Reason);

/// <summary>Prévia da importação (o que entra, o que só é completado e o que fica de fora), antes da confirmação.</summary>
public sealed record PlayniteImportPlan(
    IReadOnlyList<PlayniteNewGame> New,
    IReadOnlyList<PlayniteMatchedGame> Matched,
    IReadOnlyList<PlayniteSkippedGame> Skipped)
{
    /// <summary>Coleções que serão criadas (as que o Tako ainda não tem, pelo nome).</summary>
    public IReadOnlyList<string> CollectionNames { get; init; } = [];

    public bool IsEmpty => New.Count == 0 && Matched.Count == 0;

    /// <summary>Texto da prévia em pt-BR: "12 jogos novos, 30 que o Tako já tem (favoritos e tempo completados) e 3 coleções".</summary>
    public string Summary
    {
        get
        {
            if (IsEmpty)
            {
                return Skipped.Count == 0
                    ? "A biblioteca do Playnite está vazia"
                    : $"Nada para importar: {Count(Skipped.Count, "jogo", "jogos")} de lojas que o Tako já lê, não instalados ou que só abrem pelo Playnite";
            }

            var parts = new List<string>();
            if (New.Count > 0)
            {
                parts.Add(New.Count == 1 ? "1 jogo novo" : $"{Count(New.Count)} jogos novos");
            }

            if (Matched.Count > 0)
            {
                parts.Add($"{Count(Matched.Count, "jogo", "jogos")} que o Tako já tem (favoritos e tempo completados)");
            }

            if (CollectionNames.Count > 0)
            {
                parts.Add(Count(CollectionNames.Count, "coleção", "coleções"));
            }

            string list = parts.Count == 1 ? parts[0] : $"{string.Join(", ", parts.SkipLast(1))} e {parts[^1]}";
            return Skipped.Count == 0 ? list : $"{list}. Ficam de fora {Count(Skipped.Count, "jogo", "jogos")} (lojas que o Tako já lê, não instalados ou só pelo Playnite)";
        }
    }

    private static string Count(int n) => n.ToString(CultureInfo.InvariantCulture);

    private static string Count(int n, string one, string many) => $"{Count(n)} {(n == 1 ? one : many)}";
}

/// <summary>
/// Importação única da biblioteca do Playnite (STORE-009), com prévia e confirmação. Decide jogo a jogo sem tocar em
/// arquivo: os de lojas que o Tako lê casam pelo id da loja ou pela pasta (nunca duplicam); os demais instalados com
/// ação de arquivo ou link entram como "Adicionados"; ROMs entram com o sistema (abrem quando a emulação chegar).
/// </summary>
public static class PlayniteImport
{
    /// <summary>Plugins de biblioteca do Playnite (ids fixos dos plugins oficiais) ↔ lojas que o Tako lê.</summary>
    public static readonly IReadOnlyDictionary<Guid, GameStore> StorePlugins = new Dictionary<Guid, GameStore>
    {
        [new Guid("cb91dfc9-b977-43bf-8e70-55f46e410fab")] = GameStore.Steam,
        [new Guid("00000002-dbd1-46c6-b5d0-b1ba559d10e4")] = GameStore.Epic,
        [new Guid("aebe8b7c-6dc3-4a66-af31-e7375c6b5e9e")] = GameStore.Gog,
        [new Guid("7e4fbb5e-2ae3-48d4-8ba0-6b30e7a4e287")] = GameStore.Xbox,
        [new Guid("c2f038e5-8b92-4877-91f1-da9094155fc5")] = GameStore.Ubisoft,
        [new Guid("85dd7072-2f20-4e76-a007-41035e390724")] = GameStore.Ea,
        [new Guid("e3c26a3d-d695-4cb7-a769-5ff7612c7edd")] = GameStore.BattleNet,
        [new Guid("402674cd-4af6-4886-b6ec-0e695bfa0688")] = GameStore.Amazon,
    };

    /// <summary>Id de especificação de plataforma do Playnite → sistema do Tako (<see cref="Game.Platform"/>).</summary>
    public static readonly IReadOnlyDictionary<string, string> PlatformKeys = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
    {
        ["nintendo_nes"] = "nes",
        ["nintendo_super_nes"] = "snes",
        ["nintendo_64"] = "n64",
        ["nintendo_gamecube"] = "gamecube",
        ["nintendo_wii"] = "wii",
        ["nintendo_wiiu"] = "wiiu",
        ["nintendo_switch"] = "switch",
        ["nintendo_gameboy"] = "gb",
        ["nintendo_gameboycolor"] = "gbc",
        ["nintendo_gameboyadvance"] = "gba",
        ["nintendo_ds"] = "nds",
        ["nintendo_3ds"] = "3ds",
        ["sega_mastersystem"] = "mastersystem",
        ["sega_genesis"] = "megadrive",
        ["sega_cd"] = "segacd",
        ["sega_32x"] = "32x",
        ["sega_gamegear"] = "gamegear",
        ["sega_saturn"] = "saturn",
        ["sega_dreamcast"] = "dreamcast",
        ["sony_playstation"] = "ps1",
        ["sony_playstation2"] = "ps2",
        ["sony_playstation3"] = "ps3",
        ["sony_psp"] = "psp",
        ["sony_vita"] = "psvita",
        ["xbox"] = "xbox",
        ["xbox360"] = "xbox360",
        ["nec_turbografx_16"] = "pcengine",
        ["atari_2600"] = "atari2600",
    };

    /// <summary>Id fixo do jogo importado: importar de novo não duplica.</summary>
    public static string ImportedId(Guid playniteId) => $"manual:playnite-{playniteId:N}";

    public static PlayniteImportPlan Plan(PlayniteLibrary library, IReadOnlyList<Game> existing, IEnumerable<string> existingCollections, DateTimeOffset now)
    {
        ArgumentNullException.ThrowIfNull(library);
        ArgumentNullException.ThrowIfNull(existing);
        ArgumentNullException.ThrowIfNull(existingCollections);
        var byId = existing.GroupBy(g => g.Id, StringComparer.OrdinalIgnoreCase).ToDictionary(g => g.Key, g => g.First(), StringComparer.OrdinalIgnoreCase);
        var newGames = new List<PlayniteNewGame>();
        var matched = new List<PlayniteMatchedGame>();
        var skipped = new List<PlayniteSkippedGame>();
        foreach (var source in library.Games)
        {
            var collections = source.CategoryIds
                .Select(id => library.Categories.GetValueOrDefault(id))
                .OfType<string>()
                .Select(CollectionNames.Clean)
                .Select(n => n.Length > CollectionNames.MaxLength ? n[..CollectionNames.MaxLength].TrimEnd() : n)
                .Where(n => n.Length > 0)
                .Distinct(LibraryQuery.TitleComparer)
                .ToList();

            if (Match(source, existing, byId) is { } match)
            {
                matched.Add(new PlayniteMatchedGame(source, match.Id, collections));
                continue;
            }

            if (StorePlugins.ContainsKey(source.PluginId))
            {
                skipped.Add(new PlayniteSkippedGame(source, PlayniteSkipReason.StoreGame));
                continue;
            }

            var platform = library.PlatformOf(source);
            var (record, reason) = ToRecord(source, platform, now);
            if (record is null)
            {
                skipped.Add(new PlayniteSkippedGame(source, reason));
                continue;
            }

            newGames.Add(new PlayniteNewGame(source, record, platform?.Name, collections));
        }

        var known = new HashSet<string>(existingCollections, LibraryQuery.TitleComparer);
        var created = newGames.SelectMany(g => g.Collections).Concat(matched.SelectMany(g => g.Collections))
            .Where(n => !known.Contains(n))
            .Distinct(LibraryQuery.TitleComparer)
            .Order(LibraryQuery.TitleComparer)
            .ToList();
        return new PlayniteImportPlan(newGames, matched, skipped) { CollectionNames = created };
    }

    /// <summary>
    /// Estado a gravar para um jogo que o Tako já tem: favorito vale se estiver em qualquer um; tempo e "jogado por
    /// último" ficam com o maior (a Steam já soma o próprio tempo, então o dela não muda).
    /// </summary>
    public static GameStateRecord MergeState(GameStateRecord? current, PlayniteGame source, GameStore store)
    {
        ArgumentNullException.ThrowIfNull(source);
        current ??= new GameStateRecord();
        var last = source.LastActivity is { } activity && (current.LastPlayedAt is null || activity > current.LastPlayedAt) ? activity : current.LastPlayedAt;
        double minutes = store == GameStore.Steam ? current.PlayMinutes : Math.Max(current.PlayMinutes, source.Playtime.TotalMinutes);
        return current with { IsFavorite = current.IsFavorite || source.Favorite, LastPlayedAt = last, PlayMinutes = minutes };
    }

    /// <summary>O jogo do Tako que é este jogo do Playnite: já importado, mesmo id de loja ou mesma pasta na mesma loja.</summary>
    public static Game? Match(PlayniteGame source, IReadOnlyList<Game> existing, IReadOnlyDictionary<string, Game>? byId = null)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(existing);
        byId ??= existing.GroupBy(g => g.Id, StringComparer.OrdinalIgnoreCase).ToDictionary(g => g.Key, g => g.First(), StringComparer.OrdinalIgnoreCase);
        if (byId.TryGetValue(ImportedId(source.Id), out var imported))
        {
            return imported;
        }

        if (StorePlugins.TryGetValue(source.PluginId, out var store) && !string.IsNullOrWhiteSpace(source.GameId))
        {
            string gameId = source.GameId.Trim();
            string? takoId = store switch
            {
                GameStore.Steam => $"steam:{gameId}",
                GameStore.Epic => $"epic:{gameId}",
                GameStore.Gog => $"gog:{gameId}",
                GameStore.Ubisoft => $"ubisoft:{gameId}",
                GameStore.Amazon => $"amazon:{gameId}",
                GameStore.Ea => $"ea:{gameId}",
                _ => null,
            };
            if (takoId is not null && byId.TryGetValue(takoId, out var sameId))
            {
                return sameId;
            }

            // Xbox (StoreId ou família do pacote), Battle.net (código "Pro"), EA (oferta): pelo id do produto na loja.
            var sameProduct = existing.FirstOrDefault(g => g.Store == store && string.Equals(g.StoreProductId, gameId, StringComparison.OrdinalIgnoreCase));
            if (sameProduct is not null)
            {
                return sameProduct;
            }
        }

        // Mesma pasta de instalação (qualquer loja do Tako, inclusive um "Adicionado" igual).
        if (!string.IsNullOrWhiteSpace(source.InstallDirectory))
        {
            string folder = WindowsPath.Normalize(source.InstallDirectory);
            var sameFolder = existing.FirstOrDefault(g => g.InstallPath is { } path
                && string.Equals(WindowsPath.Normalize(path), folder, StringComparison.OrdinalIgnoreCase)
                && (!StorePlugins.TryGetValue(source.PluginId, out var plugin) || plugin == g.Store || g.Store == GameStore.Manual));
            if (sameFolder is not null)
            {
                return sameFolder;
            }
        }

        // Jogo adicionado à mão nos dois: mesmo nome entre os "Adicionados".
        string key = StoreDuplicates.Key(source.Name);
        return key.Length == 0 ? null : existing.FirstOrDefault(g => g.Store == GameStore.Manual && StoreDuplicates.Key(g.Title) == key);
    }

    /// <summary>
    /// Registro de "Adicionado" para o jogo: a ação de jogar de arquivo ou link (com <c>{InstallDir}</c> resolvido) ou,
    /// para jogo de emulador, a primeira ROM com o sistema reconhecido.
    /// </summary>
    public static (ManualGameRecord? Record, PlayniteSkipReason Reason) ToRecord(PlayniteGame source, PlaynitePlatform? platform, DateTimeOffset now)
    {
        ArgumentNullException.ThrowIfNull(source);
        var action = source.Actions.FirstOrDefault(a => a.IsPlayAction) ?? (source.Actions.Count > 0 ? source.Actions[0] : null);
        string id = ImportedId(source.Id);
        var added = source.Added ?? now;
        if (action?.Type == PlayniteActionType.Emulator)
        {
            string? system = platform?.SpecificationId is { } spec ? PlatformKeys.GetValueOrDefault(spec) : null;
            string? rom = source.Roms.Select(r => Expand(r.Path, source)).FirstOrDefault(p => p is not null && IsRooted(p));
            if (system is null || rom is null)
            {
                return (null, PlayniteSkipReason.PlayniteOnly);
            }

            return (new ManualGameRecord
            {
                Id = id,
                Title = source.Name,
                Path = rom,
                WorkingDirectory = DirectoryOf(rom),
                AddedAt = added,
                Platform = system,
            }, default);
        }

        if (!source.IsInstalled || action is null)
        {
            return (null, PlayniteSkipReason.NotPlayable);
        }

        if (action.Type is not (PlayniteActionType.File or PlayniteActionType.Url))
        {
            return (null, PlayniteSkipReason.PlayniteOnly);
        }

        string? path = Expand(action.Path, source);
        string? arguments = action.Arguments is null ? null : Expand(action.Arguments, source);
        string? workingDir = action.WorkingDir is null ? null : Expand(action.WorkingDir, source);
        if (path is null || (action.Arguments is not null && arguments is null) || (action.WorkingDir is not null && workingDir is null))
        {
            return (null, PlayniteSkipReason.PlayniteOnly);
        }

        bool isUri = action.Type == PlayniteActionType.Url || (Uri.TryCreate(path, UriKind.Absolute, out var uri) && !uri.IsFile);
        if (!isUri && !IsRooted(path) && source.InstallDirectory is { } installDir)
        {
            path = WindowsPath.Combine(installDir, path); // caminho relativo à pasta do jogo, como o Playnite resolve
        }

        if (!isUri && !IsRooted(path))
        {
            return (null, PlayniteSkipReason.PlayniteOnly);
        }

        return (new ManualGameRecord
        {
            Id = id,
            Title = source.Name,
            Path = path,
            Arguments = string.IsNullOrWhiteSpace(arguments) ? null : arguments,
            WorkingDirectory = workingDir ?? (isUri ? null : source.InstallDirectory ?? DirectoryOf(path)),
            AddedAt = added,
        }, default);
    }

    /// <summary>Resolve <c>{InstallDir}</c> e <c>{InstallDirName}</c>; outra variável do Playnite = nulo (não importável).</summary>
    public static string? Expand(string? value, PlayniteGame source)
    {
        ArgumentNullException.ThrowIfNull(source);
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        string text = value.Trim();
        if (source.InstallDirectory is { } dir)
        {
            string folder = WindowsPath.Normalize(dir);
            text = text.Replace("{InstallDir}", folder, StringComparison.OrdinalIgnoreCase)
                .Replace("{InstallDirName}", WindowsPath.LastName(folder), StringComparison.OrdinalIgnoreCase);
        }

        return text.Contains('{', StringComparison.Ordinal) && text.Contains('}', StringComparison.Ordinal) ? null : text;
    }

    private static bool IsRooted(string path) =>
        (path.Length >= 3 && char.IsAsciiLetter(path[0]) && path[1] == ':' && path[2] is '\\' or '/') || path.StartsWith(@"\\", StringComparison.Ordinal);

    private static string? DirectoryOf(string path)
    {
        string normalized = path.Replace('/', '\\');
        int slash = normalized.LastIndexOf('\\');
        return slash <= 0 ? null : normalized[..slash];
    }
}
