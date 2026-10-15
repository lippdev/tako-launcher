using System.Globalization;
using System.Text;

namespace Launcher.Core.Library.Metadata;

/// <summary>Compara o título local (nome do executável, por exemplo) com os resultados de busca.</summary>
public static class TitleMatcher
{
    /// <summary>Abaixo disso a correspondência não é aplicada automaticamente (o usuário pode corrigir).</summary>
    public const double Threshold = 0.6;

    private static readonly HashSet<string> Noise = new(StringComparer.Ordinal)
    {
        "the", "edition", "goty", "definitive", "remastered", "deluxe", "complete", "ultimate", "game", "of", "year", "launcher", "x64", "win64", "shipping",
    };

    /// <summary>Minúsculas, sem acentos, ™/®, pontuação e palavras de edição; algarismos romanos simples viram números.</summary>
    public static IReadOnlyList<string> Tokens(string title)
    {
        ArgumentNullException.ThrowIfNull(title);
        string decomposed = title.ToLowerInvariant().Normalize(NormalizationForm.FormD);
        var builder = new StringBuilder();
        foreach (char c in decomposed)
        {
            if (CharUnicodeInfo.GetUnicodeCategory(c) == UnicodeCategory.NonSpacingMark)
            {
                continue;
            }

            builder.Append(char.IsLetterOrDigit(c) ? c : ' ');
        }

        return builder.ToString()
            .Split(' ', StringSplitOptions.RemoveEmptyEntries)
            .Select(t => t switch { "ii" => "2", "iii" => "3", "iv" => "4", "v" => "5", "vi" => "6", _ => t })
            .Where(t => !Noise.Contains(t))
            .ToList();
    }

    /// <summary>1 = mesmo título; 0 = nada em comum (Dice sobre as palavras, com bônus para prefixo).</summary>
    public static double Score(string local, string candidate)
    {
        var a = Tokens(local);
        var b = Tokens(candidate);
        if (a.Count == 0 || b.Count == 0)
        {
            return 0;
        }

        if (a.SequenceEqual(b))
        {
            return 1;
        }

        int common = a.Intersect(b).Count();
        double dice = 2.0 * common / (a.Count + b.Count);
        bool prefix = b.Take(a.Count).SequenceEqual(a) || a.Take(b.Count).SequenceEqual(b);
        return Math.Min(0.99, prefix ? Math.Max(dice, 0.8) : dice);
    }

    public static MetadataMatch? Best(string title, IEnumerable<MetadataMatch> candidates)
    {
        var best = candidates
            .Select(c => (Match: c, Score: Score(title, c.Title)))
            .OrderByDescending(x => x.Score)
            .FirstOrDefault();
        return best.Match is not null && best.Score >= Threshold ? best.Match : null;
    }
}
