using System.Buffers.Binary;
using System.Text;

namespace Launcher.Core.Library.Xbox;

/// <summary>
/// <c>.GamingRoot</c>: arquivo oculto que o app Xbox grava na raiz de cada unidade escolhida para jogos. Formato:
/// "RGBX", um inteiro de 32 bits (quantidade de pastas) e as pastas em UTF-16LE terminadas em zero, relativas à
/// unidade (ex.: <c>XboxGames</c>).
/// </summary>
public static class GamingRoot
{
    public const string FileName = ".GamingRoot";

    /// <summary>Pasta que o app Xbox usa quando o arquivo não existe ou não pôde ser lido.</summary>
    public const string DefaultFolder = "XboxGames";

    private const int MaxFolders = 64;

    /// <summary>Pastas declaradas (como estão no arquivo); vazio se o arquivo não tiver o formato esperado.</summary>
    public static IReadOnlyList<string> Parse(ReadOnlySpan<byte> data)
    {
        if (data.Length < 8 || !data[..4].SequenceEqual("RGBX"u8))
        {
            return [];
        }

        uint declared = BinaryPrimitives.ReadUInt32LittleEndian(data[4..8]);
        int limit = declared is > 0 and <= MaxFolders ? (int)declared : MaxFolders;
        var folders = new List<string>();
        var rest = data[8..];
        while (rest.Length >= 2 && folders.Count < limit)
        {
            int end = 0;
            while (end + 1 < rest.Length && (rest[end] != 0 || rest[end + 1] != 0))
            {
                end += 2;
            }

            string folder = Encoding.Unicode.GetString(rest[..end]).Trim();
            if (folder.Length > 0)
            {
                folders.Add(folder);
            }

            rest = end + 2 <= rest.Length ? rest[(end + 2)..] : [];
        }

        return folders;
    }

    /// <summary>
    /// Caminhos completos das pastas de jogos de uma unidade (<paramref name="driveRoot"/>, ex.: <c>D:\</c>): as do
    /// arquivo, ou <see cref="DefaultFolder"/> quando ele não declara nenhuma.
    /// </summary>
    public static IReadOnlyList<string> Folders(string driveRoot, ReadOnlySpan<byte> data)
    {
        ArgumentNullException.ThrowIfNull(driveRoot);
        var declared = Parse(data);
        return [.. (declared.Count == 0 ? [DefaultFolder] : declared)
            .Select(f => IsRooted(f) ? f : Path.Combine(driveRoot, f.TrimStart('\\', '/')))
            .Distinct(StringComparer.OrdinalIgnoreCase)];
    }

    // "D:\Jogos" ou "\\servidor\…" são absolutos mesmo fora do Windows (testes rodam no Mac).
    private static bool IsRooted(string path) =>
        (path.Length >= 3 && char.IsAsciiLetter(path[0]) && path[1] == ':' && path[2] is '\\' or '/') || path.StartsWith(@"\\", StringComparison.Ordinal);
}
