using System.Text;

namespace Launcher.Core.Emulation;

/// <summary>
/// Monta a linha de comando do emulador a partir do modelo (EMU-004): <c>{rom}</c> (arquivo), <c>{romdir}</c> (pasta),
/// <c>{romname}</c> (nome sem extensão), <c>{core}</c> (core do RetroArch), <c>{config}</c> (.cfg extra do jogo) e
/// <c>{fullscreen}</c> (a opção de tela cheia do emulador). Caminhos entram entre aspas sozinhos (o usuário escreve
/// <c>{rom}</c>, não <c>"{rom}"</c>; com aspas no modelo também funciona). Puro, testado.
/// </summary>
public static class LaunchArguments
{
    public const string Rom = "{rom}";
    public const string RomDir = "{romdir}";
    public const string RomName = "{romname}";
    public const string Core = "{core}";
    public const string Config = "{config}";
    public const string FullscreenToken = "{fullscreen}";

    /// <summary>Marcadores aceitos, na ordem em que o editor oferece.</summary>
    public static readonly IReadOnlyList<string> Placeholders = [Rom, FullscreenToken, RomDir, RomName, Core, Config];

    // Valores que vão entre aspas quando o modelo não as tem (caminhos e nomes podem ter espaço).
    private static readonly HashSet<string> Quoted = new(StringComparer.OrdinalIgnoreCase) { Rom, RomDir, RomName, Core, Config };

    /// <summary>Marcadores desconhecidos no modelo (ex.: "{ROMPATH}"), para o editor avisar antes de salvar.</summary>
    public static IReadOnlyList<string> Unknown(string template)
    {
        ArgumentNullException.ThrowIfNull(template);
        var unknown = new List<string>();
        int start = 0;
        while ((start = template.IndexOf('{', start)) >= 0)
        {
            int end = template.IndexOf('}', start);
            if (end < 0)
            {
                break;
            }

            string token = template[start..(end + 1)];
            if (!Placeholders.Contains(token, StringComparer.OrdinalIgnoreCase) && !unknown.Contains(token))
            {
                unknown.Add(token);
            }

            start = end + 1;
        }

        return unknown;
    }

    /// <summary>
    /// Troca os marcadores. Valor ausente (ex.: <c>{core}</c> fora do RetroArch) some junto com o espaço. O resultado
    /// não tem espaços duplicados nem nas pontas.
    /// </summary>
    public static string Expand(string template, IReadOnlyDictionary<string, string?> values)
    {
        ArgumentNullException.ThrowIfNull(template);
        ArgumentNullException.ThrowIfNull(values);
        var builder = new StringBuilder();
        bool inQuotes = false;
        for (int i = 0; i < template.Length; i++)
        {
            char c = template[i];
            if (c == '"')
            {
                inQuotes = !inQuotes;
                builder.Append(c);
                continue;
            }

            if (c == '{' && template.IndexOf('}', i) is int end and > 0)
            {
                string token = template[i..(end + 1)];
                var match = values.FirstOrDefault(v => string.Equals(v.Key, token, StringComparison.OrdinalIgnoreCase));
                if (match.Key is not null)
                {
                    string value = match.Value ?? string.Empty;
                    bool quote = !inQuotes && value.Length > 0 && Quoted.Contains(token);
                    builder.Append(quote ? $"\"{value}\"" : value);
                    i = end;
                    continue;
                }
            }

            builder.Append(c);
        }

        // Aspas vazias de um valor ausente ("{core}" sem core) e espaços repetidos.
        string result = builder.ToString().Replace("\"\"", string.Empty, StringComparison.Ordinal);
        return string.Join(' ', result.Split(' ', StringSplitOptions.RemoveEmptyEntries));
    }

    /// <summary>Valores de uma ROM: arquivo, pasta e nome; core, .cfg e tela cheia quando houver.</summary>
    public static Dictionary<string, string?> For(string romPath, string? corePath, string? configFile, string fullscreen) => new(StringComparer.OrdinalIgnoreCase)
    {
        [Rom] = romPath,
        [RomDir] = WinPath.Directory(romPath),
        [RomName] = WinPath.NameWithoutExtension(romPath),
        [Core] = corePath,
        [Config] = configFile,
        [FullscreenToken] = fullscreen,
    };
}
