using System.Buffers.Binary;
using System.Text;
using Launcher.Core.Library.Metadata.ScreenScraper;

namespace Launcher.Core.Presentation;

/// <summary>Configurações › Capas e metadados › Capas animadas (ADR-0023).</summary>
public enum AnimatedCoversMode
{
    /// <summary>Nunca anima: todas mostram o primeiro quadro.</summary>
    Off,

    /// <summary>Só a capa em foco e a arte dos detalhes (padrão).</summary>
    FocusedOnly,

    /// <summary>Todas as capas animadas visíveis.</summary>
    Always,
}

/// <summary>Quando uma capa animada se mexe (regra pura, testada).</summary>
public static class AnimationPolicy
{
    public static string Label(AnimatedCoversMode mode) => mode switch
    {
        AnimatedCoversMode.Off => "Desligado",
        AnimatedCoversMode.Always => "Sempre",
        _ => "Só na capa em foco",
    };

    public static AnimatedCoversMode Next(AnimatedCoversMode mode) => mode switch
    {
        AnimatedCoversMode.Off => AnimatedCoversMode.FocusedOnly,
        AnimatedCoversMode.FocusedOnly => AnimatedCoversMode.Always,
        _ => AnimatedCoversMode.Off,
    };

    public static AnimatedCoversMode Parse(string? value) =>
        Enum.TryParse<AnimatedCoversMode>(value, out var mode) && Enum.IsDefined(mode) ? mode : AnimatedCoversMode.FocusedOnly;

    /// <summary>
    /// Anima? Nunca durante uma sessão de jogo, com a janela do Tako fora de foco ou com "Reduzir animações"; senão, conforme
    /// o modo: só a capa em foco (ou a arte aberta nos detalhes) ou todas.
    /// </summary>
    public static bool ShouldAnimate(AnimatedCoversMode mode, bool isFocused, bool inGame, bool windowActive, bool reducedMotion) =>
        !inGame && windowActive && !reducedMotion && mode switch
        {
            AnimatedCoversMode.Always => true,
            AnimatedCoversMode.FocusedOnly => isFocused,
            _ => false,
        };
}

public enum AnimatedFormat
{
    Gif,
    Png,
    WebP,
}

/// <summary>O que fazer com a área do quadro antes do próximo (GIF, APNG e WebP usam os mesmos três casos).</summary>
public enum FrameDispose
{
    None,
    Background,
    Previous,
}

/// <summary>
/// Um quadro: posição e tamanho no quadro inteiro, quanto tempo fica (ms), se mistura com o que está embaixo (alfa) ou
/// substitui, e o que fazer depois. <see cref="Still"/> é uma imagem sozinha (PNG ou WebP) que o decodificador do
/// Windows abre; no GIF é nula (o decodificador de GIF abre o quadro pelo índice).
/// </summary>
public sealed record AnimatedFrame(int X, int Y, int Width, int Height, int DelayMs, FrameDispose Dispose, bool Blend, byte[]? Still);

public sealed record AnimatedImageInfo(AnimatedFormat Format, int Width, int Height, IReadOnlyList<AnimatedFrame> Frames)
{
    public bool IsAnimated => Frames.Count > 1;

    public int TotalMs => Frames.Sum(f => f.DelayMs);
}

/// <summary>
/// Lê a estrutura de GIF, PNG animado (APNG) e WebP animado sem decodificar os pixels (ADR-0023): quadros, tempos e
/// como compor. O WPF só mostra o primeiro quadro desses formatos; com isto, o Tako separa cada quadro numa imagem
/// sozinha que o WIC do Windows decodifica, e compõe com <see cref="FrameComposer"/>. Puro e testado com arquivos
/// pequenos gerados no teste. Arquivo estranho ou cortado: nulo (a capa fica estática).
/// </summary>
public static class AnimatedImageParser
{
    /// <summary>No máximo tantos quadros: o resto da animação é ignorado (memória).</summary>
    public const int MaxFrames = 240;

    private static readonly byte[] PngSignature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    /// <summary>Formato pelo começo do arquivo, sem ler o resto.</summary>
    public static AnimatedFormat? Sniff(ReadOnlySpan<byte> data) =>
        data.Length >= 8 && data[..8].SequenceEqual(PngSignature) ? AnimatedFormat.Png
        : data.Length >= 6 && (data[..6].SequenceEqual("GIF89a"u8) || data[..6].SequenceEqual("GIF87a"u8)) ? AnimatedFormat.Gif
        : data.Length >= 12 && data[..4].SequenceEqual("RIFF"u8) && data[8..12].SequenceEqual("WEBP"u8) ? AnimatedFormat.WebP
        : null;

    public static AnimatedImageInfo? Parse(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        try
        {
            return Sniff(data) switch
            {
                AnimatedFormat.Png => ParsePng(data),
                AnimatedFormat.Gif => ParseGif(data),
                AnimatedFormat.WebP => ParseWebP(data),
                _ => null,
            };
        }
        catch (Exception ex) when (ex is ArgumentOutOfRangeException or IndexOutOfRangeException or OverflowException or ArgumentException)
        {
            return null; // arquivo cortado ou corrompido
        }
    }

    /// <summary>Tempo de um quadro: navegadores tratam ≤ 10 ms como 100 ms; teto de 10 s.</summary>
    public static int NormalizeDelay(int ms) => ms <= 10 ? 100 : Math.Min(ms, 10_000);

    // ---------------------------------------------------------------- APNG

    private static AnimatedImageInfo? ParsePng(byte[] data)
    {
        int pos = 8;
        byte[]? ihdr = null;
        var shared = new List<(string Type, byte[] Body)>(); // PLTE, tRNS, gAMA...: vão em todo quadro
        var frames = new List<AnimatedFrame>();
        (int W, int H, int X, int Y, int Delay, FrameDispose Dispose, bool Blend)? control = null;
        var parts = new List<byte[]>();
        bool animated = false;
        bool seenData = false;

        void Flush()
        {
            if (control is { } c && parts.Count > 0 && ihdr is not null && frames.Count < MaxFrames)
            {
                frames.Add(new AnimatedFrame(c.X, c.Y, c.W, c.H, c.Delay, c.Dispose, c.Blend, BuildPng(ihdr, c.W, c.H, shared, parts)));
            }

            parts.Clear();
            control = null;
        }

        while (pos + 12 <= data.Length)
        {
            int length = checked((int)BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(pos)));
            string type = Encoding.ASCII.GetString(data, pos + 4, 4);
            var body = data.AsSpan(pos + 8, length);
            pos += 12 + length;
            switch (type)
            {
                case "IHDR":
                    ihdr = body.ToArray();
                    break;
                case "acTL":
                    animated = true;
                    break;
                case "fcTL":
                    Flush();
                    int den = BinaryPrimitives.ReadUInt16BigEndian(body[22..]);
                    int num = BinaryPrimitives.ReadUInt16BigEndian(body[20..]);
                    control = (
                        (int)BinaryPrimitives.ReadUInt32BigEndian(body[4..]),
                        (int)BinaryPrimitives.ReadUInt32BigEndian(body[8..]),
                        (int)BinaryPrimitives.ReadUInt32BigEndian(body[12..]),
                        (int)BinaryPrimitives.ReadUInt32BigEndian(body[16..]),
                        NormalizeDelay((int)Math.Round(num * 1000.0 / (den == 0 ? 100 : den))),
                        body[24] switch { 1 => FrameDispose.Background, 2 => FrameDispose.Previous, _ => FrameDispose.None },
                        body[25] == 1);
                    break;
                case "IDAT":
                    seenData = true;
                    if (control is not null)
                    {
                        parts.Add(body.ToArray()); // imagem padrão também é o primeiro quadro
                    }

                    break;
                case "fdAT":
                    parts.Add(body[4..].ToArray());
                    break;
                case "IEND":
                    pos = data.Length;
                    break;
                default:
                    if (!seenData && type is "PLTE" or "tRNS" or "gAMA" or "cHRM" or "sRGB" or "iCCP" or "sBIT")
                    {
                        shared.Add((type, body.ToArray()));
                    }

                    break;
            }
        }

        Flush();
        if (ihdr is null)
        {
            return null;
        }

        int width = (int)BinaryPrimitives.ReadUInt32BigEndian(ihdr);
        int height = (int)BinaryPrimitives.ReadUInt32BigEndian(ihdr.AsSpan(4));
        if (!animated || frames.Count == 0)
        {
            return new AnimatedImageInfo(AnimatedFormat.Png, width, height, []);
        }

        return new AnimatedImageInfo(AnimatedFormat.Png, width, height, frames);
    }

    private static byte[] BuildPng(byte[] ihdr, int width, int height, List<(string Type, byte[] Body)> shared, List<byte[]> data)
    {
        using var output = new MemoryStream();
        output.Write(PngSignature);
        var header = (byte[])ihdr.Clone();
        BinaryPrimitives.WriteUInt32BigEndian(header, (uint)width);
        BinaryPrimitives.WriteUInt32BigEndian(header.AsSpan(4), (uint)height);
        WriteChunk(output, "IHDR", header);
        foreach (var (type, body) in shared)
        {
            WriteChunk(output, type, body);
        }

        foreach (var part in data)
        {
            WriteChunk(output, "IDAT", part);
        }

        WriteChunk(output, "IEND", []);
        return output.ToArray();
    }

    /// <summary>Um bloco PNG com o tamanho e o CRC (também usado pelos testes para montar arquivos).</summary>
    public static void WriteChunk(Stream output, string type, ReadOnlySpan<byte> body)
    {
        ArgumentNullException.ThrowIfNull(output);
        Span<byte> number = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(number, (uint)body.Length);
        output.Write(number);
        byte[] typeBytes = Encoding.ASCII.GetBytes(type);
        output.Write(typeBytes);
        output.Write(body);
        var crc = new Crc32();
        crc.Append(typeBytes);
        crc.Append(body);
        BinaryPrimitives.WriteUInt32BigEndian(number, crc.Value);
        output.Write(number);
    }

    // ---------------------------------------------------------------- GIF

    private static AnimatedImageInfo? ParseGif(byte[] data)
    {
        int width = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(6));
        int height = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(8));
        byte flags = data[10];
        int pos = 13 + ((flags & 0x80) != 0 ? 3 * (1 << ((flags & 0x07) + 1)) : 0);
        var frames = new List<AnimatedFrame>();
        int delay = 100;
        var dispose = FrameDispose.None;
        while (pos < data.Length && frames.Count < MaxFrames)
        {
            byte block = data[pos++];
            if (block == 0x3B)
            {
                break;
            }

            if (block == 0x21)
            {
                byte label = data[pos++];
                if (label == 0xF9 && data[pos] >= 4)
                {
                    byte packed = data[pos + 1];
                    dispose = ((packed >> 2) & 0x07) switch { 2 => FrameDispose.Background, 3 => FrameDispose.Previous, _ => FrameDispose.None };
                    delay = NormalizeDelay(BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(pos + 2)) * 10);
                }

                pos = SkipSubBlocks(data, pos);
            }
            else if (block == 0x2C)
            {
                int x = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(pos));
                int y = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(pos + 2));
                int w = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(pos + 4));
                int h = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(pos + 6));
                byte local = data[pos + 8];
                pos += 9 + ((local & 0x80) != 0 ? 3 * (1 << ((local & 0x07) + 1)) : 0);
                pos++; // tamanho mínimo do código LZW
                pos = SkipSubBlocks(data, pos);
                frames.Add(new AnimatedFrame(x, y, w, h, delay, dispose, Blend: true, Still: null));
                delay = 100;
                dispose = FrameDispose.None;
            }
            else
            {
                return null;
            }
        }

        return new AnimatedImageInfo(AnimatedFormat.Gif, width, height, frames.Count > 1 ? frames : []);
    }

    private static int SkipSubBlocks(byte[] data, int pos)
    {
        while (data[pos] != 0)
        {
            pos += data[pos] + 1;
        }

        return pos + 1;
    }

    // ---------------------------------------------------------------- WebP

    private static AnimatedImageInfo? ParseWebP(byte[] data)
    {
        int end = Math.Min(data.Length, 8 + checked((int)BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(4))));
        int pos = 12;
        int width = 0;
        int height = 0;
        bool animated = false;
        var frames = new List<AnimatedFrame>();
        while (pos + 8 <= end && frames.Count < MaxFrames)
        {
            string type = Encoding.ASCII.GetString(data, pos, 4);
            int size = checked((int)BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(pos + 4)));
            var body = data.AsSpan(pos + 8, size);
            pos += 8 + size + (size & 1);
            switch (type)
            {
                case "VP8X":
                    animated = (body[0] & 0x02) != 0;
                    width = Read24(body[4..]) + 1;
                    height = Read24(body[7..]) + 1;
                    break;
                case "ANMF":
                    int fx = Read24(body) * 2;
                    int fy = Read24(body[3..]) * 2;
                    int fw = Read24(body[6..]) + 1;
                    int fh = Read24(body[9..]) + 1;
                    int duration = Read24(body[12..]);
                    byte flags = body[15];
                    frames.Add(new AnimatedFrame(fx, fy, fw, fh, NormalizeDelay(duration), (flags & 0x01) != 0 ? FrameDispose.Background : FrameDispose.None, Blend: (flags & 0x02) == 0, BuildWebP(body[16..], fw, fh)));
                    break;
            }
        }

        return animated && frames.Count > 1 ? new AnimatedImageInfo(AnimatedFormat.WebP, width, height, frames) : new AnimatedImageInfo(AnimatedFormat.WebP, width, height, []);
    }

    /// <summary>Os blocos do quadro (ALPH opcional + VP8/VP8L) numa imagem WebP sozinha.</summary>
    private static byte[] BuildWebP(ReadOnlySpan<byte> frameData, int width, int height)
    {
        bool hasAlpha = frameData.Length >= 4 && frameData[..4].SequenceEqual("ALPH"u8);
        using var body = new MemoryStream();
        body.Write("WEBP"u8);
        if (hasAlpha)
        {
            Span<byte> vp8x = stackalloc byte[10];
            vp8x.Clear();
            vp8x[0] = 0x10; // tem alfa
            Write24(vp8x[4..], width - 1);
            Write24(vp8x[7..], height - 1);
            WriteRiffChunk(body, "VP8X", vp8x);
        }

        body.Write(frameData);
        using var output = new MemoryStream();
        output.Write("RIFF"u8);
        Span<byte> size = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(size, (uint)body.Length);
        output.Write(size);
        body.Position = 0;
        body.CopyTo(output);
        return output.ToArray();
    }

    /// <summary>Um bloco RIFF com o tamanho e o preenchimento par (também usado pelos testes).</summary>
    public static void WriteRiffChunk(Stream output, string type, ReadOnlySpan<byte> body)
    {
        ArgumentNullException.ThrowIfNull(output);
        output.Write(Encoding.ASCII.GetBytes(type));
        Span<byte> size = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(size, (uint)body.Length);
        output.Write(size);
        output.Write(body);
        if ((body.Length & 1) != 0)
        {
            output.WriteByte(0);
        }
    }

    public static int Read24(ReadOnlySpan<byte> data) => data[0] | (data[1] << 8) | (data[2] << 16);

    public static void Write24(Span<byte> data, int value)
    {
        data[0] = (byte)value;
        data[1] = (byte)(value >> 8);
        data[2] = (byte)(value >> 16);
    }
}

/// <summary>
/// Monta os quadros de uma animação num quadro inteiro BGRA (premultiplicado não; alfa direto), seguindo mistura e
/// descarte de cada quadro (puro, testado com pixels de exemplo).
/// </summary>
public sealed class FrameComposer(int width, int height)
{
    private readonly byte[] _canvas = new byte[width * height * 4];
    private byte[]? _saved;
    private (int X, int Y, int W, int H, FrameDispose Dispose)? _pending;

    public int Width { get; } = width;

    public int Height { get; } = height;

    /// <summary>
    /// Aplica o descarte do quadro anterior e desenha este (<paramref name="pixels"/> BGRA, <c>w×h</c>); devolve uma cópia
    /// do quadro inteiro pronto para exibir.
    /// </summary>
    public byte[] Draw(AnimatedFrame frame, byte[] pixels)
    {
        ArgumentNullException.ThrowIfNull(frame);
        ArgumentNullException.ThrowIfNull(pixels);
        if (_pending is { } previous)
        {
            if (previous.Dispose == FrameDispose.Background)
            {
                Clear(previous.X, previous.Y, previous.W, previous.H);
            }
            else if (previous.Dispose == FrameDispose.Previous && _saved is not null)
            {
                Buffer.BlockCopy(_saved, 0, _canvas, 0, _canvas.Length);
            }
        }

        _saved = frame.Dispose == FrameDispose.Previous ? (byte[])_canvas.Clone() : null;
        for (int row = 0; row < frame.Height; row++)
        {
            int y = frame.Y + row;
            if (y < 0 || y >= Height)
            {
                continue;
            }

            for (int col = 0; col < frame.Width; col++)
            {
                int x = frame.X + col;
                if (x < 0 || x >= Width)
                {
                    continue;
                }

                int s = ((row * frame.Width) + col) * 4;
                int d = ((y * Width) + x) * 4;
                if (s + 3 >= pixels.Length)
                {
                    continue;
                }

                byte alpha = pixels[s + 3];
                if (!frame.Blend || alpha == 255 || _canvas[d + 3] == 0)
                {
                    if (frame.Blend && alpha == 0)
                    {
                        continue; // transparente sobre o que já está
                    }

                    Buffer.BlockCopy(pixels, s, _canvas, d, 4);
                    continue;
                }

                if (alpha == 0)
                {
                    continue;
                }

                // "Por cima" com alfa direto.
                float a = alpha / 255f;
                float b = _canvas[d + 3] / 255f * (1 - a);
                float outA = a + b;
                for (int c = 0; c < 3; c++)
                {
                    _canvas[d + c] = (byte)Math.Round(((pixels[s + c] * a) + (_canvas[d + c] * b)) / outA);
                }

                _canvas[d + 3] = (byte)Math.Round(outA * 255);
            }
        }

        _pending = (frame.X, frame.Y, frame.Width, frame.Height, frame.Dispose);
        return (byte[])_canvas.Clone();
    }

    private void Clear(int x0, int y0, int w, int h)
    {
        for (int y = Math.Max(0, y0); y < Math.Min(Height, y0 + h); y++)
        {
            Array.Clear(_canvas, ((y * Width) + Math.Max(0, x0)) * 4, (Math.Min(Width, x0 + w) - Math.Max(0, x0)) * 4);
        }
    }
}
