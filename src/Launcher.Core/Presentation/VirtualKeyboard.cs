using System.Globalization;
using System.Text;

namespace Launcher.Core.Presentation;

public enum VirtualKeyKind
{
    Character,
    Space,
    Backspace,
    Clear,
    Done,
}

/// <summary>Tecla do teclado virtual. <paramref name="Width"/> é a largura em colunas (a grade tem 10).</summary>
public sealed record VirtualKey(VirtualKeyKind Kind, string Text, string Label, int Width = 1)
{
    public static VirtualKey Character(char c) =>
        new(VirtualKeyKind.Character, c.ToString(), char.ToUpperInvariant(c).ToString(CultureInfo.InvariantCulture));
}

/// <summary>
/// Teclado virtual da busca (UX-001): grade de 10 colunas que o direcional percorre, com Espaço, Apagar, Limpar e
/// Pronto. A digitação física passa pelas mesmas regras (<see cref="Type"/>), então o texto nunca começa com espaço,
/// não repete espaços nem passa de <see cref="MaxLength"/> caracteres.
/// </summary>
public static class VirtualKeyboard
{
    public const int Columns = 10;

    public const int MaxLength = 60;

    public static IReadOnlyList<IReadOnlyList<VirtualKey>> Rows { get; } =
    [
        Letters("1234567890"),
        Letters("qwertyuiop"),
        Letters("asdfghjkl-"),
        [.. Letters("zxcvbnm"), new VirtualKey(VirtualKeyKind.Backspace, string.Empty, "Apagar", 3)],
        [
            new VirtualKey(VirtualKeyKind.Space, " ", "Espaço", 4),
            new VirtualKey(VirtualKeyKind.Clear, string.Empty, "Limpar", 3),
            new VirtualKey(VirtualKeyKind.Done, string.Empty, "Pronto", 3),
        ],
    ];

    /// <summary>Texto depois de apertar a tecla. Pronto não muda o texto (quem chama fecha o teclado).</summary>
    public static string Apply(string? text, VirtualKey key)
    {
        ArgumentNullException.ThrowIfNull(key);
        return key.Kind switch
        {
            VirtualKeyKind.Character or VirtualKeyKind.Space => Type(text, key.Text),
            VirtualKeyKind.Backspace => Backspace(text),
            VirtualKeyKind.Clear => string.Empty,
            _ => text ?? string.Empty,
        };
    }

    /// <summary>Acrescenta o que foi digitado (teclado físico ou virtual), ignorando caracteres de controle.</summary>
    public static string Type(string? text, string? typed)
    {
        var result = new StringBuilder(text ?? string.Empty);
        foreach (char c in typed ?? string.Empty)
        {
            if (result.Length >= MaxLength)
            {
                break;
            }

            if (char.IsControl(c))
            {
                continue; // Enter, Tab, Backspace...
            }

            if (!char.IsWhiteSpace(c))
            {
                result.Append(c);
            }
            else if (result.Length > 0 && result[^1] != ' ')
            {
                result.Append(' ');
            }
        }

        return result.ToString();
    }

    /// <summary>Apaga o último caractere (um emoji ou outro par substituto sai inteiro).</summary>
    public static string Backspace(string? text)
    {
        if (string.IsNullOrEmpty(text))
        {
            return string.Empty;
        }

        int cut = text.Length >= 2 && char.IsSurrogatePair(text[^2], text[^1]) ? 2 : 1;
        return text[..^cut];
    }

    private static List<VirtualKey> Letters(string letters) => [.. letters.Select(VirtualKey.Character)];
}
