namespace Launcher.Core.Themes;

/// <summary>Por que uma origem de imagem ou mídia pedida por um tema foi bloqueada.</summary>
public enum ThemeSourceBlock
{
    /// <summary>Permitida.</summary>
    None,

    /// <summary>Caminho de rede (UNC <c>\\servidor\share</c>, <c>file://servidor/...</c>): no Windows vazaria o hash NTLM.</summary>
    Network,

    /// <summary>Esquema fora da lista (<c>http</c>, <c>https</c>, <c>ftp</c>, <c>pack</c> de outro pacote etc.).</summary>
    External,

    /// <summary>Arquivo local fora da pasta do tema, do tema padrão e do cache de artes.</summary>
    OutsideTheme,

    /// <summary>Caminho que passa por link simbólico ou junção abaixo da pasta permitida.</summary>
    Link,

    /// <summary>Texto que não é URI nem caminho válido.</summary>
    Invalid,
}

/// <summary>Decisão sobre uma origem: a URI a carregar (absoluta quando foi resolvida) ou o motivo do bloqueio.</summary>
public readonly record struct ThemeSourceDecision(Uri? Uri, ThemeSourceBlock Block)
{
    public bool IsAllowed => Block == ThemeSourceBlock.None;

    public static ThemeSourceDecision Allow(Uri uri) => new(uri, ThemeSourceBlock.None);

    public static ThemeSourceDecision Deny(ThemeSourceBlock block) => new(null, block);
}

/// <summary>
/// Origens que um tema pode carregar como imagem ou mídia em tempo de execução (MVP-016, questão aberta 8): arquivos
/// dentro da pasta do tema (ou do tema padrão, de onde vêm os arquivos herdados), o cache de artes do Tako e recursos
/// <c>pack://application</c>. Todo o resto é bloqueado: UNC, <c>http(s)</c>, <c>ftp</c>, outros esquemas e arquivos
/// fora dessas pastas. A decisão é só sobre texto até o caminho cair numa pasta permitida: um caminho de rede nunca
/// chega a tocar o sistema de arquivos (o que já abriria a conexão SMB).
/// </summary>
public sealed class ThemeSourcePolicy
{
    private const string PackApplication = "pack://application:,,,/";

    private readonly Lock _gate = new();
    private volatile Root[] _roots = [];

    /// <summary>Política do processo: o motor de temas registra as pastas, e a guarda de mídia do WPF a consulta.</summary>
    public static ThemeSourcePolicy Shared { get; } = new();

    /// <summary>Pasta de um tema carregado. <paramref name="isBase"/>: o tema padrão, de onde qualquer tema herda arquivos.</summary>
    public void AllowTheme(string directory, bool isBase = false) => Add(directory, isBase ? RootKind.BaseTheme : RootKind.Theme);

    /// <summary>Outra pasta cujos arquivos qualquer tema pode exibir (o cache de artes do Tako).</summary>
    public void AllowDirectory(string directory) => Add(directory, RootKind.Shared);

    /// <summary>
    /// Decide sobre um texto vindo de um binding do tema, resolvido como o WPF faria a partir de <paramref name="baseUri"/>
    /// (o arquivo XAML do elemento de destino, ou nulo/<c>pack://application</c> quando não há).
    /// </summary>
    public ThemeSourceDecision Evaluate(string? text, Uri? baseUri)
    {
        string value = text?.Trim() ?? string.Empty;
        if (value.Length == 0)
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.Invalid);
        }

        // \\servidor, //servidor, \\?\UNC\..., \\.\...: rede ou dispositivo, decidido antes de qualquer outra coisa.
        if (IsNetworkPrefix(value))
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.Network);
        }

        // Caminho local absoluto ("C:\\...", ou "/..." fora do Windows): direto para a regra de pastas.
        if (Path.IsPathFullyQualified(value))
        {
            return EvaluateLocalFile(value, baseUri);
        }

        if (!Uri.TryCreate(value, UriKind.RelativeOrAbsolute, out var uri))
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.Invalid);
        }

        return Evaluate(uri, baseUri);
    }

    public ThemeSourceDecision Evaluate(Uri uri, Uri? baseUri)
    {
        ArgumentNullException.ThrowIfNull(uri);
        if (!uri.IsAbsoluteUri)
        {
            string relative = uri.OriginalString.Replace('\\', '/');
            if (baseUri is null || !baseUri.IsAbsoluteUri || IsPackApplication(baseUri))
            {
                // Relativo ao pacote do próprio app: só alcança recursos embutidos no Tako.
                return relative.StartsWith("//", StringComparison.Ordinal)
                    ? ThemeSourceDecision.Deny(ThemeSourceBlock.Network)
                    : ThemeSourceDecision.Allow(uri);
            }

            if (!Uri.TryCreate(baseUri, relative, out var combined))
            {
                return ThemeSourceDecision.Deny(ThemeSourceBlock.Invalid);
            }

            uri = combined;
        }

        if (IsPackApplication(uri))
        {
            return ThemeSourceDecision.Allow(uri);
        }

        if (!uri.IsFile)
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.External);
        }

        if (uri.IsUnc || !string.IsNullOrEmpty(uri.Host))
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.Network);
        }

        return EvaluateLocalFile(uri.LocalPath, baseUri);
    }

    /// <summary>Arquivo local absoluto: precisa estar numa pasta permitida, sem link simbólico ou junção no caminho.</summary>
    public ThemeSourceDecision EvaluateLocalFile(string path, Uri? baseUri)
    {
        ArgumentNullException.ThrowIfNull(path);
        if (IsNetworkPrefix(path))
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.Network);
        }

        string full;
        try
        {
            full = Path.GetFullPath(path);
        }
        catch (Exception ex) when (ex is ArgumentException or NotSupportedException or PathTooLongException)
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.Invalid);
        }

        if (IsNetworkPrefix(full))
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.Network);
        }

        string? root = RootsFor(baseUri).FirstOrDefault(r => IsUnder(full, r));
        if (root is null)
        {
            return ThemeSourceDecision.Deny(ThemeSourceBlock.OutsideTheme);
        }

        return CrossesLink(root, full) ? ThemeSourceDecision.Deny(ThemeSourceBlock.Link) : ThemeSourceDecision.Allow(new Uri(full));
    }

    /// <summary>
    /// Algum item entre <paramref name="root"/> (exclusive) e <paramref name="fullPath"/> (inclusive) é link simbólico ou
    /// junção? Um link dentro do tema poderia apontar para fora dele (ou para um compartilhamento de rede). Partes que
    /// não existem não contam (nada é carregado delas); erro de acesso conta como link.
    /// </summary>
    public static bool CrossesLink(string root, string fullPath)
    {
        ArgumentNullException.ThrowIfNull(root);
        ArgumentNullException.ThrowIfNull(fullPath);
        string top = Path.TrimEndingDirectorySeparator(root);
        for (string? current = Path.TrimEndingDirectorySeparator(fullPath);
             current is not null && current.Length > top.Length;
             current = Path.GetDirectoryName(current))
        {
            try
            {
                if ((File.GetAttributes(current) & FileAttributes.ReparsePoint) != 0)
                {
                    return true;
                }
            }
            catch (Exception ex) when (ex is FileNotFoundException or DirectoryNotFoundException)
            {
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or ArgumentException or NotSupportedException)
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>Caminho absoluto normalizado dentro de <paramref name="root"/> (sem ser a própria pasta).</summary>
    public static bool IsUnder(string fullPath, string root)
    {
        ArgumentNullException.ThrowIfNull(fullPath);
        ArgumentNullException.ThrowIfNull(root);
        string prefix = Path.TrimEndingDirectorySeparator(root) + Path.DirectorySeparatorChar;
        return fullPath.StartsWith(prefix, StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>Dois separadores no início (<c>\\servidor</c>, <c>//servidor</c>, <c>\\?\UNC</c>, <c>\\.\</c>): rede ou dispositivo.</summary>
    private static bool IsNetworkPrefix(string path) =>
        path.Length >= 2 && path[0] is '\\' or '/' && path[1] is '\\' or '/';

    private static bool IsPackApplication(Uri uri) =>
        uri.IsAbsoluteUri && uri.OriginalString.StartsWith(PackApplication, StringComparison.OrdinalIgnoreCase)
        && uri.AbsoluteUri.StartsWith(PackApplication, StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Pastas permitidas para um elemento cujo XAML está em <paramref name="baseUri"/>: se ele é de um tema registrado,
    /// só aquele tema, o tema padrão e as pastas compartilhadas; senão (elemento de template sem origem conhecida),
    /// qualquer tema registrado e as pastas compartilhadas.
    /// </summary>
    private IEnumerable<string> RootsFor(Uri? baseUri)
    {
        var roots = _roots;
        string? owner = null;
        if (baseUri is { IsAbsoluteUri: true, IsFile: true, IsUnc: false })
        {
            string basePath = Path.GetFullPath(baseUri.LocalPath);
            owner = roots.Where(r => r.Kind is RootKind.Theme or RootKind.BaseTheme && IsUnder(basePath, r.Path))
                .OrderByDescending(r => r.Path.Length)
                .Select(r => r.Path)
                .FirstOrDefault();
        }

        return owner is null
            ? roots.Select(r => r.Path)
            : roots.Where(r => r.Kind is RootKind.BaseTheme or RootKind.Shared || string.Equals(r.Path, owner, StringComparison.OrdinalIgnoreCase)).Select(r => r.Path);
    }

    private void Add(string directory, RootKind kind)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(directory);
        if (IsNetworkPrefix(directory))
        {
            throw new ArgumentException("Pastas de rede não podem ser origem de temas.", nameof(directory));
        }

        string full = Path.TrimEndingDirectorySeparator(Path.GetFullPath(directory));
        if (IsNetworkPrefix(full))
        {
            throw new ArgumentException("Pastas de rede não podem ser origem de temas.", nameof(directory));
        }

        lock (_gate)
        {
            var existing = _roots.FirstOrDefault(r => string.Equals(r.Path, full, StringComparison.OrdinalIgnoreCase));
            if (existing is not null && existing.Kind >= kind)
            {
                return;
            }

            _roots = [.. _roots.Where(r => r != existing), new Root(full, kind)];
        }
    }

    private enum RootKind
    {
        Theme,
        BaseTheme,
        Shared,
    }

    private sealed record Root(string Path, RootKind Kind);
}
