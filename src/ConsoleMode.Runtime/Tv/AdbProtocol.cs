// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/Tv/AdbProtocol.cs e src/ConsoleMode/Services/Tv/AdbShellOutput.cs
using System.Buffers.Binary;
using System.Numerics;
using System.Security.Cryptography;
using System.Text;

namespace ConsoleMode.Runtime.Tv;

/// <summary>
/// O pedaço do protocolo do ADB (o mesmo do <c>adb connect</c> na porta 5555) que o Tako usa para mandar teclas a uma
/// Android TV / Google TV, sem distribuir o adb.exe. Lógica pura: enquadramento, assinatura da autenticação e o formato
/// de chave pública do Android.
/// </summary>
public static class AdbProtocol
{
    public const uint Connect = 0x4e584e43; // CNXN
    public const uint Auth = 0x48545541; // AUTH
    public const uint Open = 0x4e45504f; // OPEN
    public const uint Okay = 0x59414b4f; // OKAY
    public const uint Close = 0x45534c43; // CLSE
    public const uint Write = 0x45545257; // WRTE
    public const uint StartTls = 0x534c5453; // STLS

    public const uint AuthToken = 1;
    public const uint AuthSignature = 2;
    public const uint AuthRsaPublicKey = 3;

    /// <summary>Versão com checksum do conteúdo, que todo aparelho ainda aceita.</summary>
    public const uint Version = 0x01000000;
    public const uint MaxPayload = 256 * 1024;
    public const int MaxShellOutput = 1024 * 1024;
    public const int HeaderSize = 24;
    public const int DefaultPort = 5555;

    // Códigos de android.view.KeyEvent.
    public const int KeyWakeUp = 224;
    public const int KeySleep = 223;
    public const int KeyTvInputHdmi1 = 243;

    public readonly record struct Message(uint Command, uint Arg0, uint Arg1, byte[] Data);

    public static byte[] Encode(uint command, uint arg0, uint arg1, byte[]? data = null)
    {
        data ??= [];
        var buffer = new byte[HeaderSize + data.Length];
        var span = buffer.AsSpan();
        BinaryPrimitives.WriteUInt32LittleEndian(span, command);
        BinaryPrimitives.WriteUInt32LittleEndian(span[4..], arg0);
        BinaryPrimitives.WriteUInt32LittleEndian(span[8..], arg1);
        BinaryPrimitives.WriteUInt32LittleEndian(span[12..], (uint)data.Length);
        BinaryPrimitives.WriteUInt32LittleEndian(span[16..], Checksum(data));
        BinaryPrimitives.WriteUInt32LittleEndian(span[20..], command ^ 0xFFFFFFFF);
        data.CopyTo(buffer, HeaderSize);
        return buffer;
    }

    /// <summary>Lê um cabeçalho: comando, argumentos, tamanho e checksum do conteúdo.</summary>
    public static (uint Command, uint Arg0, uint Arg1, int Length, uint Checksum) DecodeHeader(ReadOnlySpan<byte> header)
    {
        if (header.Length < HeaderSize)
        {
            throw new InvalidDataException("ADB: cabeçalho curto");
        }

        uint command = BinaryPrimitives.ReadUInt32LittleEndian(header);
        uint magic = BinaryPrimitives.ReadUInt32LittleEndian(header[20..]);
        if (magic != (command ^ 0xFFFFFFFF))
        {
            throw new InvalidDataException("ADB: cabeçalho inválido");
        }

        uint length = BinaryPrimitives.ReadUInt32LittleEndian(header[12..]);
        if (length > MaxPayload)
        {
            throw new InvalidDataException("ADB: mensagem grande demais");
        }

        return (command,
            BinaryPrimitives.ReadUInt32LittleEndian(header[4..]),
            BinaryPrimitives.ReadUInt32LittleEndian(header[8..]),
            (int)length,
            BinaryPrimitives.ReadUInt32LittleEndian(header[16..]));
    }

    public static void ValidatePayload(uint expectedChecksum, ReadOnlySpan<byte> data)
    {
        if (Checksum(data) != expectedChecksum)
        {
            throw new InvalidDataException("ADB: checksum do pacote inválido");
        }
    }

    public static uint Checksum(ReadOnlySpan<byte> data)
    {
        uint sum = 0;
        foreach (byte b in data)
        {
            sum += b;
        }

        return sum;
    }

    /// <summary>Banner "host::" enviado com o CNXN.</summary>
    public static byte[] ConnectBanner() => Encoding.ASCII.GetBytes("host::\0");

    public static byte[] ShellService(string command) => Encoding.UTF8.GetBytes($"shell:{command}\0");

    /// <summary>
    /// O adb assina o token de 20 bytes como se fosse um resumo SHA-1 (RSA_sign com NID_sha1): PKCS#1 v1.5 sobre o
    /// DigestInfo SHA-1 do token cru.
    /// </summary>
    public static byte[] SignToken(RSA key, byte[] token)
    {
        ArgumentNullException.ThrowIfNull(key);
        return key.SignHash(token, HashAlgorithmName.SHA1, RSASignaturePadding.Pkcs1);
    }

    /// <summary>
    /// Estrutura RSAPublicKey do Android (android_pubkey_encode) em base64, seguida de " comentário" e NUL: é o que a TV
    /// mostra em "Permitir depuração deste computador?".
    /// </summary>
    public static byte[] EncodePublicKey(RSAParameters key, string comment)
    {
        byte[] modulus = key.Modulus ?? throw new ArgumentException("Chave sem módulo", nameof(key));
        byte[] exponent = key.Exponent ?? throw new ArgumentException("Chave sem expoente", nameof(key));
        if (modulus.Length != 256)
        {
            throw new ArgumentException("Chaves ADB são RSA-2048", nameof(key));
        }

        var n = new BigInteger(modulus, isUnsigned: true, isBigEndian: true);
        uint n0 = BinaryPrimitives.ReadUInt32BigEndian(modulus.AsSpan(modulus.Length - 4));

        // Iteração de Newton para o inverso de um número ímpar módulo 2^32.
        uint inverse = n0;
        for (int i = 0; i < 5; i++)
        {
            inverse = unchecked(inverse * (2 - (n0 * inverse)));
        }

        uint n0Inverse = unchecked(0u - inverse);
        var rr = BigInteger.ModPow(2, 2 * 2048, n);

        var buffer = new byte[4 + 4 + 256 + 256 + 4];
        var span = buffer.AsSpan();
        BinaryPrimitives.WriteUInt32LittleEndian(span, 256 / 4);
        BinaryPrimitives.WriteUInt32LittleEndian(span[4..], n0Inverse);
        WriteLittleEndian(n, span.Slice(8, 256));
        WriteLittleEndian(rr, span.Slice(264, 256));
        uint e = 0;
        foreach (byte b in exponent)
        {
            e = (e << 8) | b;
        }

        BinaryPrimitives.WriteUInt32LittleEndian(span[520..], e);
        return Encoding.ASCII.GetBytes($"{Convert.ToBase64String(buffer)} {comment}\0");
    }

    /// <summary>"host" ou "host:porta" (padrão 5555).</summary>
    public static (string Host, int Port) ParseEndpoint(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        string text = value.Trim();
        int colon = text.LastIndexOf(':');
        if (colon > 0 && text.IndexOf(':', StringComparison.Ordinal) == colon
            && int.TryParse(text[(colon + 1)..], System.Globalization.NumberStyles.None, System.Globalization.CultureInfo.InvariantCulture, out int port)
            && port is > 0 and < 65536)
        {
            return (text[..colon], port);
        }

        return (text, DefaultPort);
    }

    /// <summary>Código da tecla "HDMI n" (1 a 4).</summary>
    public static int HdmiKey(int input) => KeyTvInputHdmi1 + Math.Clamp(input, 1, 4) - 1;

    private static void WriteLittleEndian(BigInteger value, Span<byte> destination)
    {
        destination.Clear();
        byte[] bytes = value.ToByteArray(isUnsigned: true, isBigEndian: false);
        bytes.AsSpan(0, Math.Min(bytes.Length, destination.Length)).CopyTo(destination);
    }
}

/// <summary>Acumula, com limite, a saída de um comando de shell remoto.</summary>
public sealed class AdbShellOutput
{
    private readonly StringBuilder _text = new();
    private int _bytes;

    public void Append(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length > AdbProtocol.MaxShellOutput - _bytes)
        {
            throw new InvalidDataException("ADB: a saída do comando passou do limite de 1 MiB");
        }

        _bytes += data.Length;
        _text.Append(Encoding.UTF8.GetString(data));
    }

    public override string ToString() => _text.ToString();
}
