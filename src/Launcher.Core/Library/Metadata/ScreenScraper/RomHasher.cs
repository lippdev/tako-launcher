using System.Collections.Concurrent;
using System.Globalization;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;

namespace Launcher.Core.Library.Metadata.ScreenScraper;

/// <summary>
/// Identidade de uma ROM para o <c>jeuInfos.php</c>: nome do arquivo, tamanho e os hashes que valeram a pena calcular
/// (nulos quando não calculados; ver <see cref="RomHasher"/>). Hashes em hexadecimal maiúsculo, como o ScreenScraper usa.
/// </summary>
public sealed record RomIdentity(string FileName, long Size, string? Crc32 = null, string? Md5 = null, string? Sha1 = null);

/// <summary>CRC-32 (IEEE 802.3, o mesmo do zip e dos dats No-Intro) incremental; a BCL não tem um sem pacote extra.</summary>
public sealed class Crc32
{
    private static readonly uint[] Table = BuildTable();
    private uint _value = 0xFFFFFFFFu;

    public void Append(ReadOnlySpan<byte> data)
    {
        uint crc = _value;
        foreach (byte b in data)
        {
            crc = Table[(crc ^ b) & 0xFF] ^ (crc >> 8);
        }

        _value = crc;
    }

    public uint Value => ~_value;

    public string Hex => Value.ToString("X8", CultureInfo.InvariantCulture);

    public static string Compute(ReadOnlySpan<byte> data)
    {
        var crc = new Crc32();
        crc.Append(data);
        return crc.Hex;
    }

    private static uint[] BuildTable()
    {
        var table = new uint[256];
        for (uint i = 0; i < 256; i++)
        {
            uint c = i;
            for (int k = 0; k < 8; k++)
            {
                c = (c & 1) != 0 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
            }

            table[i] = c;
        }

        return table;
    }
}

/// <summary>
/// Calcula a identidade da ROM em streaming (um passe, buffer de 1 MB), sempre fora da thread de quem chama (ADR-0022):
/// <list type="bullet">
/// <item>até <see cref="FullHashLimit"/> (cartuchos, a maioria): CRC32 + MD5 + SHA1;</item>
/// <item>acima disso (ISOs): só CRC32, que o ScreenScraper aceita junto com nome e tamanho;</item>
/// <item>imagens de disco compactadas (.chd, .rvz, .cso...): só nome e tamanho, pois o hash do contêiner não identifica o jogo;</item>
/// <item>.zip de sistema de cartucho com um arquivo só: hashes do arquivo de dentro (é o que os dats registram); .zip do
/// arcade/Neo Geo: hashes do próprio .zip (o conjunto do MAME é o arquivo). .7z: só nome e tamanho.</item>
/// </list>
/// </summary>
public static class RomHasher
{
    public const long FullHashLimit = 64L * 1024 * 1024;

    private const int BufferSize = 1024 * 1024;

    public static Task<RomIdentity> IdentifyAsync(string path, EmulatedSystem? system, CancellationToken cancellationToken)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);
        return Task.Run(() => Identify(path, system, cancellationToken), cancellationToken);
    }

    /// <summary>Versão síncrona (testes e quem já está fora da UI).</summary>
    public static RomIdentity Identify(string path, EmulatedSystem? system, CancellationToken cancellationToken = default)
    {
        var file = new FileInfo(path);
        string extension = file.Extension.ToLowerInvariant();
        if (ScreenScraperSystems.CompressedDiscExtensions.Contains(extension) || extension == ".7z")
        {
            return new RomIdentity(file.Name, file.Length);
        }

        if (extension == ".zip" && system is not { ArchiveIsRom: true } && TryIdentifyInside(file, cancellationToken) is { } inner)
        {
            return inner;
        }

        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read, BufferSize, FileOptions.SequentialScan);
        return Hash(stream, file.Name, file.Length, full: file.Length <= FullHashLimit, cancellationToken);
    }

    /// <summary>Hash de um fluxo já aberto (puro; testado com bytes em memória).</summary>
    public static RomIdentity Hash(Stream stream, string fileName, long size, bool full, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(stream);
        var crc = new Crc32();
        using var md5 = full ? IncrementalHash.CreateHash(HashAlgorithmName.MD5) : null;
        using var sha1 = full ? IncrementalHash.CreateHash(HashAlgorithmName.SHA1) : null;
        byte[] buffer = new byte[BufferSize];
        int read;
        while ((read = stream.Read(buffer, 0, buffer.Length)) > 0)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var chunk = buffer.AsSpan(0, read);
            crc.Append(chunk);
            md5?.AppendData(chunk);
            sha1?.AppendData(chunk);
        }

        return new RomIdentity(fileName, size, crc.Hex, md5 is null ? null : Convert.ToHexString(md5.GetHashAndReset()), sha1 is null ? null : Convert.ToHexString(sha1.GetHashAndReset()));
    }

    private static RomIdentity? TryIdentifyInside(FileInfo file, CancellationToken cancellationToken)
    {
        try
        {
            using var archive = ZipFile.OpenRead(file.FullName);
            var entries = archive.Entries.Where(e => e.Length > 0 && !e.FullName.EndsWith('/')).ToList();
            if (entries.Count != 1)
            {
                return null; // vários arquivos: hash do .zip inteiro
            }

            var entry = entries[0];
            using var stream = entry.Open();
            return Hash(stream, entry.Name, entry.Length, full: entry.Length <= FullHashLimit, cancellationToken);
        }
        catch (InvalidDataException)
        {
            return null; // .zip corrompido ou com compressão desconhecida: hash do arquivo como está
        }
    }
}

/// <summary>
/// Cache das identidades por caminho + tamanho + data de modificação (ADR-0022): uma ISO de 4 GB não é lida de novo a cada
/// atualização. Guarda num arquivo JSON da pasta de cache quando <c>path</c> é informado; arquivo ilegível = cache vazio.
/// </summary>
public sealed class RomHashCache(string? path = null)
{
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = false };
    private readonly object _gate = new();
    private ConcurrentDictionary<string, RomIdentity>? _entries;

    public async Task<RomIdentity> GetAsync(string romPath, EmulatedSystem? system, CancellationToken cancellationToken)
    {
        ArgumentException.ThrowIfNullOrEmpty(romPath);
        var file = new FileInfo(romPath);
        string key = KeyFor(file, system);
        var entries = Entries();
        if (entries.TryGetValue(key, out var known))
        {
            return known;
        }

        var identity = await RomHasher.IdentifyAsync(romPath, system, cancellationToken).ConfigureAwait(false);
        entries[key] = identity;
        Save(entries);
        return identity;
    }

    /// <summary>Chave: caminho completo, tamanho, data (UTC) e se o .zip é a ROM (muda o que é calculado).</summary>
    public static string KeyFor(FileInfo file, EmulatedSystem? system)
    {
        ArgumentNullException.ThrowIfNull(file);
        return string.Join('|', file.FullName.ToUpperInvariant(), file.Length.ToString(CultureInfo.InvariantCulture), file.LastWriteTimeUtc.Ticks.ToString(CultureInfo.InvariantCulture), system?.ArchiveIsRom == true ? "a" : "r");
    }

    private ConcurrentDictionary<string, RomIdentity> Entries()
    {
        lock (_gate)
        {
            return _entries ??= Load();
        }
    }

    private ConcurrentDictionary<string, RomIdentity> Load()
    {
        try
        {
            if (path is not null && File.Exists(path) && JsonSerializer.Deserialize<Dictionary<string, RomIdentity>>(File.ReadAllText(path), Options) is { } stored)
            {
                return new ConcurrentDictionary<string, RomIdentity>(stored, StringComparer.Ordinal);
            }
        }
        catch (Exception ex) when (ex is JsonException or IOException or UnauthorizedAccessException)
        {
            // Cache ilegível: as ROMs são lidas de novo.
        }

        return new ConcurrentDictionary<string, RomIdentity>(StringComparer.Ordinal);
    }

    private void Save(ConcurrentDictionary<string, RomIdentity> entries)
    {
        if (path is null)
        {
            return;
        }

        lock (_gate)
        {
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(path))!);
                string temp = path + ".tmp";
                File.WriteAllText(temp, JsonSerializer.Serialize(new Dictionary<string, RomIdentity>(entries, StringComparer.Ordinal), Options));
                File.Move(temp, path, overwrite: true);
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                // Sem cache em disco, o hash é recalculado na próxima abertura.
            }
        }
    }
}
