using System.IO;

namespace Launcher.Tests;

/// <summary>
/// Biblioteca do Playnite fictícia (STORE-009) gravada no formato do LiteDB 4: um jogo adicionado à mão, um da Steam,
/// um da Epic que o Tako não tem, um por script, um não instalado, uma ROM de SNES, um link e um com descrição longa
/// (página de extensão).
/// </summary>
public static class PlayniteSample
{
    public static readonly Guid SteamPlugin = new("cb91dfc9-b977-43bf-8e70-55f46e410fab");
    public static readonly Guid EpicPlugin = new("00000002-dbd1-46c6-b5d0-b1ba559d10e4");

    public static readonly Guid Lanterna = new("11111111-1111-1111-1111-111111111111");
    public static readonly Guid Farol = new("22222222-2222-2222-2222-222222222222");
    public static readonly Guid Corrida = new("33333333-3333-3333-3333-333333333333");
    public static readonly Guid Roteiro = new("44444444-4444-4444-4444-444444444444");
    public static readonly Guid Ausente = new("55555555-5555-5555-5555-555555555555");
    public static readonly Guid Cartucho = new("66666666-6666-6666-6666-666666666666");
    public static readonly Guid Navegador = new("77777777-7777-7777-7777-777777777777");
    public static readonly Guid Longo = new("88888888-8888-8888-8888-888888888888");

    public static readonly Guid Pc = new("a0000000-0000-0000-0000-000000000001");
    public static readonly Guid Snes = new("a0000000-0000-0000-0000-000000000002");
    public static readonly Guid Sofa = new("b0000000-0000-0000-0000-000000000001");
    public static readonly Guid Zerados = new("b0000000-0000-0000-0000-000000000002");

    /// <summary>Grava <c>games.db</c>, <c>platforms.db</c>, <c>categories.db</c> e <c>sources.db</c> na pasta.</summary>
    public static void Write(string folder)
    {
        Directory.CreateDirectory(folder);
        File.WriteAllBytes(Path.Combine(folder, "games.db"), LiteDbV7Writer.Write(new Dictionary<string, IReadOnlyList<Dictionary<string, object?>>> { ["Game"] = Games() }));
        File.WriteAllBytes(Path.Combine(folder, "platforms.db"), LiteDbV7Writer.Write(new Dictionary<string, IReadOnlyList<Dictionary<string, object?>>>
        {
            ["Platform"] =
            [
                new() { ["_id"] = Pc, ["Name"] = "PC (Windows)", ["SpecificationId"] = "pc_windows" },
                new() { ["_id"] = Snes, ["Name"] = "Super Nintendo", ["SpecificationId"] = "nintendo_super_nes" },
            ],
        }));
        File.WriteAllBytes(Path.Combine(folder, "categories.db"), LiteDbV7Writer.Write(new Dictionary<string, IReadOnlyList<Dictionary<string, object?>>>
        {
            ["Category"] =
            [
                new() { ["_id"] = Sofa, ["Name"] = "Jogar no sofá" },
                new() { ["_id"] = Zerados, ["Name"] = "Zerados" },
            ],
        }));
        File.WriteAllBytes(Path.Combine(folder, "sources.db"), LiteDbV7Writer.Write(new Dictionary<string, IReadOnlyList<Dictionary<string, object?>>>
        {
            ["GameSource"] = [new() { ["_id"] = new Guid("c0000000-0000-0000-0000-000000000001"), ["Name"] = "Steam" }],
        }));
    }

    public static List<Dictionary<string, object?>> Games() =>
    [
        Game(Lanterna, "Lanterna Distante", installDir: @"C:\Jogos\Lanterna", playtime: 7200L, favorite: true, platform: Pc, categories: [Sofa, Zerados],
            action: Action("File", @"{InstallDir}\lanterna.exe", arguments: "-tela-cheia", workingDir: "{InstallDir}"),
            added: new DateTime(2025, 3, 1, 12, 0, 0, DateTimeKind.Utc), lastActivity: new DateTime(2026, 9, 30, 20, 0, 0, DateTimeKind.Utc)),
        Game(Farol, "Farol", plugin: SteamPlugin, gameId: "7", installDir: @"C:\Steam\steamapps\common\Farol", playtime: 600, favorite: true, categories: [Sofa]),
        Game(Corrida, "Corrida Noturna", plugin: EpicPlugin, gameId: "CorridaNoturna", installDir: @"D:\Epic\Corrida"),
        Game(Roteiro, "Roteiro", installDir: @"C:\Jogos\Roteiro", action: Action("Script", null)),
        Game(Ausente, "Ausente", installDir: null, installed: false, action: Action("File", @"C:\Jogos\Ausente\ausente.exe")),
        Game(Cartucho, "Cartucho Azul", installDir: @"D:\ROMs\SNES", platform: Snes, action: Action("Emulator", null),
            roms: [new Dictionary<string, object?> { ["Name"] = "Cartucho Azul", ["Path"] = @"{InstallDir}\azul.sfc" }]),
        Game(Navegador, "Jogo no Navegador", installDir: null, action: Action("URL", "https://example.com/jogo")),
        Game(Longo, "Descrição Longa", installDir: @"C:\Jogos\Longo", action: Action("File", @"C:\Jogos\Longo\longo.exe"), description: new string('x', 9000)),
    ];

    private static Dictionary<string, object?> Action(string type, string? path, string? arguments = null, string? workingDir = null) => new()
    {
        ["Type"] = type,
        ["Path"] = path,
        ["Arguments"] = arguments,
        ["WorkingDir"] = workingDir,
        ["IsPlayAction"] = true,
        ["Name"] = "Jogar",
    };

    private static Dictionary<string, object?> Game(
        Guid id,
        string name,
        Guid? plugin = null,
        string? gameId = null,
        string? installDir = null,
        bool installed = true,
        object? playtime = null,
        bool favorite = false,
        Guid? platform = null,
        Guid[]? categories = null,
        Dictionary<string, object?>? action = null,
        Dictionary<string, object?>[]? roms = null,
        DateTime? added = null,
        DateTime? lastActivity = null,
        string? description = null) => new()
        {
            ["_id"] = id,
            ["Name"] = name,
            ["PluginId"] = plugin ?? Guid.Empty,
            ["GameId"] = gameId,
            ["InstallDirectory"] = installDir,
            ["IsInstalled"] = installed,
            ["Hidden"] = false,
            ["Favorite"] = favorite,
            ["Playtime"] = playtime ?? 0L,
            ["PlatformIds"] = platform is Guid p ? new List<object?> { p } : null,
            ["CategoryIds"] = categories?.Cast<object?>().ToList(),
            ["GameActions"] = action is null ? null : new List<object?> { action },
            ["Roms"] = roms?.Cast<object?>().ToList(),
            ["Added"] = added,
            ["LastActivity"] = lastActivity,
            ["Description"] = description,
        };
}
