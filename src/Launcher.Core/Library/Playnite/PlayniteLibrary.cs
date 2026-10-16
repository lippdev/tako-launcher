using System.Globalization;

namespace Launcher.Core.Library.Playnite;

/// <summary>Tipo da ação de jogar no Playnite (<c>GameActionType</c>).</summary>
public enum PlayniteActionType
{
    File,
    Url,
    Emulator,
    Script,
}

public sealed record PlayniteAction(PlayniteActionType Type, string? Path, string? Arguments, string? WorkingDir, bool IsPlayAction);

public sealed record PlayniteRom(string? Name, string? Path);

/// <summary>Um jogo da biblioteca do Playnite, só com o que a importação usa.</summary>
public sealed record PlayniteGame
{
    public required Guid Id { get; init; }

    public required string Name { get; init; }

    /// <summary>Plugin da biblioteca (Steam, Epic…); vazio = jogo adicionado à mão no Playnite.</summary>
    public Guid PluginId { get; init; }

    /// <summary>Id do jogo na loja do plugin (appid da Steam, AppName da Epic, id da Ubisoft…).</summary>
    public string? GameId { get; init; }

    public string? InstallDirectory { get; init; }

    public bool IsInstalled { get; init; }

    public bool Hidden { get; init; }

    public bool Favorite { get; init; }

    /// <summary>Tempo jogado registrado pelo Playnite.</summary>
    public TimeSpan Playtime { get; init; }

    public DateTimeOffset? LastActivity { get; init; }

    public DateTimeOffset? Added { get; init; }

    public IReadOnlyList<Guid> PlatformIds { get; init; } = [];

    public IReadOnlyList<Guid> CategoryIds { get; init; } = [];

    public Guid SourceId { get; init; }

    public IReadOnlyList<PlayniteAction> Actions { get; init; } = [];

    public IReadOnlyList<PlayniteRom> Roms { get; init; } = [];
}

/// <summary>Plataforma do Playnite: nome e o id de especificação ("pc_windows", "nintendo_super_nes").</summary>
public sealed record PlaynitePlatform(string Name, string? SpecificationId);

/// <summary>
/// A biblioteca do Playnite (<c>%APPDATA%\Playnite\library</c>, ou a pasta <c>library</c> da versão portátil), lida
/// sem alterar nada (STORE-009): jogos, plataformas, categorias e fontes, cada um no próprio arquivo do LiteDB 4.
/// </summary>
public sealed record PlayniteLibrary(
    IReadOnlyList<PlayniteGame> Games,
    IReadOnlyDictionary<Guid, PlaynitePlatform> Platforms,
    IReadOnlyDictionary<Guid, string> Categories,
    IReadOnlyDictionary<Guid, string> Sources)
{
    /// <summary>
    /// Lê a pasta. Sem <c>games.db</c> lança <see cref="FileNotFoundException"/>; formato desconhecido ou arquivo
    /// corrompido lança <see cref="InvalidDataException"/>; arquivo preso por outro programa lança <see cref="IOException"/>.
    /// </summary>
    public static PlayniteLibrary Read(string folder)
    {
        ArgumentNullException.ThrowIfNull(folder);
        string games = Path.Combine(folder, "games.db");
        if (!File.Exists(games))
        {
            throw new FileNotFoundException("Biblioteca do Playnite não encontrada", games);
        }

        return FromDocuments(
            Documents(games),
            Documents(Path.Combine(folder, "platforms.db")),
            Documents(Path.Combine(folder, "categories.db")),
            Documents(Path.Combine(folder, "sources.db")));
    }

    /// <summary>Monta a biblioteca a partir dos documentos BSON já lidos (testável sem arquivo).</summary>
    public static PlayniteLibrary FromDocuments(
        IEnumerable<Dictionary<string, object?>> games,
        IEnumerable<Dictionary<string, object?>> platforms,
        IEnumerable<Dictionary<string, object?>> categories,
        IEnumerable<Dictionary<string, object?>> sources)
    {
        ArgumentNullException.ThrowIfNull(games);
        ArgumentNullException.ThrowIfNull(platforms);
        ArgumentNullException.ThrowIfNull(categories);
        ArgumentNullException.ThrowIfNull(sources);
        return new PlayniteLibrary(
            [.. games.Select(ToGame).OfType<PlayniteGame>()],
            Named(platforms, d => new PlaynitePlatform(Text(d, "Name")!, Text(d, "SpecificationId"))),
            Named(categories, d => Text(d, "Name")!),
            Named(sources, d => Text(d, "Name")!));
    }

    /// <summary>Nome da plataforma ("PC (Windows)") ou nulo.</summary>
    public PlaynitePlatform? PlatformOf(PlayniteGame game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return game.PlatformIds.Select(id => Platforms.GetValueOrDefault(id)).FirstOrDefault(p => p is not null);
    }

    private static IReadOnlyList<Dictionary<string, object?>> Documents(string file)
    {
        if (!File.Exists(file))
        {
            return [];
        }

        var reader = LiteDbV7Reader.Open(file);
        // Uma coleção por arquivo (o nome é o do tipo: "Game", "Platform"…); se houver outras, a maior é a dos dados.
        return reader.Collections.Select(reader.Documents).OrderByDescending(d => d.Count).FirstOrDefault() ?? [];
    }

    private static Dictionary<Guid, T> Named<T>(IEnumerable<Dictionary<string, object?>> documents, Func<Dictionary<string, object?>, T> create)
    {
        var result = new Dictionary<Guid, T>();
        foreach (var document in documents)
        {
            if (Id(document) is Guid id && !string.IsNullOrWhiteSpace(Text(document, "Name")))
            {
                result[id] = create(document);
            }
        }

        return result;
    }

    private static PlayniteGame? ToGame(Dictionary<string, object?> d)
    {
        if (Id(d) is not Guid id || Text(d, "Name") is not { } name)
        {
            return null;
        }

        return new PlayniteGame
        {
            Id = id,
            Name = name.Trim(),
            PluginId = Guid(d, "PluginId") ?? System.Guid.Empty,
            GameId = Text(d, "GameId"),
            InstallDirectory = Text(d, "InstallDirectory"),
            IsInstalled = Bool(d, "IsInstalled"),
            Hidden = Bool(d, "Hidden"),
            Favorite = Bool(d, "Favorite"),
            Playtime = TimeSpan.FromSeconds(Math.Max(0, Number(d, "Playtime") ?? 0)),
            LastActivity = Date(d, "LastActivity"),
            Added = Date(d, "Added"),
            PlatformIds = Guids(d, "PlatformIds"),
            CategoryIds = Guids(d, "CategoryIds"),
            SourceId = Guid(d, "SourceId") ?? System.Guid.Empty,
            Actions = [.. List(d, "GameActions").OfType<Dictionary<string, object?>>().Select(ToAction)],
            Roms = [.. List(d, "Roms").OfType<Dictionary<string, object?>>().Select(r => new PlayniteRom(Text(r, "Name"), Text(r, "Path")))],
        };
    }

    private static PlayniteAction ToAction(Dictionary<string, object?> d) => new(
        ActionType(d.GetValueOrDefault("Type")),
        Text(d, "Path"),
        Text(d, "Arguments"),
        Text(d, "WorkingDir"),
        Bool(d, "IsPlayAction"));

    /// <summary>O LiteDB grava enums como texto ("File") por padrão; aceita também o número.</summary>
    private static PlayniteActionType ActionType(object? value) => value switch
    {
        string text when Enum.TryParse(text, ignoreCase: true, out PlayniteActionType type) => type,
        string text when string.Equals(text, "URL", StringComparison.OrdinalIgnoreCase) => PlayniteActionType.Url,
        int number when Enum.IsDefined((PlayniteActionType)number) => (PlayniteActionType)number,
        long number when number is >= 0 and <= 3 => (PlayniteActionType)number,
        _ => PlayniteActionType.Script, // desconhecido: não é importado
    };

    private static Guid? Id(Dictionary<string, object?> d) => Guid(d, "_id") ?? Guid(d, "Id");

    private static Guid? Guid(Dictionary<string, object?> d, string key) => d.GetValueOrDefault(key) switch
    {
        Guid g when g != System.Guid.Empty => g,
        string s when System.Guid.TryParse(s, out var g) && g != System.Guid.Empty => g,
        _ => null,
    };

    private static List<Guid> Guids(Dictionary<string, object?> d, string key) =>
        [.. List(d, key).Select(v => v switch
        {
            Guid g => g,
            string s when System.Guid.TryParse(s, out var g) => g,
            _ => System.Guid.Empty,
        }).Where(g => g != System.Guid.Empty)];

    private static IEnumerable<object?> List(Dictionary<string, object?> d, string key) =>
        d.GetValueOrDefault(key) as IEnumerable<object?> ?? [];

    private static string? Text(Dictionary<string, object?> d, string key) =>
        d.GetValueOrDefault(key) is string { Length: > 0 } s && !string.IsNullOrWhiteSpace(s) ? s : null;

    private static bool Bool(Dictionary<string, object?> d, string key) => d.GetValueOrDefault(key) is true;

    private static double? Number(Dictionary<string, object?> d, string key) => d.GetValueOrDefault(key) switch
    {
        int i => i,
        long l => l,
        double x => x,
        string s when double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out double x) => x,
        _ => null,
    };

    private static DateTimeOffset? Date(Dictionary<string, object?> d, string key) => d.GetValueOrDefault(key) switch
    {
        DateTime date when date > DateTime.MinValue && date < DateTime.MaxValue => new DateTimeOffset(DateTime.SpecifyKind(date, DateTimeKind.Utc)),
        _ => null,
    };
}
