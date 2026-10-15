using System.Security.Cryptography;
using System.Text;

namespace Launcher.Core.Library.Xbox;

/// <summary>
/// Identidade de pacotes MSIX/AppX sem WinRT: nome da família (<c>Nome_IdDoPublicador</c>), leitura do nome completo e
/// o AUMID usado para abrir o jogo pelo Windows (<c>shell:AppsFolder\Família!App</c>), nunca pelo executável.
/// </summary>
public static class PackageIdentity
{
    // Base32 do Windows para o id do publicador (sem i, l, o, u).
    private const string Alphabet = "0123456789abcdefghjkmnpqrstvwxyz";

    /// <summary>
    /// Id do publicador como o Windows calcula: SHA-256 do <c>Publisher</c> (UTF-16LE), primeiros 8 bytes, 13 caracteres
    /// em base32 (64 bits + 1 bit zero). Ex.: "CN=Microsoft Corporation, …" → <c>8wekyb3d8bbwe</c>.
    /// </summary>
    public static string PublisherId(string publisher)
    {
        ArgumentNullException.ThrowIfNull(publisher);
        byte[] hash = SHA256.HashData(Encoding.Unicode.GetBytes(publisher));
        ulong bits = 0;
        for (int i = 0; i < 8; i++)
        {
            bits = (bits << 8) | hash[i];
        }

        var id = new StringBuilder(13);
        for (int i = 0; i < 13; i++)
        {
            // 65 bits lidos de 5 em 5 a partir do mais significativo; o último grupo tem 4 bits + o zero de enchimento.
            int shift = 59 - (i * 5);
            int value = shift >= 0 ? (int)((bits >> shift) & 0x1F) : (int)((bits << 1) & 0x1F);
            id.Append(Alphabet[value]);
        }

        return id.ToString();
    }

    public static string FamilyName(string name, string publisher)
    {
        ArgumentNullException.ThrowIfNull(name);
        return $"{name}_{PublisherId(publisher)}";
    }

    /// <summary>
    /// Família a partir do nome completo (<c>Nome_Versão_Arquitetura_Recurso_IdDoPublicador</c>). Falso quando o texto
    /// não tem esse formato.
    /// </summary>
    public static bool TryFamilyFromFullName(string? fullName, out string family)
    {
        family = string.Empty;
        string[] parts = fullName?.Split('_') ?? [];
        if (parts.Length != 5 || parts[0].Length == 0 || parts[4].Length != 13)
        {
            return false;
        }

        family = $"{parts[0]}_{parts[4]}";
        return true;
    }

    /// <summary>Nome do pacote (antes do primeiro "_") de uma família ou nome completo.</summary>
    public static string NameOf(string familyOrFullName)
    {
        ArgumentNullException.ThrowIfNull(familyOrFullName);
        int index = familyOrFullName.IndexOf('_', StringComparison.Ordinal);
        return index < 0 ? familyOrFullName : familyOrFullName[..index];
    }

    /// <summary>Alvo para abrir o app pelo shell, como o menu Iniciar faz (respeita a proteção do pacote).</summary>
    public static string LaunchTarget(string family, string appId)
    {
        ArgumentNullException.ThrowIfNull(family);
        ArgumentNullException.ThrowIfNull(appId);
        return $@"shell:AppsFolder\{family}!{appId}";
    }

    /// <summary>
    /// Texto para <c>SHLoadIndirectString</c> que resolve um <c>ms-resource:</c> do manifesto no idioma do Windows.
    /// Nulo quando o valor não é um recurso (já é o texto final).
    /// </summary>
    public static string? IndirectString(string packageFullName, string packageName, string? value)
    {
        ArgumentNullException.ThrowIfNull(packageFullName);
        ArgumentNullException.ThrowIfNull(packageName);
        const string Prefix = "ms-resource:";
        if (value is null || !value.StartsWith(Prefix, StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        string key = value[Prefix.Length..].Trim();
        string uri = key.StartsWith("//", StringComparison.Ordinal) ? $"ms-resource:{key}"
            : key.StartsWith('/') ? $"ms-resource://{packageName}{key}"
            : key.Contains('/', StringComparison.Ordinal) ? $"ms-resource://{packageName}/{key}"
            : $"ms-resource://{packageName}/resources/{key}";
        return $"@{{{packageFullName}?{uri}}}";
    }
}
