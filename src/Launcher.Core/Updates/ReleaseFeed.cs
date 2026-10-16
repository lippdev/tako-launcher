using System.Buffers;
using System.Security.Cryptography;
using System.Text.Json;

namespace Launcher.Core.Updates;

/// <summary>Atualização encontrada nas releases do GitHub. Sem arquivo confiável (hash), só a página é oferecida.</summary>
public sealed record UpdateInfo(string Version, string Name, string Notes, string PageUrl, string? AssetUrl, string? AssetName, string? AssetDigest, bool IsPrerelease)
{
    public bool CanInstall => AssetUrl is not null && AssetName is not null && AssetDigest is not null;
}

/// <summary>Escolhe a atualização na lista de releases do GitHub (puro, testável). Mesmas regras do Console Mode.</summary>
public static class ReleaseFeed
{
    public const string Repository = "lippdev/tako-launcher";
    public const string SetupAssetPrefix = "Tako-Setup";
    public const string PortableAssetPrefix = "Tako-Portable";

    /// <summary>
    /// Versões de teste (alpha/beta) entram quando o usuário optou por elas ou quando já roda uma versão de teste.
    /// Desligar a opção nunca rebaixa: uma versão de teste continua recebendo testes até sair uma estável mais nova.
    /// </summary>
    public static bool IncludePrereleases(bool currentIsPrerelease, bool optedIn) => currentIsPrerelease || optedIn;

    /// <summary>Release publicada mais nova acima da versão atual, com o arquivo do tipo de instalação, ou nulo.</summary>
    public static UpdateInfo? Pick(JsonElement releases, SemVer current, bool optedIntoPrereleases, bool installed)
    {
        bool allowPrerelease = IncludePrereleases(current.IsPrerelease, optedIntoPrereleases);
        UpdateInfo? best = null;
        SemVer? bestVersion = null;
        foreach (var release in releases.EnumerateArray())
        {
            if (release.GetProperty("draft").GetBoolean())
            {
                continue;
            }

            bool prerelease = release.GetProperty("prerelease").GetBoolean();
            if (prerelease && !allowPrerelease)
            {
                continue;
            }

            string tag = release.GetProperty("tag_name").GetString() ?? string.Empty;
            string name = release.TryGetProperty("name", out var n) ? n.GetString() ?? string.Empty : string.Empty;
            var version = SemVer.TryFind(tag) ?? SemVer.TryFind(name);
            if (version is null || version.CompareTo(current) <= 0 || (bestVersion is not null && version.CompareTo(bestVersion) <= 0))
            {
                continue;
            }

            var (url, assetName, digest) = PickAsset(release, installed);
            best = new UpdateInfo(
                version.ToString(),
                string.IsNullOrWhiteSpace(name) ? tag : name,
                release.TryGetProperty("body", out var body) ? body.GetString() ?? string.Empty : string.Empty,
                release.GetProperty("html_url").GetString() ?? $"https://github.com/{Repository}/releases",
                url,
                assetName,
                digest,
                prerelease);
            bestVersion = version;
        }

        return best;
    }

    /// <summary>Instalado recebe o instalador; portátil, o .exe portátil. Sem o SHA-256 do GitHub, nada é baixado.</summary>
    private static (string? Url, string? Name, string? Digest) PickAsset(JsonElement release, bool installed)
    {
        if (!release.TryGetProperty("assets", out var assets))
        {
            return (null, null, null);
        }

        string prefix = installed ? SetupAssetPrefix : PortableAssetPrefix;
        foreach (var asset in assets.EnumerateArray())
        {
            string name = asset.GetProperty("name").GetString() ?? string.Empty;
            if (name.StartsWith(prefix, StringComparison.OrdinalIgnoreCase) && name.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
            {
                string? digest = asset.TryGetProperty("digest", out var d) ? d.GetString() : null;
                return IsValidSha256Digest(digest) ? (asset.GetProperty("browser_download_url").GetString(), name, digest) : (null, null, null);
            }
        }

        return (null, null, null);
    }

    /// <summary>Seção <c>## [versão]</c> de um changelog (o mesmo recorte que o workflow de release usa), ou nulo.</summary>
    public static string? ChangelogSection(string changelog, string version)
    {
        ArgumentNullException.ThrowIfNull(changelog);
        var match = System.Text.RegularExpressions.Regex.Match(
            changelog,
            $@"(?ms)^## \[{System.Text.RegularExpressions.Regex.Escape(version)}\]\r?\n(?<notes>.*?)(?=^## |\z)");
        return match.Success ? match.Groups["notes"].Value.Trim() : null;
    }

    /// <summary>Primeiro item das notas ("- …"), sem o "(#NN)" do fim, para o aviso de atualização.</summary>
    public static string? FirstNote(string notes)
    {
        ArgumentNullException.ThrowIfNull(notes);
        string? line = notes.Split('\n').Select(l => l.Trim()).FirstOrDefault(l => l.StartsWith("- ", StringComparison.Ordinal));
        if (line is null)
        {
            return null;
        }

        line = line[2..].Replace("**", string.Empty, StringComparison.Ordinal);
        int pr = line.LastIndexOf(" (#", StringComparison.Ordinal);
        return pr > 0 ? line[..pr] : line;
    }

    private const string DigestPrefix = "sha256:";
    private static readonly SearchValues<char> Hex = SearchValues.Create("0123456789abcdefABCDEF");

    public static bool IsValidSha256Digest(string? digest) =>
        digest is { Length: 71 }
        && digest.StartsWith(DigestPrefix, StringComparison.OrdinalIgnoreCase)
        && digest.AsSpan(DigestPrefix.Length).IndexOfAnyExcept(Hex) < 0;

    /// <summary>Confere o arquivo baixado com o SHA-256 publicado pelo GitHub (comparação em tempo constante).</summary>
    public static async Task<bool> VerifyFileAsync(string path, string? expectedDigest, CancellationToken cancellationToken = default)
    {
        if (!IsValidSha256Digest(expectedDigest))
        {
            return false;
        }

        await using var file = File.OpenRead(path);
        byte[] actual = await SHA256.HashDataAsync(file, cancellationToken).ConfigureAwait(false);
        byte[] expected = Convert.FromHexString(expectedDigest![DigestPrefix.Length..]);
        return CryptographicOperations.FixedTimeEquals(actual, expected);
    }
}
