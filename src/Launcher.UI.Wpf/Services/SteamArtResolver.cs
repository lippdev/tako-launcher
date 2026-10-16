using System.IO;
using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Endereços das artes de um jogo da Steam (<c>steam-art://570/cover</c> ou <c>/hero</c>), do melhor ao pior:
/// arquivos 2x informados pela API pública da loja (jogos novos guardam as artes num caminho com hash), o caminho
/// antigo da CDN e, sem rede, o cache local da própria Steam (<c>appcache/librarycache</c>).
/// </summary>
public sealed partial class SteamArtResolver(HttpClient http, ILogger<SteamArtResolver> logger)
{
    private const string AssetBase = "https://shared.akamai.steamstatic.com/store_item_assets/";
    private const string LegacyBase = "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/";
    private readonly Dictionary<int, Task<JsonElement?>> _assets = [];

    /// <summary>Pasta da Steam (para o cache local); nula se a Steam não estiver instalada.</summary>
    public string? SteamRoot { get; set; }

    public async Task<IReadOnlyList<Uri>> ResolveAsync(Uri uri)
    {
        if (!int.TryParse(uri.Host, out int appId))
        {
            return [];
        }

        string kind = uri.AbsolutePath.Trim('/');
        if (kind is "logo" or "icon")
        {
            return await LogoOrIconAsync(appId, kind == "icon").ConfigureAwait(false);
        }

        bool hero = kind == "hero";
        var candidates = new List<Uri>();
        if (await AssetsAsync(appId).ConfigureAwait(false) is { } assets
            && assets.TryGetProperty("asset_url_format", out var format))
        {
            string[] keys = hero ? ["library_hero_2x", "library_hero"] : ["library_capsule_2x", "library_capsule"];
            foreach (string key in keys)
            {
                if (assets.TryGetProperty(key, out var file) && file.GetString() is { Length: > 0 } name)
                {
                    candidates.Add(new Uri(AssetBase + format.GetString()!.Replace("${FILENAME}", name, StringComparison.Ordinal)));
                }
            }
        }

        candidates.Add(new Uri($"{LegacyBase}{appId}/{(hero ? "library_hero_2x.jpg" : "library_600x900_2x.jpg")}"));
        candidates.AddRange(LocalFiles(appId, hero).Select(p => new Uri(p)));
        return candidates;
    }

    /// <summary>
    /// Logo (ADR-0023): <c>library_logo_2x</c>/<c>library_logo</c> da API, o caminho antigo <c>logo.png</c> e o cache da
    /// Steam. Ícone: o <c>community_icon</c> da API na CDN da comunidade e o cache da Steam.
    /// </summary>
    private async Task<IReadOnlyList<Uri>> LogoOrIconAsync(int appId, bool icon)
    {
        var candidates = new List<Uri>();
        var assets = await AssetsAsync(appId).ConfigureAwait(false);
        if (icon)
        {
            if (assets is { } a && a.TryGetProperty("community_icon", out var hash) && hash.GetString() is { Length: > 0 } iconHash)
            {
                candidates.Add(new Uri($"https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/{appId}/{iconHash}.jpg"));
            }

            candidates.AddRange(LocalNamed(appId, ["icon.jpg", "icon.png"]).Select(p => new Uri(p)));
            return candidates;
        }

        if (assets is { } logo && logo.TryGetProperty("asset_url_format", out var format))
        {
            foreach (string key in (string[])["library_logo_2x", "library_logo"])
            {
                if (logo.TryGetProperty(key, out var file) && file.GetString() is { Length: > 0 } name)
                {
                    candidates.Add(new Uri(AssetBase + format.GetString()!.Replace("${FILENAME}", name, StringComparison.Ordinal)));
                }
            }
        }

        candidates.Add(new Uri($"{LegacyBase}{appId}/logo_2x.png"));
        candidates.Add(new Uri($"{LegacyBase}{appId}/logo.png"));
        candidates.AddRange(LocalNamed(appId, ["logo.png"]).Select(p => new Uri(p)));
        return candidates;
    }

    private IEnumerable<string> LocalNamed(int appId, string[] names)
    {
        if (SteamRoot is null)
        {
            yield break;
        }

        string cache = Path.Combine(SteamRoot, "appcache", "librarycache");
        string folder = Path.Combine(cache, appId.ToString(System.Globalization.CultureInfo.InvariantCulture));
        foreach (string name in names)
        {
            if (Directory.Exists(folder))
            {
                foreach (string file in Directory.EnumerateFiles(folder, name, SearchOption.AllDirectories))
                {
                    yield return file;
                }
            }

            string flat = Path.Combine(cache, $"{appId}_{name}");
            if (File.Exists(flat))
            {
                yield return flat;
            }
        }
    }

    /// <summary>Uma consulta por jogo, guardada durante a sessão.</summary>
    private Task<JsonElement?> AssetsAsync(int appId)
    {
        lock (_assets)
        {
            if (!_assets.TryGetValue(appId, out var task))
            {
                task = FetchAssetsAsync(appId);
                _assets[appId] = task;
            }

            return task;
        }
    }

    private async Task<JsonElement?> FetchAssetsAsync(int appId)
    {
        string input = $$$"""{"ids":[{"appid":{{{appId}}}}],"context":{"language":"brazilian","country_code":"BR"},"data_request":{"include_assets":true}}""";
        var url = new Uri("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/?input_json=" + Uri.EscapeDataString(input));
        try
        {
            using var response = await http.GetAsync(url).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            using var document = JsonDocument.Parse(await response.Content.ReadAsStringAsync().ConfigureAwait(false));
            if (document.RootElement.TryGetProperty("response", out var body)
                && body.TryGetProperty("store_items", out var items)
                && items.GetArrayLength() > 0
                && items[0].TryGetProperty("assets", out var assets))
            {
                return assets.Clone();
            }
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException or JsonException)
        {
            LogLookupFailed(logger, appId, ex.Message);
        }

        return null;
    }

    /// <summary>Cache da Steam: formato novo (<c>librarycache/570/…/library_600x900.jpg</c>) e antigo (<c>570_library_600x900.jpg</c>).</summary>
    private IEnumerable<string> LocalFiles(int appId, bool hero)
    {
        if (SteamRoot is null)
        {
            yield break;
        }

        string cache = Path.Combine(SteamRoot, "appcache", "librarycache");
        string[] names = hero ? ["library_hero.jpg"] : ["library_600x900.jpg", "library_capsule.jpg"];
        string folder = Path.Combine(cache, appId.ToString(System.Globalization.CultureInfo.InvariantCulture));
        foreach (string name in names)
        {
            if (Directory.Exists(folder))
            {
                foreach (string file in Directory.EnumerateFiles(folder, name, SearchOption.AllDirectories))
                {
                    yield return file;
                }
            }

            string flat = Path.Combine(cache, $"{appId}_{name}");
            if (File.Exists(flat))
            {
                yield return flat;
            }
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Artes da Steam indisponíveis pela API para {AppId}: {Reason}")]
    private static partial void LogLookupFailed(ILogger logger, int appId, string reason);
}
