using System.Globalization;

namespace Launcher.Core.Themes;

/// <summary>Modo do Playnite para o qual o tema foi feito.</summary>
public enum ThemeMode
{
    Unknown,
    Fullscreen,
    Desktop,
}

public enum ThemeDiagnosticSeverity
{
    Warning,
    Error,
}

/// <summary>Problema encontrado no manifesto ou no pacote. Erros impedem a instalação; avisos só informam.</summary>
public sealed record ThemeDiagnostic(ThemeDiagnosticSeverity Severity, string Code, string Message)
{
    public bool IsError => Severity == ThemeDiagnosticSeverity.Error;

    public static ThemeDiagnostic Error(string code, string message) => new(ThemeDiagnosticSeverity.Error, code, message);

    public static ThemeDiagnostic Warning(string code, string message) => new(ThemeDiagnosticSeverity.Warning, code, message);

    public override string ToString() => $"{(IsError ? "Erro" : "Aviso")}: {Message}";
}

/// <summary>
/// <c>theme.yaml</c> de um tema Playnite (MVP-015). O arquivo é "Chave: valor" em linhas; os campos que importam são
/// <c>Id</c>, <c>Name</c>, <c>Author</c>, <c>Version</c>, <c>Mode</c> e <c>ThemeApiVersion</c>. Outros campos ficam em
/// <see cref="Values"/>.
/// </summary>
public sealed record ThemeManifest(
    string? Id,
    string? Name,
    string? Author,
    string? Version,
    ThemeMode Mode,
    string? ModeText,
    string? ThemeApiVersionText,
    Version? ThemeApiVersion,
    IReadOnlyDictionary<string, string> Values)
{
    public const string FileName = "theme.yaml";

    /// <summary>Tamanho máximo aceito: o manifesto tem poucas linhas; algo maior é suspeito.</summary>
    public const int MaxBytes = 64 * 1024;

    /// <summary>Nome para exibir: o do manifesto ou, sem ele, o identificador.</summary>
    public string DisplayName => Name ?? Id ?? "(sem nome)";

    /// <summary>Lê o texto do manifesto. Nunca falha: campos ausentes ou inválidos ficam nulos e viram diagnóstico em <see cref="ThemeManifestValidator"/>.</summary>
    public static ThemeManifest Parse(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        var values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (string rawLine in text.Split('\n'))
        {
            string line = rawLine.TrimEnd('\r').TrimStart('﻿');
            // Só chaves no nível de topo: listas e mapas aninhados (ex.: Links) não interessam ao launcher.
            if (line.Length == 0 || char.IsWhiteSpace(line[0]) || line[0] is '#' or '-')
            {
                continue;
            }

            int colon = line.IndexOf(':', StringComparison.Ordinal);
            if (colon <= 0)
            {
                continue;
            }

            string key = line[..colon].Trim();
            string value = Unquote(StripComment(line[(colon + 1)..].Trim()));
            if (value.Length > 0)
            {
                values[key] = value;
            }
        }

        string? modeText = values.GetValueOrDefault("Mode");
        string? apiText = values.GetValueOrDefault("ThemeApiVersion");
        return new ThemeManifest(
            values.GetValueOrDefault("Id"),
            values.GetValueOrDefault("Name"),
            values.GetValueOrDefault("Author"),
            values.GetValueOrDefault("Version"),
            ParseMode(modeText),
            modeText,
            apiText,
            ParseApiVersion(apiText),
            values);
    }

    /// <summary>Lê e valida o <c>theme.yaml</c> de uma pasta de tema.</summary>
    public static ThemeManifestResult Read(string themeDirectory)
    {
        string path = Path.Combine(themeDirectory, FileName);
        if (!File.Exists(path))
        {
            return new ThemeManifestResult(Parse(string.Empty), [ThemeDiagnostic.Error("manifest.missing", $"{FileName} não encontrado na pasta do tema.")]);
        }

        if (new FileInfo(path).Length > MaxBytes)
        {
            return new ThemeManifestResult(Parse(string.Empty), [ThemeDiagnostic.Error("manifest.too-large", $"{FileName} tem mais de {MaxBytes / 1024} KB.")]);
        }

        return ThemeManifestValidator.Validate(Parse(File.ReadAllText(path)));
    }

    private static ThemeMode ParseMode(string? text) => text?.Trim().ToUpperInvariant() switch
    {
        "FULLSCREEN" => ThemeMode.Fullscreen,
        "DESKTOP" => ThemeMode.Desktop,
        _ => ThemeMode.Unknown,
    };

    /// <summary>"2", "2.0" e "2.0.0" são aceitos; o que não for número vira nulo.</summary>
    private static Version? ParseApiVersion(string? text)
    {
        if (text is null)
        {
            return null;
        }

        string normalized = text.Trim();
        if (int.TryParse(normalized, NumberStyles.None, CultureInfo.InvariantCulture, out int major))
        {
            return new Version(major, 0, 0);
        }

        return System.Version.TryParse(normalized, out var version) ? version : null;
    }

    /// <summary>Comentário no fim da linha (" # ...") fora de aspas.</summary>
    private static string StripComment(string value)
    {
        if (value.StartsWith('"') || value.StartsWith('\''))
        {
            return value;
        }

        int hash = value.IndexOf(" #", StringComparison.Ordinal);
        return hash >= 0 ? value[..hash].TrimEnd() : value;
    }

    private static string Unquote(string value)
    {
        if (value.Length >= 2 && ((value[0] == '"' && value[^1] == '"') || (value[0] == '\'' && value[^1] == '\'')))
        {
            return value[1..^1].Trim();
        }

        return value;
    }
}

/// <summary>Manifesto lido com os diagnósticos de compatibilidade.</summary>
public sealed record ThemeManifestResult(ThemeManifest Manifest, IReadOnlyList<ThemeDiagnostic> Diagnostics)
{
    public bool IsCompatible => !Diagnostics.Any(d => d.IsError);

    public IEnumerable<ThemeDiagnostic> Errors => Diagnostics.Where(d => d.IsError);
}
