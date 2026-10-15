using System.Text.Json;

namespace Launcher.Core.Library.Epic;

/// <summary>Jogo instalado pela Epic Games Launcher (um arquivo <c>.item</c> da pasta de manifests).</summary>
public sealed record EpicInstall(
    string AppName,
    string DisplayName,
    string InstallLocation,
    string? LaunchExecutable,
    string CatalogNamespace,
    string CatalogItemId,
    long InstallSize);

/// <summary>
/// Formatos de arquivo da Epic Games Launcher, lidos sem a Epic aberta: os manifests <c>*.item</c> (JSON, um por jogo
/// instalado) em <c>%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests</c> e a lista <c>LauncherInstalled.dat</c>
/// (JSON) em <c>%ProgramData%\Epic\UnrealEngineLauncher</c>.
/// </summary>
public static class EpicManifests
{
    private static readonly JsonDocumentOptions Options = new() { AllowTrailingCommas = true, CommentHandling = JsonCommentHandling.Skip };

    /// <summary>
    /// Um manifest <c>.item</c>; nulo quando não é um jogo jogável: instalação incompleta, DLC (o jogo principal é outro),
    /// plugin ou engine (sem a categoria "games") ou campos essenciais faltando. JSON inválido lança <see cref="JsonException"/>.
    /// </summary>
    public static EpicInstall? ParseManifest(string json)
    {
        ArgumentNullException.ThrowIfNull(json);
        using var document = JsonDocument.Parse(json, Options);
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object || Bool(root, "bIsIncompleteInstall"))
        {
            return null;
        }

        string? appName = Text(root, "AppName");
        string? displayName = Text(root, "DisplayName");
        string? location = Text(root, "InstallLocation");
        string? ns = Text(root, "CatalogNamespace");
        string? itemId = Text(root, "CatalogItemId");
        if (appName is null || displayName is null || location is null || ns is null || itemId is null)
        {
            return null;
        }

        // DLC: aponta para o jogo principal, que tem o próprio manifest.
        if (Text(root, "MainGameAppName") is { } main && !string.Equals(main, appName, StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        // Plugins do Unreal Engine e o próprio engine aparecem como "itens" sem a categoria "games".
        if (root.TryGetProperty("AppCategories", out var categories) && categories.ValueKind == JsonValueKind.Array
            && !categories.EnumerateArray().Any(c => c.ValueKind == JsonValueKind.String && string.Equals(c.GetString(), "games", StringComparison.OrdinalIgnoreCase)))
        {
            return null;
        }

        long size = root.TryGetProperty("InstallSize", out var sizeElement) && sizeElement.ValueKind == JsonValueKind.Number && sizeElement.TryGetInt64(out long bytes) ? bytes : 0;
        return new EpicInstall(appName, displayName.Trim(), location, Text(root, "LaunchExecutable"), ns, itemId, size);
    }

    /// <summary><c>LauncherInstalled.dat</c>: pasta de instalação por AppName (a Epic atualiza ao mover um jogo).</summary>
    public static IReadOnlyDictionary<string, string> ParseInstalledList(string json)
    {
        ArgumentNullException.ThrowIfNull(json);
        var result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        using var document = JsonDocument.Parse(json, Options);
        if (document.RootElement.ValueKind != JsonValueKind.Object
            || !document.RootElement.TryGetProperty("InstallationList", out var list)
            || list.ValueKind != JsonValueKind.Array)
        {
            return result;
        }

        foreach (var entry in list.EnumerateArray().Where(e => e.ValueKind == JsonValueKind.Object))
        {
            if (Text(entry, "AppName") is { } app && Text(entry, "InstallLocation") is { } location)
            {
                result[app] = location;
            }
        }

        return result;
    }

    /// <summary>
    /// Abrir pelo protocolo oficial (respeita login, DRM e atualizações da Epic):
    /// <c>com.epicgames.launcher://apps/&lt;Namespace&gt;%3A&lt;CatalogItemId&gt;%3A&lt;AppName&gt;?action=launch&amp;silent=true</c>.
    /// </summary>
    public static string LaunchUri(EpicInstall install)
    {
        ArgumentNullException.ThrowIfNull(install);
        return $"com.epicgames.launcher://apps/{Uri.EscapeDataString(install.CatalogNamespace)}%3A{Uri.EscapeDataString(install.CatalogItemId)}%3A{Uri.EscapeDataString(install.AppName)}?action=launch&silent=true";
    }

    private static string? Text(JsonElement element, string name) =>
        element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String && value.GetString() is { Length: > 0 } text && !string.IsNullOrWhiteSpace(text)
            ? text
            : null;

    private static bool Bool(JsonElement element, string name) =>
        element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.True;
}
