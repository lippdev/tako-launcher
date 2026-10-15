using System.Globalization;

namespace Launcher.Core.Library.Xbox;

/// <summary>
/// Imagens do pacote: o manifesto cita <c>Assets\Logo.png</c>, mas no disco ficam variações com qualificadores
/// (<c>Logo.scale-200.png</c>, <c>Logo.targetsize-256.png</c>, <c>Logo.contrast-black_scale-100.png</c>).
/// </summary>
public static class PackageAssets
{
    /// <summary>
    /// Entre os <paramref name="fileNames"/> da pasta, o melhor arquivo para <paramref name="declared"/> (só o nome, ex.:
    /// <c>Logo.png</c>): o próprio, senão a variação de maior escala sem alto contraste. Nulo se nada casar.
    /// </summary>
    public static string? Pick(string declared, IEnumerable<string> fileNames)
    {
        ArgumentNullException.ThrowIfNull(declared);
        ArgumentNullException.ThrowIfNull(fileNames);
        string stem = Path.GetFileNameWithoutExtension(declared);
        string extension = Path.GetExtension(declared);
        string? best = null;
        int bestScore = int.MinValue;
        foreach (string file in fileNames)
        {
            if (string.Equals(file, declared, StringComparison.OrdinalIgnoreCase))
            {
                return file;
            }

            if (!file.StartsWith(stem + ".", StringComparison.OrdinalIgnoreCase) || !file.EndsWith(extension, StringComparison.OrdinalIgnoreCase)
                || file.Length <= stem.Length + 1 + extension.Length)
            {
                continue;
            }

            string qualifiers = file[(stem.Length + 1)..^extension.Length];
            int score = Score(qualifiers);
            if (score > bestScore)
            {
                best = file;
                bestScore = score;
            }
        }

        return best;
    }

    /// <summary>Caminho real de uma imagem citada no manifesto, dentro da pasta do pacote; nulo se não existir.</summary>
    public static string? Resolve(string packageFolder, string? declared)
    {
        ArgumentNullException.ThrowIfNull(packageFolder);
        if (string.IsNullOrWhiteSpace(declared) || declared.Contains("..", StringComparison.Ordinal) || Path.IsPathRooted(declared))
        {
            return null;
        }

        string relative = declared.Replace('\\', Path.DirectorySeparatorChar).Replace('/', Path.DirectorySeparatorChar);
        string folder = Path.Combine(packageFolder, Path.GetDirectoryName(relative) ?? string.Empty);
        try
        {
            if (!Directory.Exists(folder))
            {
                return null;
            }

            string? file = Pick(Path.GetFileName(relative), Directory.EnumerateFiles(folder).Select(Path.GetFileName).OfType<string>());
            return file is null ? null : Path.Combine(folder, file);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    // Maior escala/tamanho primeiro; alto contraste e "unplated" (ícone sem fundo) por último.
    private static int Score(string qualifiers)
    {
        int score = 0;
        foreach (string part in qualifiers.Split('_', '.'))
        {
            int dash = part.IndexOf('-', StringComparison.Ordinal);
            string name = dash < 0 ? part : part[..dash];
            string value = dash < 0 ? string.Empty : part[(dash + 1)..];
            _ = int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out int number);
            score += name.ToLowerInvariant() switch
            {
                "scale" => number,
                "targetsize" => number,
                "contrast" => -10_000,
                "altform" => -1_000,
                _ => 0,
            };
        }

        return score;
    }
}
