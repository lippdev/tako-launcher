// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/Tv/AdbClient.cs (AdbKeyStore) e src/ConsoleMode/Services/SecretProtector.cs
using System.Security.Cryptography;

namespace ConsoleMode.Runtime.Tv;

/// <summary>
/// Protege segredos gravados em disco para o usuário atual do Windows (DPAPI na implementação da plataforma).
/// <see cref="TryUnprotect"/> devolve falso quando o texto foi protegido em outro usuário ou PC.
/// </summary>
public interface ISecretProtector
{
    string Protect(string secret);

    bool TryUnprotect(string stored, out string secret);
}

/// <summary>Como a chave foi obtida (para o log).</summary>
public enum AdbKeyOrigin
{
    /// <summary>Chave já pareada, lida do disco.</summary>
    Loaded,

    /// <summary>Primeira vez: chave nova (a TV vai pedir "Permitir depuração").</summary>
    Created,

    /// <summary>A chave do disco era ilegível (outro usuário/PC ou arquivo corrompido): uma nova foi criada.</summary>
    Replaced,
}

/// <summary>
/// Chave ADB deste PC (RSA-2048), guardada na pasta de dados para a TV pedir permissão uma vez só. A chave privada fica
/// protegida (<see cref="ISecretProtector"/>): quem tem a chave controla a TV pareada.
/// </summary>
public sealed class AdbKeyStore(string path, ISecretProtector protector)
{
    private readonly object _gate = new();

    public string FilePath { get; } = path;

    /// <summary>A chave salva ou uma nova; quem chama descarta a chave.</summary>
    public (RSA Key, AdbKeyOrigin Origin) LoadOrCreate()
    {
        lock (_gate)
        {
            bool existed = File.Exists(FilePath);
            if (existed && TryLoad() is { } saved)
            {
                return (saved, AdbKeyOrigin.Loaded);
            }

            var created = RSA.Create(2048);
            Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(FilePath))!); // caminho completo sempre tem pasta
            File.WriteAllText(FilePath, protector.Protect(created.ExportPkcs8PrivateKeyPem()));
            return (created, existed ? AdbKeyOrigin.Replaced : AdbKeyOrigin.Created);
        }
    }

    private RSA? TryLoad()
    {
        var key = RSA.Create();
        try
        {
            string stored = File.ReadAllText(FilePath).Trim();
            if (protector.TryUnprotect(stored, out string pem) && pem.Length > 0)
            {
                key.ImportFromPem(pem);
                if (key.KeySize == 2048)
                {
                    return key;
                }
            }
        }
        catch (Exception ex) when (ex is CryptographicException or ArgumentException or IOException or UnauthorizedAccessException)
        {
            // Ilegível: uma chave nova só faz a TV pedir permissão de novo.
        }

        key.Dispose();
        return null;
    }
}
