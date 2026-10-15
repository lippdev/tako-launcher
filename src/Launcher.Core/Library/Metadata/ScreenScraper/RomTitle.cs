using System.Text.RegularExpressions;

namespace Launcher.Core.Library.Metadata.ScreenScraper;

/// <summary>
/// Título legível a partir do nome do arquivo da ROM: tira as marcações dos dats ("(USA)", "(Rev 1)", "[!]") e troca
/// "_" por espaço. "Super Mario World (USA) [!].sfc" → "Super Mario World". Puro, testado.
/// </summary>
public static partial class RomTitle
{
    public static string FromFileName(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        string name = Path.GetFileNameWithoutExtension(path).Replace('_', ' ');
        string clean = Tags().Replace(name, " ");
        clean = Spaces().Replace(clean, " ").Trim(' ', '-', '.');
        return clean.Length > 0 ? clean : name.Trim();
    }

    [GeneratedRegex(@"\s*[\(\[][^\)\]]*[\)\]]")]
    private static partial Regex Tags();

    [GeneratedRegex(@"\s{2,}")]
    private static partial Regex Spaces();
}
