using System.IO.Compression;

namespace Launcher.Core.Themes;

/// <summary>Limites de um pacote de tema. Os padrões cobrem com folga temas reais (dezenas de arquivos, poucos MB).</summary>
public sealed record ThemePackageLimits
{
    public static ThemePackageLimits Default { get; } = new();

    /// <summary>Soma dos tamanhos descompactados.</summary>
    public long MaxTotalBytes { get; init; } = 200L * 1024 * 1024;

    /// <summary>Tamanho descompactado de um único arquivo.</summary>
    public long MaxFileBytes { get; init; } = 64L * 1024 * 1024;

    /// <summary>Entradas do zip (arquivos e pastas).</summary>
    public int MaxEntries { get; init; } = 5000;

    /// <summary>Tamanho do pacote em disco.</summary>
    public long MaxPackageBytes { get; init; } = 200L * 1024 * 1024;
}

/// <summary>Resultado de importar um pacote: o tema instalado ou os motivos da recusa.</summary>
public sealed record ThemeInstallResult(
    bool Succeeded,
    ThemeManifest? Manifest,
    string? Directory,
    int FileCount,
    long TotalBytes,
    IReadOnlyList<ThemeDiagnostic> Diagnostics)
{
    public IEnumerable<ThemeDiagnostic> Errors => Diagnostics.Where(d => d.IsError);

    public IEnumerable<ThemeDiagnostic> Warnings => Diagnostics.Where(d => !d.IsError);
}

/// <summary>
/// Importa pacotes <c>.pthm</c> do Playnite (um zip com o tema) sem o Playnite instalado (MVP-014). O pacote é dado
/// não confiável: cada entrada é validada (sem path traversal/zip-slip, sem caminhos absolutos ou de dispositivo), os
/// tamanhos são contados durante a extração (o tamanho declarado no zip pode mentir), o manifesto é validado antes de
/// gravar qualquer arquivo, e a extração vai para uma pasta temporária que só vira o tema instalado no fim.
/// Nada do pacote é executado; arquivos executáveis nem são extraídos.
/// </summary>
public static class ThemePackage
{
    public const string Extension = ".pthm";

    /// <summary>Extensões que um tema não precisa e que não devem parar no disco do usuário.</summary>
    private static readonly HashSet<string> BlockedExtensions = new(StringComparer.OrdinalIgnoreCase)
    {
        ".exe", ".dll", ".com", ".scr", ".msi", ".msix", ".appx", ".bat", ".cmd", ".ps1", ".psm1", ".vbs", ".vbe",
        ".js", ".jse", ".wsf", ".wsh", ".hta", ".lnk", ".url", ".reg", ".cpl", ".jar", ".sys", ".pif",
    };

    /// <summary>Instala o pacote em <c><paramref name="themesRoot"/>/&lt;Id&gt;</c>, substituindo uma versão anterior do mesmo tema.</summary>
    /// <param name="reservedIds">Ids que não podem ser instalados (temas embutidos).</param>
    public static ThemeInstallResult Install(string packagePath, string themesRoot, ThemePackageLimits? limits = null, IEnumerable<string>? reservedIds = null)
    {
        ArgumentNullException.ThrowIfNull(packagePath);
        ArgumentNullException.ThrowIfNull(themesRoot);
        limits ??= ThemePackageLimits.Default;
        try
        {
            var info = new FileInfo(packagePath);
            if (!info.Exists)
            {
                return Fail(ThemeDiagnostic.Error("package.not-found", $"Arquivo não encontrado: {packagePath}"));
            }

            if (info.Length > limits.MaxPackageBytes)
            {
                return Fail(ThemeDiagnostic.Error("package.too-large", $"O pacote tem {Mb(info.Length)}; o limite é {Mb(limits.MaxPackageBytes)}."));
            }

            using var stream = info.OpenRead();
            return Install(stream, themesRoot, limits, reservedIds);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return Fail(ThemeDiagnostic.Error("package.io", $"Não foi possível ler o pacote: {ex.Message}"));
        }
    }

    /// <inheritdoc cref="Install(string, string, ThemePackageLimits?, IEnumerable{string}?)"/>
    public static ThemeInstallResult Install(Stream package, string themesRoot, ThemePackageLimits? limits = null, IEnumerable<string>? reservedIds = null)
    {
        ArgumentNullException.ThrowIfNull(package);
        ArgumentNullException.ThrowIfNull(themesRoot);
        limits ??= ThemePackageLimits.Default;
        ZipArchive archive;
        try
        {
            archive = new ZipArchive(package, ZipArchiveMode.Read, leaveOpen: true);
        }
        catch (InvalidDataException)
        {
            return Fail(ThemeDiagnostic.Error("package.not-zip", "O arquivo não é um pacote de tema válido (.pthm é um arquivo zip)."));
        }

        using (archive)
        {
            try
            {
                return Install(archive, Path.GetFullPath(themesRoot), limits, reservedIds);
            }
            catch (InvalidDataException ex)
            {
                return Fail(ThemeDiagnostic.Error("package.corrupt", $"Pacote corrompido: {ex.Message}"));
            }
        }
    }

    private static ThemeInstallResult Install(ZipArchive archive, string themesRoot, ThemePackageLimits limits, IEnumerable<string>? reservedIds)
    {
        // 1. Inventário: só lê o diretório central do zip; nada é gravado ainda.
        if (archive.Entries.Count > limits.MaxEntries)
        {
            return Fail(ThemeDiagnostic.Error("package.too-many-entries", $"O pacote tem {archive.Entries.Count} entradas; o limite é {limits.MaxEntries}."));
        }

        var diagnostics = new List<ThemeDiagnostic>();
        var files = new List<(ZipArchiveEntry Entry, string Path)>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        long declared = 0;
        foreach (var entry in archive.Entries)
        {
            if (!ZipEntryPath.TryNormalize(entry.FullName, out string? relative, out string? problem))
            {
                return Fail(ThemeDiagnostic.Error("package.unsafe-path", $"Entrada \"{entry.FullName}\" recusada: {problem}."));
            }

            if (relative is null)
            {
                continue; // pasta
            }

            if (!seen.Add(relative))
            {
                return Fail(ThemeDiagnostic.Error("package.duplicate-entry", $"Entrada \"{relative}\" aparece mais de uma vez no pacote."));
            }

            if (entry.Length > limits.MaxFileBytes)
            {
                return Fail(ThemeDiagnostic.Error("package.file-too-large", $"\"{relative}\" tem {Mb(entry.Length)}; o limite por arquivo é {Mb(limits.MaxFileBytes)}."));
            }

            declared += entry.Length;
            if (declared > limits.MaxTotalBytes)
            {
                return Fail(ThemeDiagnostic.Error("package.too-large", $"O tema descompactado passa de {Mb(limits.MaxTotalBytes)}."));
            }

            files.Add((entry, relative));
        }

        // 2. Manifesto na raiz ou dentro de uma única pasta de topo (pacotes feitos zipando a pasta do tema).
        string? prefix = FindRoot(files.Select(f => f.Path).ToList());
        if (prefix is null)
        {
            return Fail(ThemeDiagnostic.Error("package.missing-manifest", "theme.yaml não encontrado na raiz do pacote. Não parece ser um tema do Playnite."));
        }

        var manifestEntry = files.First(f => string.Equals(f.Path, prefix + ThemeManifest.FileName, StringComparison.OrdinalIgnoreCase)).Entry;
        if (manifestEntry.Length > ThemeManifest.MaxBytes)
        {
            return Fail(ThemeDiagnostic.Error("manifest.too-large", $"theme.yaml tem mais de {ThemeManifest.MaxBytes / 1024} KB."));
        }

        var manifest = ThemeManifestValidator.Validate(ThemeManifest.Parse(ReadText(manifestEntry, ThemeManifest.MaxBytes)));
        diagnostics.AddRange(manifest.Diagnostics);
        if (!manifest.IsCompatible)
        {
            return new ThemeInstallResult(false, manifest.Manifest, null, 0, 0, diagnostics);
        }

        string id = manifest.Manifest.Id!; // IsCompatible garante Id presente e seguro.
        if (reservedIds?.Contains(id, StringComparer.OrdinalIgnoreCase) == true)
        {
            diagnostics.Add(ThemeDiagnostic.Error("package.reserved-id", $"O Id \"{id}\" pertence a um tema embutido do Tako e não pode ser substituído."));
            return new ThemeInstallResult(false, manifest.Manifest, null, 0, 0, diagnostics);
        }

        // 3. Extração para uma pasta temporária dentro da raiz de temas (mesmo volume: a troca final é um rename).
        Directory.CreateDirectory(themesRoot);
        string staging = Path.Combine(themesRoot, ".import-" + Guid.NewGuid().ToString("N"));
        int count = 0;
        long total = 0;
        try
        {
            Directory.CreateDirectory(staging);
            foreach (var (entry, path) in files)
            {
                if (!path.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                string relative = path[prefix.Length..];
                if (BlockedExtensions.Contains(Path.GetExtension(relative)))
                {
                    diagnostics.Add(ThemeDiagnostic.Warning("package.skipped-executable", $"\"{relative}\" não foi extraído: temas não precisam de arquivos executáveis."));
                    continue;
                }

                string target = ResolveInside(staging, relative);
                Directory.CreateDirectory(Path.GetDirectoryName(target)!);
                total += CopyBounded(entry, target, Math.Min(limits.MaxFileBytes, limits.MaxTotalBytes - total), relative);
                count++;
            }

            // 4. Troca: a versão anterior do mesmo tema só sai depois que a nova está completa.
            string destination = ResolveInside(themesRoot, id);
            string? previous = null;
            if (Directory.Exists(destination))
            {
                previous = Path.Combine(themesRoot, ".old-" + Guid.NewGuid().ToString("N"));
                Directory.Move(destination, previous);
            }

            Directory.Move(staging, destination);
            if (previous is not null)
            {
                TryDelete(previous);
            }

            return new ThemeInstallResult(true, manifest.Manifest, destination, count, total, diagnostics);
        }
        catch (Exception ex) when (ex is PackageLimitException or IOException or UnauthorizedAccessException or InvalidDataException)
        {
            TryDelete(staging);
            string code = ex is PackageLimitException ? "package.too-large" : "package.extract-failed";
            diagnostics.Add(ThemeDiagnostic.Error(code, ex is PackageLimitException ? ex.Message : $"Falha ao extrair o tema: {ex.Message}"));
            return new ThemeInstallResult(false, manifest.Manifest, null, 0, 0, diagnostics);
        }
    }

    /// <summary>Remove um tema importado. Só apaga pastas diretamente dentro de <paramref name="themesRoot"/>.</summary>
    public static void Remove(string themesRoot, string themeDirectory)
    {
        string root = Path.GetFullPath(themesRoot);
        string directory = Path.GetFullPath(themeDirectory).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        if (!string.Equals(Path.GetDirectoryName(directory), root.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar), StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException($"\"{themeDirectory}\" não é um tema importado em \"{themesRoot}\".");
        }

        if (Directory.Exists(directory))
        {
            Directory.Delete(directory, recursive: true);
        }
    }

    /// <summary>Prefixo ("" ou "Pasta/") onde está o theme.yaml; nulo se não houver.</summary>
    private static string? FindRoot(List<string> paths)
    {
        if (paths.Contains(ThemeManifest.FileName, StringComparer.OrdinalIgnoreCase))
        {
            return string.Empty;
        }

        var tops = paths.Select(p => p.Split('/')[0]).Distinct(StringComparer.OrdinalIgnoreCase).ToList();
        if (tops.Count == 1 && paths.All(p => p.Contains('/', StringComparison.Ordinal)))
        {
            string prefix = tops[0] + "/";
            return paths.Contains(prefix + ThemeManifest.FileName, StringComparer.OrdinalIgnoreCase) ? prefix : null;
        }

        return null;
    }

    /// <summary>Segunda barreira contra zip-slip: o caminho final precisa ficar dentro de <paramref name="root"/>.</summary>
    private static string ResolveInside(string root, string relative)
    {
        string full = Path.GetFullPath(Path.Combine(root, relative));
        string prefix = Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
        if (!full.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException($"\"{relative}\" sai da pasta do tema.");
        }

        return full;
    }

    /// <summary>Copia contando os bytes reais: zip bombs e tamanhos declarados falsos param no limite.</summary>
    private static long CopyBounded(ZipArchiveEntry entry, string target, long limit, string name)
    {
        using var source = entry.Open();
        using var destination = new FileStream(target, FileMode.CreateNew, FileAccess.Write);
        var buffer = new byte[81920];
        long written = 0;
        int read;
        while ((read = source.Read(buffer, 0, buffer.Length)) > 0)
        {
            written += read;
            if (written > limit)
            {
                throw new PackageLimitException($"\"{name}\" passa do limite de tamanho do tema ao descompactar.");
            }

            destination.Write(buffer, 0, read);
        }

        return written;
    }

    private static string ReadText(ZipArchiveEntry entry, int maxBytes)
    {
        using var source = entry.Open();
        using var memory = new MemoryStream();
        var buffer = new byte[8192];
        int read;
        while ((read = source.Read(buffer, 0, buffer.Length)) > 0)
        {
            if (memory.Length + read > maxBytes)
            {
                throw new InvalidDataException("theme.yaml maior que o declarado.");
            }

            memory.Write(buffer, 0, read);
        }

        memory.Position = 0;
        using var reader = new StreamReader(memory, detectEncodingFromByteOrderMarks: true);
        return reader.ReadToEnd();
    }

    private static void TryDelete(string directory)
    {
        try
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, recursive: true);
            }
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Sobra numa pasta oculta (.import-*/.old-*); a listagem de temas ignora pastas que começam com ponto.
        }
    }

    private static ThemeInstallResult Fail(ThemeDiagnostic diagnostic) => new(false, null, null, 0, 0, [diagnostic]);

    private static string Mb(long bytes) => $"{bytes / (1024.0 * 1024.0):0.#} MB";

    private sealed class PackageLimitException(string message) : Exception(message);
}

/// <summary>Validação dos nomes de entrada de um zip antes de qualquer gravação (zip-slip e nomes perigosos no Windows).</summary>
public static class ZipEntryPath
{
    public const int MaxLength = 240;

    private static readonly System.Buffers.SearchValues<char> InvalidChars = System.Buffers.SearchValues.Create("<>:\"|?*");

    /// <summary>
    /// Normaliza para "a/b/c". Retorna falso (com o motivo) para caminhos absolutos, com unidade, UNC, <c>..</c>,
    /// caracteres inválidos, nomes de dispositivo ou segmentos terminados em ponto/espaço; <paramref name="relative"/>
    /// fica nulo para entradas de pasta.
    /// </summary>
    public static bool TryNormalize(string name, out string? relative, out string? problem)
    {
        relative = null;
        problem = null;
        if (string.IsNullOrEmpty(name))
        {
            problem = "nome vazio";
            return false;
        }

        string path = name.Replace('\\', '/');
        if (path.StartsWith('/'))
        {
            problem = "caminho absoluto";
            return false;
        }

        if (path.Length > MaxLength)
        {
            problem = "caminho longo demais";
            return false;
        }

        var segments = new List<string>();
        foreach (string segment in path.Split('/'))
        {
            if (segment.Length == 0 || segment == ".")
            {
                continue;
            }

            if (segment == "..")
            {
                problem = "sai da pasta do tema (..)";
                return false;
            }

            if (segment.Any(c => char.IsControl(c)) || segment.AsSpan().IndexOfAny(InvalidChars) >= 0)
            {
                problem = segment.Contains(':', StringComparison.Ordinal) ? "unidade ou fluxo alternativo (:) no caminho" : "caractere inválido no nome";
                return false;
            }

            if (segment[^1] is '.' or ' ')
            {
                problem = "nome terminado em ponto ou espaço";
                return false;
            }

            if (ThemeId.IsDeviceName(segment.Split('.')[0]))
            {
                problem = "nome reservado do Windows";
                return false;
            }

            segments.Add(segment);
        }

        if (segments.Count == 0)
        {
            return true;
        }

        if (path.EndsWith('/'))
        {
            return true; // pasta
        }

        relative = string.Join('/', segments);
        return true;
    }
}
