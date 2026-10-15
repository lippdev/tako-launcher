namespace Launcher.Core.Themes;

/// <summary>
/// Compatibilidade de um <c>theme.yaml</c> com o adaptador Playnite 10 do Tako (MVP-015).
/// <para>
/// Faixa suportada de <c>ThemeApiVersion</c>: <b>2.x</b> (maior 2, qualquer menor), a API dos temas Fullscreen do
/// Playnite 10 para a qual o adaptador P10 foi escrito. Como no Playnite, a versão maior quebra compatibilidade:
/// 1.x (Playnite 8 e anteriores) e 3.x ou maior são recusados. Uma versão menor acima de
/// <see cref="TestedApiVersion"/> só gera aviso: pode usar recursos que o adaptador ainda não implementa.
/// </para>
/// </summary>
public static class ThemeManifestValidator
{
    /// <summary>Versão maior da API de temas Fullscreen aceita.</summary>
    public const int SupportedApiMajor = 2;

    /// <summary>Maior versão homologada (a do tema Fullscreen padrão do Playnite 10 incluído no Tako).</summary>
    public static readonly Version TestedApiVersion = new(2, 0, 0);

    public static string SupportedRange => $"{SupportedApiMajor}.x";

    public static ThemeManifestResult Validate(ThemeManifest manifest)
    {
        ArgumentNullException.ThrowIfNull(manifest);
        var diagnostics = new List<ThemeDiagnostic>();

        if (manifest.Id is null)
        {
            diagnostics.Add(ThemeDiagnostic.Error("manifest.missing-id", "Campo Id ausente no theme.yaml."));
        }
        else if (!ThemeId.IsSafe(manifest.Id))
        {
            diagnostics.Add(ThemeDiagnostic.Error("manifest.invalid-id", $"Id \"{manifest.Id}\" inválido: use só letras, números, ponto, hífen e sublinhado (até {ThemeId.MaxLength} caracteres)."));
        }

        if (manifest.Name is null)
        {
            diagnostics.Add(ThemeDiagnostic.Error("manifest.missing-name", "Campo Name ausente no theme.yaml."));
        }

        if (manifest.Author is null)
        {
            diagnostics.Add(ThemeDiagnostic.Warning("manifest.missing-author", "Campo Author ausente no theme.yaml."));
        }

        if (manifest.Version is null)
        {
            diagnostics.Add(ThemeDiagnostic.Warning("manifest.missing-version", "Campo Version ausente no theme.yaml."));
        }

        switch (manifest.Mode)
        {
            case ThemeMode.Desktop:
                diagnostics.Add(ThemeDiagnostic.Error("manifest.desktop", "Tema do modo Desktop do Playnite. O Tako só usa temas do modo Fullscreen."));
                break;
            case ThemeMode.Unknown when manifest.ModeText is null:
                diagnostics.Add(ThemeDiagnostic.Error("manifest.missing-mode", "Campo Mode ausente no theme.yaml (esperado: Fullscreen)."));
                break;
            case ThemeMode.Unknown:
                diagnostics.Add(ThemeDiagnostic.Error("manifest.invalid-mode", $"Mode \"{manifest.ModeText}\" desconhecido (esperado: Fullscreen)."));
                break;
        }

        CheckApiVersion(manifest, diagnostics);
        return new ThemeManifestResult(manifest, diagnostics);
    }

    private static void CheckApiVersion(ThemeManifest manifest, List<ThemeDiagnostic> diagnostics)
    {
        if (manifest.ThemeApiVersionText is null)
        {
            diagnostics.Add(ThemeDiagnostic.Error("manifest.missing-api", $"Campo ThemeApiVersion ausente no theme.yaml (suportado: {SupportedRange})."));
            return;
        }

        if (manifest.ThemeApiVersion is not { } api)
        {
            diagnostics.Add(ThemeDiagnostic.Error("manifest.invalid-api", $"ThemeApiVersion \"{manifest.ThemeApiVersionText}\" não é uma versão (suportado: {SupportedRange})."));
            return;
        }

        if (api.Major != SupportedApiMajor)
        {
            diagnostics.Add(ThemeDiagnostic.Error(
                "manifest.unsupported-api",
                $"ThemeApiVersion {manifest.ThemeApiVersionText} fora da faixa suportada pelo adaptador Playnite 10 ({SupportedRange})."));
            return;
        }

        if (new Version(api.Major, api.Minor) > new Version(TestedApiVersion.Major, TestedApiVersion.Minor))
        {
            diagnostics.Add(ThemeDiagnostic.Warning(
                "manifest.newer-api",
                $"ThemeApiVersion {manifest.ThemeApiVersionText} é mais nova que a homologada ({TestedApiVersion.ToString(3)}); partes do tema podem não carregar."));
        }
    }
}

/// <summary>Identificador de tema, também usado como nome da pasta de instalação.</summary>
public static class ThemeId
{
    public const int MaxLength = 128;

    private static readonly HashSet<string> WindowsDeviceNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    };

    /// <summary>Seguro como nome de pasta no Windows: <c>[A-Za-z0-9._-]</c>, sem começar/terminar com ponto e sem nomes de dispositivo.</summary>
    public static bool IsSafe(string id)
    {
        if (string.IsNullOrEmpty(id) || id.Length > MaxLength || id[0] == '.' || id[^1] == '.')
        {
            return false;
        }

        foreach (char c in id)
        {
            if (!(char.IsAsciiLetterOrDigit(c) || c is '.' or '-' or '_'))
            {
                return false;
            }
        }

        return !IsDeviceName(id.Split('.')[0]);
    }

    /// <summary>CON, NUL, COM1… são dispositivos no Windows mesmo com extensão ("nul.txt").</summary>
    public static bool IsDeviceName(string stem) => WindowsDeviceNames.Contains(stem.TrimEnd());
}
