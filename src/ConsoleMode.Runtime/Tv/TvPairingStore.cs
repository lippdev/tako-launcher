using System.Text.Json;

namespace ConsoleMode.Runtime.Tv;

/// <summary>
/// Chaves de pareamento das TVs LG (client-key) e Samsung (token), por tipo e IP, num arquivo da pasta de dados. O
/// arquivo inteiro fica protegido (<see cref="ISecretProtector"/>, DPAPI no Windows), como a chave ADB: quem tem a chave
/// controla a TV. Arquivo ilegível (outro usuário/PC, corrompido) conta como vazio, e a TV pede permissão de novo.
/// </summary>
public sealed class TvPairingStore(string path, ISecretProtector protector)
{
    private readonly object _gate = new();

    public string FilePath { get; } = path;

    public string? Get(TvKind kind, string host)
    {
        lock (_gate)
        {
            return Load().GetValueOrDefault(Key(kind, host));
        }
    }

    public void Set(TvKind kind, string host, string secret)
    {
        ArgumentException.ThrowIfNullOrEmpty(secret);
        lock (_gate)
        {
            var all = Load();
            all[Key(kind, host)] = secret;
            Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(FilePath))!); // caminho completo sempre tem pasta
            File.WriteAllText(FilePath, protector.Protect(JsonSerializer.Serialize(all)));
        }
    }

    public static string Key(TvKind kind, string host)
    {
        ArgumentNullException.ThrowIfNull(host);
        return $"{kind}:{host.Trim().ToUpperInvariant()}";
    }

    private Dictionary<string, string> Load()
    {
        try
        {
            if (File.Exists(FilePath) && protector.TryUnprotect(File.ReadAllText(FilePath).Trim(), out string json) && json.Length > 0)
            {
                return JsonSerializer.Deserialize<Dictionary<string, string>>(json) ?? [];
            }
        }
        catch (Exception ex) when (ex is JsonException or IOException or UnauthorizedAccessException)
        {
            // Ilegível: parear de novo só faz a TV pedir permissão outra vez.
        }

        return [];
    }
}
