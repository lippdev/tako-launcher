using System.Globalization;
using System.Text;

namespace Launcher.Core.Library;

/// <summary>
/// Busca por texto na biblioteca (UX-001): sem diferenciar maiúsculas nem acentos ("pokemon" acha "Pokémon"),
/// ignorando pontuação ("god of war" acha "God of War: Ragnarök") e com todas as palavras digitadas em qualquer ordem.
/// </summary>
public static class SearchText
{
    /// <summary>Minúsculas, sem acentos, pontuação vira espaço e espaços repetidos viram um.</summary>
    public static string Normalize(string? text)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return string.Empty;
        }

        string decomposed = text.Normalize(NormalizationForm.FormD);
        var builder = new StringBuilder(decomposed.Length);
        bool space = true; // sem espaço no início
        foreach (char c in decomposed)
        {
            switch (CharUnicodeInfo.GetUnicodeCategory(c))
            {
                case UnicodeCategory.NonSpacingMark or UnicodeCategory.SpacingCombiningMark or UnicodeCategory.EnclosingMark:
                    continue;
                case UnicodeCategory.LowercaseLetter or UnicodeCategory.UppercaseLetter or UnicodeCategory.TitlecaseLetter
                    or UnicodeCategory.OtherLetter or UnicodeCategory.ModifierLetter or UnicodeCategory.DecimalDigitNumber
                    or UnicodeCategory.LetterNumber or UnicodeCategory.OtherNumber:
                    builder.Append(char.ToLowerInvariant(Fold(c)));
                    space = false;
                    break;
                default:
                    // Apóstrofo some ("Assassin's" = "assassins"); o resto da pontuação separa palavras.
                    if (c is '\'' or '’' or '`' or '´')
                    {
                        continue;
                    }

                    if (!space)
                    {
                        builder.Append(' ');
                        space = true;
                    }

                    break;
            }
        }

        return builder.ToString().TrimEnd();
    }

    /// <summary>
    /// Verdadeiro quando cada palavra da busca aparece no texto (em qualquer ordem). Busca vazia combina com tudo.
    /// Também aceita a busca sem espaços ("residentevil" acha "Resident Evil").
    /// </summary>
    public static bool Matches(string? text, string? query) => Matches(Normalize(text), Query.Parse(query));

    /// <summary>Versão para listas: normalize a busca uma vez com <see cref="Query.Parse"/>.</summary>
    public static bool Matches(string normalizedText, Query query)
    {
        ArgumentNullException.ThrowIfNull(normalizedText);
        ArgumentNullException.ThrowIfNull(query);
        if (query.IsEmpty)
        {
            return true;
        }

        if (query.Words.All(w => normalizedText.Contains(w, StringComparison.Ordinal)))
        {
            return true;
        }

        return query.Compact.Length > 1 && normalizedText.Replace(" ", string.Empty, StringComparison.Ordinal).Contains(query.Compact, StringComparison.Ordinal);
    }

    /// <summary>Letras que não se decompõem em letra + acento (ß, æ, ø, ł...).</summary>
    private static char Fold(char c) => c switch
    {
        'ø' or 'Ø' => 'o',
        'ł' or 'Ł' => 'l',
        'đ' or 'Đ' => 'd',
        'ı' => 'i',
        _ => c,
    };

    /// <summary>Busca já normalizada.</summary>
    public sealed class Query
    {
        private Query(string normalized)
        {
            Normalized = normalized;
            Words = normalized.Length == 0 ? [] : normalized.Split(' ');
            Compact = normalized.Replace(" ", string.Empty, StringComparison.Ordinal);
        }

        public static Query Empty { get; } = new(string.Empty);

        public string Normalized { get; }

        public IReadOnlyList<string> Words { get; }

        public string Compact { get; }

        public bool IsEmpty => Normalized.Length == 0;

        public static Query Parse(string? text) => string.IsNullOrWhiteSpace(text) ? Empty : new Query(Normalize(text));
    }
}
