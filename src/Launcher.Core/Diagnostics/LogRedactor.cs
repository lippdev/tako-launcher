using System.Text.RegularExpressions;

namespace Launcher.Core.Diagnostics;

/// <summary>
/// Remove dados sensíveis antes de gravar o log (MVP-033): a pasta do perfil vira <c>%USERPROFILE%</c>, URLs perdem a
/// query (onde ficam tokens e chaves) e pares como <c>token=...</c> ou <c>Bearer ...</c> são mascarados.
/// </summary>
public sealed partial class LogRedactor(string? userProfile)
{
    public const string Mask = "[removido]";

    public static LogRedactor Default { get; } = new(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile));

    private readonly string? _userProfile = string.IsNullOrWhiteSpace(userProfile) ? null : userProfile.TrimEnd('\\', '/');

    public string Redact(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        if (_userProfile is not null)
        {
            text = text.Replace(_userProfile, "%USERPROFILE%", StringComparison.OrdinalIgnoreCase);
            string forward = _userProfile.Replace('\\', '/');
            if (forward != _userProfile)
            {
                text = text.Replace(forward, "%USERPROFILE%", StringComparison.OrdinalIgnoreCase);
            }
        }

        text = UrlQuery().Replace(text, m => m.Groups["base"].Value + "?" + Mask);
        text = Bearer().Replace(text, m => m.Groups["scheme"].Value + " " + Mask);
        return SecretPair().Replace(text, m => m.Groups["key"].Value + m.Groups["sep"].Value + Mask);
    }

    // URL com query: mantém esquema, host e caminho; a query inteira sai.
    [GeneratedRegex(@"(?<base>\b[a-z][a-z0-9+.\-]*://[^\s?#""'<>]+)\?[^\s#""'<>]*", RegexOptions.IgnoreCase | RegexOptions.CultureInvariant)]
    private static partial Regex UrlQuery();

    [GeneratedRegex(@"(?<scheme>\b(?:Bearer|Basic))\s+[A-Za-z0-9\-._~+/]+=*", RegexOptions.IgnoreCase | RegexOptions.CultureInvariant)]
    private static partial Regex Bearer();

    [GeneratedRegex(@"(?<key>\b(?:access_token|refresh_token|id_token|token|api[_-]?key|apikey|client_secret|secret|password|passwd|pwd|authorization|sig|signature)\b)(?<sep>\s*[=:]\s*)[^\s&;,""']+", RegexOptions.IgnoreCase | RegexOptions.CultureInvariant)]
    private static partial Regex SecretPair();
}
