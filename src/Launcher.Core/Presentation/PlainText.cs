using System.Net;
using System.Text;

namespace Launcher.Core.Presentation;

/// <summary>
/// HTML de descrições (lojas, temas do Playnite) → texto puro. Não interpreta nada: tags somem, entidades viram
/// caracteres e quebras de bloco viram linhas. Usado pelo substituto seguro do <c>HtmlTextView</c> do Playnite (MVP-018),
/// que não carrega imagens, links, estilos nem scripts.
/// </summary>
public static class PlainText
{
    /// <summary>Limite do texto exibido: descrições enormes não travam a UI.</summary>
    public const int MaxLength = 20_000;

    /// <summary>Tags que começam ou terminam uma linha.</summary>
    private static readonly HashSet<string> LineTags = new(StringComparer.OrdinalIgnoreCase) { "li", "div", "tr", "hr" };

    /// <summary>Tags que separam parágrafos (uma linha em branco).</summary>
    private static readonly HashSet<string> ParagraphTags = new(StringComparer.OrdinalIgnoreCase)
    {
        "p", "ul", "ol", "h1", "h2", "h3", "h4", "h5", "h6", "table", "blockquote",
    };

    /// <summary>Conteúdo dessas tags não é texto para o usuário e é descartado inteiro.</summary>
    private static readonly HashSet<string> SkippedContentTags = new(StringComparer.OrdinalIgnoreCase) { "script", "style", "head", "title" };

    public static string FromHtml(string? html)
    {
        if (string.IsNullOrEmpty(html))
        {
            return string.Empty;
        }

        var text = new StringBuilder(Math.Min(html.Length, MaxLength));
        string? skipping = null;
        int i = 0;
        while (i < html.Length && text.Length < MaxLength)
        {
            char c = html[i];
            if (c == '<')
            {
                int end = TagEnd(html, i);
                if (end < 0)
                {
                    // "<" solto (ex.: "a < b"): texto comum.
                    if (skipping is null)
                    {
                        text.Append(c);
                    }

                    i++;
                    continue;
                }

                var (name, closing) = TagName(html, i + 1, end);
                if (skipping is not null)
                {
                    if (closing && string.Equals(name, skipping, StringComparison.OrdinalIgnoreCase))
                    {
                        skipping = null;
                    }
                }
                else if (!closing && SkippedContentTags.Contains(name) && html[end - 1] != '/')
                {
                    skipping = name;
                }
                else if (string.Equals(name, "br", StringComparison.OrdinalIgnoreCase))
                {
                    text.Append('\n');
                }
                else if (LineTags.Contains(name) || ParagraphTags.Contains(name))
                {
                    Break(text, ParagraphTags.Contains(name) ? 2 : 1);
                }

                i = end + 1;
                continue;
            }

            int next = html.IndexOf('<', i);
            if (next < 0)
            {
                next = html.Length;
            }

            if (skipping is null)
            {
                AppendText(text, WebUtility.HtmlDecode(html[i..next]));
            }

            i = next;
        }

        return Collapse(text.ToString());
    }

    /// <summary>Posição do <c>&gt;</c> que fecha a tag, respeitando aspas; -1 se não for uma tag.</summary>
    private static int TagEnd(string html, int start)
    {
        if (start + 1 >= html.Length || !(char.IsLetter(html[start + 1]) || html[start + 1] is '/' or '!' or '?'))
        {
            return -1;
        }

        char? quote = null;
        for (int i = start + 1; i < html.Length; i++)
        {
            char c = html[i];
            if (quote is not null)
            {
                if (c == quote)
                {
                    quote = null;
                }
            }
            else if (c is '"' or '\'')
            {
                quote = c;
            }
            else if (c == '>')
            {
                return i;
            }
        }

        return -1;
    }

    private static (string Name, bool Closing) TagName(string html, int start, int end)
    {
        bool closing = start < end && html[start] == '/';
        int i = closing ? start + 1 : start;
        int nameStart = i;
        while (i < end && (char.IsLetterOrDigit(html[i]) || html[i] is '-' or ':'))
        {
            i++;
        }

        return (html[nameStart..i], closing);
    }

    private static void AppendText(StringBuilder text, string fragment)
    {
        foreach (char c in fragment)
        {
            if (text.Length >= MaxLength)
            {
                return;
            }

            // Espaços e quebras do código-fonte HTML não são quebras de linha visíveis.
            text.Append(char.IsWhiteSpace(c) ? ' ' : c);
        }
    }

    /// <summary>Garante <paramref name="lines"/> quebras no fim do texto (nada no começo).</summary>
    private static void Break(StringBuilder text, int lines)
    {
        int trailing = 0;
        for (int i = text.Length - 1; i >= 0 && text[i] is '\n' or ' '; i--)
        {
            trailing += text[i] == '\n' ? 1 : 0;
        }

        if (text.Length == 0)
        {
            return;
        }

        for (; trailing < lines; trailing++)
        {
            text.Append('\n');
        }
    }

    /// <summary>Espaços repetidos viram um; no máximo uma linha em branco entre parágrafos; sem bordas vazias.</summary>
    private static string Collapse(string text)
    {
        var lines = text.Split('\n').Select(l => string.Join(' ', l.Split(' ', StringSplitOptions.RemoveEmptyEntries)));
        var result = new StringBuilder();
        int blank = 0;
        foreach (string line in lines)
        {
            if (line.Length == 0)
            {
                blank++;
                continue;
            }

            if (result.Length > 0)
            {
                result.Append(blank > 0 ? "\n\n" : "\n");
            }

            result.Append(line);
            blank = 0;
        }

        return result.ToString();
    }
}
