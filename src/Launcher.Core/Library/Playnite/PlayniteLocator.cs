using System.Text.Json;

namespace Launcher.Core.Library.Playnite;

/// <summary>Onde fica a biblioteca do Playnite instalado: o <c>DatabasePath</c> do <c>config.json</c> ou a pasta padrão.</summary>
public static class PlayniteLocator
{
    /// <summary>
    /// Pastas candidatas, na ordem: a do <c>config.json</c> (variáveis de ambiente resolvidas) e
    /// <c>%APPDATA%\Playnite\library</c>. <paramref name="configJson"/> inválido é ignorado.
    /// </summary>
    public static IReadOnlyList<string> Candidates(string appData, string? configJson, Func<string, string>? expand = null)
    {
        ArgumentNullException.ThrowIfNull(appData);
        expand ??= Environment.ExpandEnvironmentVariables;
        var folders = new List<string>();
        if (!string.IsNullOrWhiteSpace(configJson))
        {
            try
            {
                using var document = JsonDocument.Parse(configJson);
                if (document.RootElement.ValueKind == JsonValueKind.Object
                    && document.RootElement.TryGetProperty("DatabasePath", out var value)
                    && value.ValueKind == JsonValueKind.String
                    && value.GetString() is { Length: > 0 } path
                    && !path.Contains("{PlayniteDir}", StringComparison.OrdinalIgnoreCase))
                {
                    folders.Add(expand(path));
                }
            }
            catch (JsonException)
            {
                // config.json sendo gravado ou corrompido: fica a pasta padrão
            }
        }

        folders.Add(Path.Combine(appData, "Playnite", "library"));
        return [.. folders.Distinct(StringComparer.OrdinalIgnoreCase)];
    }

    /// <summary>A primeira pasta candidata que tem o <c>games.db</c>; nula se o Playnite não está neste PC.</summary>
    public static string? Find(string appData)
    {
        ArgumentNullException.ThrowIfNull(appData);
        string config = Path.Combine(appData, "Playnite", "config.json");
        string? json;
        try
        {
            json = File.Exists(config) ? File.ReadAllText(config) : null;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            json = null;
        }

        return Candidates(appData, json).FirstOrDefault(folder => File.Exists(Path.Combine(folder, "games.db")));
    }
}
