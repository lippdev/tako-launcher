// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/SecretProtector.cs
using System.Runtime.InteropServices;
using System.Text;
using ConsoleMode.Runtime.Tv;

namespace ConsoleMode.Runtime.Windows;

/// <summary>
/// Segredos em disco (a chave ADB da TV) cifrados com DPAPI para o usuário atual do Windows: a pasta de dados pode ser
/// copiada com o Tako portátil, e o texto puro iria junto. Só o mesmo usuário no mesmo PC consegue ler; qualquer outro
/// caso volta como "ilegível" e quem chama cria um segredo novo. Chamada direta a <c>CryptProtectData</c>, sem pacote.
/// </summary>
public sealed partial class DpapiSecretProtector : ISecretProtector
{
    public const string Prefix = "dpapi:";

    /// <summary>Sem interface: falha em vez de mostrar diálogo.</summary>
    private const uint UiForbidden = 0x1;

    public string Protect(string secret)
    {
        ArgumentNullException.ThrowIfNull(secret);
        if (secret.Length == 0)
        {
            return string.Empty;
        }

        byte[] output = Transform(Encoding.UTF8.GetBytes(secret), protect: true)
            ?? throw new RuntimeEnvironmentException($"CryptProtectData falhou (erro {Marshal.GetLastPInvokeError()})");
        return Prefix + Convert.ToBase64String(output);
    }

    /// <summary>Texto que nunca foi protegido passa direto. Falso quando é de outro usuário ou PC.</summary>
    public bool TryUnprotect(string stored, out string secret)
    {
        secret = string.Empty;
        if (string.IsNullOrEmpty(stored))
        {
            return true;
        }

        if (!stored.StartsWith(Prefix, StringComparison.Ordinal))
        {
            secret = stored;
            return true;
        }

        byte[] data;
        try
        {
            data = Convert.FromBase64String(stored[Prefix.Length..]);
        }
        catch (FormatException)
        {
            return false;
        }

        if (Transform(data, protect: false) is not { } plain)
        {
            return false;
        }

        secret = Encoding.UTF8.GetString(plain);
        return true;
    }

    private static unsafe byte[]? Transform(byte[] input, bool protect)
    {
        fixed (byte* pointer = input)
        {
            var source = new DataBlob { Size = (uint)input.Length, Data = (nint)pointer };
            DataBlob result = default;
            bool ok = protect
                ? CryptProtectData(&source, null, null, 0, 0, UiForbidden, &result)
                : CryptUnprotectData(&source, 0, null, 0, 0, UiForbidden, &result);
            if (!ok)
            {
                return null;
            }

            try
            {
                var output = new byte[result.Size];
                Marshal.Copy(result.Data, output, 0, output.Length);
                return output;
            }
            finally
            {
                _ = LocalFree(result.Data);
            }
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct DataBlob
    {
        public uint Size;
        public nint Data;
    }

    [LibraryImport("crypt32.dll", SetLastError = true, StringMarshalling = StringMarshalling.Utf16)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static unsafe partial bool CryptProtectData(DataBlob* dataIn, string? description, DataBlob* entropy, nint reserved, nint prompt, uint flags, DataBlob* dataOut);

    [LibraryImport("crypt32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static unsafe partial bool CryptUnprotectData(DataBlob* dataIn, nint description, DataBlob* entropy, nint reserved, nint prompt, uint flags, DataBlob* dataOut);

    [LibraryImport("kernel32.dll")]
    private static partial nint LocalFree(nint memory);
}
