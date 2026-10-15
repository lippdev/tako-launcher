using System.Text;

namespace Launcher.Core.Library.Steam;

/// <summary>
/// Nó do formato KeyValues/VDF da Steam (texto): <c>"chave" "valor"</c> ou <c>"chave" { ... }</c>. Chaves sem
/// diferenciar maiúsculas; comentários <c>//</c> e condicionais <c>[$WIN32]</c> são ignorados.
/// </summary>
public sealed class VdfNode
{
    private readonly Dictionary<string, object> _children = new(StringComparer.OrdinalIgnoreCase);

    public IEnumerable<KeyValuePair<string, object>> Children => _children;

    public string? Value(string key) => _children.TryGetValue(key, out var value) ? value as string : null;

    public VdfNode? Node(string key) => _children.TryGetValue(key, out var value) ? value as VdfNode : null;

    /// <summary>Caminho de nós, ex.: <c>Path("UserLocalConfigStore", "Software", "Valve")</c>.</summary>
    public VdfNode? Path(params string[] keys)
    {
        VdfNode? node = this;
        foreach (string key in keys)
        {
            node = node?.Node(key);
        }

        return node;
    }

    internal void Set(string key, object value) => _children[key] = value;
}

public static class Vdf
{
    /// <summary>Lê o documento inteiro; o resultado é o nó raiz (que contém o nó de nível superior, ex.: "AppState").</summary>
    public static VdfNode Parse(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        int position = 0;
        var root = new VdfNode();
        ParseInto(root, text, ref position);
        return root;
    }

    public static VdfNode Load(string path) => Parse(File.ReadAllText(path, Encoding.UTF8));

    private static void ParseInto(VdfNode node, string text, ref int position)
    {
        while (true)
        {
            string? key = NextToken(text, ref position, out bool keyQuoted);
            if (key is null || (key == "}" && !keyQuoted))
            {
                return; // fim do arquivo ou do bloco
            }

            string? value = NextToken(text, ref position, out bool valueQuoted);
            if (value is null)
            {
                return;
            }

            // Condicional de plataforma depois do valor: [$WIN32]
            SkipConditional(text, ref position);
            if (value == "{" && !valueQuoted)
            {
                var child = new VdfNode();
                ParseInto(child, text, ref position);
                node.Set(key, child);
            }
            else
            {
                node.Set(key, value);
            }
        }
    }

    private static void SkipConditional(string text, ref int position)
    {
        int p = position;
        SkipWhitespaceAndComments(text, ref p);
        if (p < text.Length && text[p] == '[')
        {
            int end = text.IndexOf(']', p);
            position = end < 0 ? text.Length : end + 1;
        }
    }

    private static string? NextToken(string text, ref int position, out bool quoted)
    {
        quoted = false;
        SkipWhitespaceAndComments(text, ref position);
        if (position >= text.Length)
        {
            return null;
        }

        char c = text[position];
        if (c is '{' or '}')
        {
            position++;
            return c.ToString();
        }

        var builder = new StringBuilder();
        if (c == '"')
        {
            quoted = true;
            position++;
            while (position < text.Length && text[position] != '"')
            {
                if (text[position] == '\\' && position + 1 < text.Length)
                {
                    position++;
                    builder.Append(text[position] switch
                    {
                        'n' => '\n',
                        't' => '\t',
                        _ => text[position],
                    });
                }
                else
                {
                    builder.Append(text[position]);
                }

                position++;
            }

            position++; // aspas de fechamento
            return builder.ToString();
        }

        while (position < text.Length && !char.IsWhiteSpace(text[position]) && text[position] is not ('{' or '}' or '"'))
        {
            builder.Append(text[position++]);
        }

        return builder.ToString();
    }

    private static void SkipWhitespaceAndComments(string text, ref int position)
    {
        while (position < text.Length)
        {
            if (char.IsWhiteSpace(text[position]))
            {
                position++;
            }
            else if (text[position] == '/' && position + 1 < text.Length && text[position + 1] == '/')
            {
                int end = text.IndexOf('\n', position);
                position = end < 0 ? text.Length : end + 1;
            }
            else
            {
                return;
            }
        }
    }
}
