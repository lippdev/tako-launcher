using System.Text.Json;

namespace Launcher.Core.Library.Gog;

/// <summary>
/// Jogo do GOG gravado no registro pelo instalador (Galaxy ou instalador offline) em
/// <c>HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\&lt;id&gt;</c>.
/// </summary>
public sealed record GogRegistryEntry(
    string GameId,
    string Name,
    string Path,
    string? Exe = null,
    string? LaunchParam = null,
    string? WorkingDir = null,
    string? LaunchCommand = null,
    string? DependsOn = null);

/// <summary>Tarefa principal de <c>goggame-&lt;id&gt;.info</c> (na pasta do jogo).</summary>
public sealed record GogInfo(string GameId, string? RootGameId, string? Name, string? PlayPath, string? PlayArguments, string? PlayWorkingDir);

/// <summary>Como abrir um jogo do GOG: executável e argumentos já resolvidos.</summary>
public sealed record GogLaunch(string Target, string? Arguments, string? WorkingDirectory, bool ViaGalaxy);

/// <summary>Regras puras dos jogos do GOG (registro, <c>.info</c> e como abrir).</summary>
public static class GogGames
{
    private static readonly JsonDocumentOptions Options = new() { AllowTrailingCommas = true, CommentHandling = JsonCommentHandling.Skip };

    /// <summary>DLC (depende de outro jogo) ou entrada sem pasta: não vira item da biblioteca.</summary>
    public static bool IsPlayable(GogRegistryEntry entry, GogInfo? info)
    {
        ArgumentNullException.ThrowIfNull(entry);
        bool isDlc = !string.IsNullOrWhiteSpace(entry.DependsOn)
            || (info?.RootGameId is { Length: > 0 } root && !string.Equals(root, info.GameId, StringComparison.Ordinal));
        return !isDlc && !string.IsNullOrWhiteSpace(entry.Path) && !string.IsNullOrWhiteSpace(entry.Name);
    }

    /// <summary><c>goggame-&lt;id&gt;.info</c>: a tarefa de jogo principal (<c>isPrimary</c>, tipo FileTask). JSON inválido lança <see cref="JsonException"/>.</summary>
    public static GogInfo? ParseInfo(string json)
    {
        ArgumentNullException.ThrowIfNull(json);
        using var document = JsonDocument.Parse(json, Options);
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object || Text(root, "gameId") is not { } gameId)
        {
            return null;
        }

        JsonElement? task = null;
        if (root.TryGetProperty("playTasks", out var tasks) && tasks.ValueKind == JsonValueKind.Array)
        {
            var files = tasks.EnumerateArray()
                .Where(t => t.ValueKind == JsonValueKind.Object && string.Equals(Text(t, "type"), "FileTask", StringComparison.OrdinalIgnoreCase) && Text(t, "path") is not null)
                .ToList();
            task = files.Cast<JsonElement?>().FirstOrDefault(t => t!.Value.TryGetProperty("isPrimary", out var p) && p.ValueKind == JsonValueKind.True)
                ?? files.Cast<JsonElement?>().FirstOrDefault();
        }

        return new GogInfo(
            gameId,
            Text(root, "rootGameId"),
            Text(root, "name"),
            task is { } t1 ? Text(t1, "path") : null,
            task is { } t2 ? Text(t2, "arguments") : null,
            task is { } t3 ? Text(t3, "workingDir") : null);
    }

    /// <summary>
    /// Pelo GOG Galaxy quando instalado (<c>GalaxyClient.exe /command=runGame /gameId=&lt;id&gt; /path="&lt;pasta&gt;"</c>,
    /// que mantém nuvem e conquistas); sem ele, o executável do próprio jogo (jogos do GOG não têm DRM). Nulo quando não
    /// há executável conhecido nem Galaxy.
    /// </summary>
    public static GogLaunch? Launch(GogRegistryEntry entry, GogInfo? info, string? galaxyExe)
    {
        ArgumentNullException.ThrowIfNull(entry);
        if (!string.IsNullOrWhiteSpace(galaxyExe))
        {
            return new GogLaunch(galaxyExe, $"/command=runGame /gameId={entry.GameId} /path=\"{entry.Path.TrimEnd('\\', '/')}\"", System.IO.Path.GetDirectoryName(galaxyExe), ViaGalaxy: true);
        }

        if (!string.IsNullOrWhiteSpace(entry.Exe))
        {
            return new GogLaunch(entry.Exe, Blank(entry.LaunchParam), Blank(entry.WorkingDir) ?? entry.Path, ViaGalaxy: false);
        }

        if (info?.PlayPath is { } relative)
        {
            string? workingDir = info.PlayWorkingDir is { } wd ? Combine(entry.Path, wd) : null;
            return new GogLaunch(Combine(entry.Path, relative), Blank(info.PlayArguments), workingDir ?? entry.Path, ViaGalaxy: false);
        }

        if (SplitCommand(entry.LaunchCommand) is ({ } exe, var arguments))
        {
            return new GogLaunch(exe, arguments, Blank(entry.WorkingDir) ?? entry.Path, ViaGalaxy: false);
        }

        return null;
    }

    /// <summary>Linha de comando (<c>"C:\Jogo\jogo.exe" -arg</c>) em executável e argumentos.</summary>
    public static (string? Exe, string? Arguments) SplitCommand(string? command)
    {
        string text = command?.Trim() ?? string.Empty;
        if (text.Length == 0)
        {
            return (null, null);
        }

        if (text[0] == '"')
        {
            int end = text.IndexOf('"', 1);
            return end < 0 ? (text.Trim('"'), null) : (text[1..end], Blank(text[(end + 1)..].Trim()));
        }

        int exeEnd = text.IndexOf(".exe", StringComparison.OrdinalIgnoreCase);
        return exeEnd < 0 ? (text, null) : (text[..(exeEnd + 4)], Blank(text[(exeEnd + 4)..].Trim()));
    }

    private static string Combine(string folder, string relative) =>
        System.IO.Path.IsPathRooted(relative) || relative.Length > 1 && relative[1] == ':'
            ? relative
            : folder.TrimEnd('\\', '/') + "\\" + relative.TrimStart('\\', '/');

    private static string? Blank(string? text) => string.IsNullOrWhiteSpace(text) ? null : text;

    private static string? Text(JsonElement element, string name) =>
        !element.TryGetProperty(name, out var value) ? null
        : value.ValueKind == JsonValueKind.Number ? value.GetRawText() // ids às vezes vêm como número
        : value.ValueKind == JsonValueKind.String && !string.IsNullOrWhiteSpace(value.GetString()) ? value.GetString()
        : null;
}
