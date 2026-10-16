using System.Globalization;

namespace Launcher.Core.Library.Metadata.Art;

/// <summary>Imagens da Steam para o seletor: endereços lógicos <c>steam-art://</c>, resolvidos ao baixar (ADR-0023).</summary>
public static class SteamArtCandidates
{
    public static IReadOnlyList<ArtCandidate> For(int appId)
    {
        Uri Art(string kind) => new($"{Steam.SteamGames.ArtScheme}://{appId.ToString(CultureInfo.InvariantCulture)}/{kind}");
        return
        [
            new ArtCandidate(ArtSources.Steam, ArtKind.Cover, Art("cover")) { Label = "Capa da biblioteca (2x)", Width = 600, Height = 900 },
            new ArtCandidate(ArtSources.Steam, ArtKind.Hero, Art("hero")) { Label = "Fundo da biblioteca (2x)", Width = 3840, Height = 1240 },
            new ArtCandidate(ArtSources.Steam, ArtKind.Logo, Art("logo")) { Label = "Logo da biblioteca" },
            new ArtCandidate(ArtSources.Steam, ArtKind.Icon, Art("icon")) { Label = "Ícone da comunidade" },
        ];
    }
}

/// <summary>Imagens fixadas pelo usuário: aplicar ao jogo, trocar e voltar à automática (regra pura, testada).</summary>
public static class ArtPins
{
    /// <summary>Novo conjunto com a imagem do tipo fixada (ou solta, com <paramref name="url"/> nula).</summary>
    public static IReadOnlyDictionary<ArtKind, Uri> With(IReadOnlyDictionary<ArtKind, Uri> current, ArtKind kind, Uri? url)
    {
        ArgumentNullException.ThrowIfNull(current);
        var next = new Dictionary<ArtKind, Uri>(current);
        if (url is null)
        {
            next.Remove(kind);
        }
        else
        {
            next[kind] = url;
        }

        return next;
    }

    /// <summary>Endereço automático do tipo (o que aparece sem escolha do usuário).</summary>
    public static Uri? Automatic(Game game, ArtKind kind)
    {
        ArgumentNullException.ThrowIfNull(game);
        return kind switch
        {
            ArtKind.Cover => game.CoverUri,
            ArtKind.Hero => game.HeroUri,
            ArtKind.Logo => game.Media.GetValueOrDefault(GameMedia.Logo),
            _ => game.Media.GetValueOrDefault(GameMedia.Icon),
        };
    }

    /// <summary>Endereço exibido do tipo: o fixado ou o automático.</summary>
    public static Uri? Shown(Game game, ArtKind kind)
    {
        ArgumentNullException.ThrowIfNull(game);
        return game.PinnedArt.GetValueOrDefault(kind) ?? Automatic(game, kind);
    }
}

/// <summary>Tamanhos de arquivo para a tela ("Limpar cache de imagens · 12,4 MB").</summary>
public static class ByteSize
{
    private static readonly CultureInfo PtBr = CultureInfo.GetCultureInfo("pt-BR");

    public static string Format(long bytes) => bytes switch
    {
        <= 0 => "vazio",
        < 1024 => $"{bytes} B",
        < 1024 * 1024 => $"{Math.Max(1, bytes / 1024)} KB",
        < 1024L * 1024 * 1024 => (bytes / (1024.0 * 1024)).ToString("0.0", PtBr) + " MB",
        _ => (bytes / (1024.0 * 1024 * 1024)).ToString("0.0", PtBr) + " GB",
    };
}

/// <summary>
/// Aba "Arquivo do PC" do seletor: as imagens mais recentes das pastas comuns (Imagens, Downloads, Área de Trabalho),
/// até um nível de subpasta, para escolher pelo controle sem a janela do Windows.
/// </summary>
public static class LocalImages
{
    public static readonly IReadOnlyList<string> Extensions = [".jpg", ".jpeg", ".png", ".webp", ".bmp", ".gif"];

    public static IReadOnlyList<string> Find(IEnumerable<string> folders, int max = 60)
    {
        ArgumentNullException.ThrowIfNull(folders);
        var files = new List<FileInfo>();
        foreach (string folder in folders.Where(Directory.Exists).Distinct(StringComparer.OrdinalIgnoreCase))
        {
            try
            {
                var root = new DirectoryInfo(folder);
                files.AddRange(Images(root));
                foreach (var sub in root.EnumerateDirectories().Where(d => !d.Attributes.HasFlag(FileAttributes.Hidden)).Take(40))
                {
                    files.AddRange(Images(sub));
                }
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                // Pasta sem permissão: segue com as outras.
            }
        }

        return [.. files.OrderByDescending(f => f.LastWriteTimeUtc).Select(f => f.FullName).Distinct(StringComparer.OrdinalIgnoreCase).Take(max)];
    }

    private static IEnumerable<FileInfo> Images(DirectoryInfo folder)
    {
        try
        {
            return [.. folder.EnumerateFiles().Where(f => Extensions.Contains(f.Extension.ToLowerInvariant()) && f.Length > 0)];
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return [];
        }
    }
}
