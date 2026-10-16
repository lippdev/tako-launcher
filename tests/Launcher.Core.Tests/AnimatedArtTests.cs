using System.Buffers.Binary;
using Launcher.Core.Presentation;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// ADR-0023: capas animadas. Quando animar (política) e a leitura de quadros e tempos de APNG, GIF e WebP animado com
/// arquivos minúsculos montados no próprio teste (sem imagens reais); composição dos quadros com pixels de exemplo.
/// </summary>
public sealed class AnimatedArtTests
{
    [Theory]
    [InlineData(AnimatedCoversMode.FocusedOnly, true, false, true, false, true)]
    [InlineData(AnimatedCoversMode.FocusedOnly, false, false, true, false, false)]
    [InlineData(AnimatedCoversMode.Always, false, false, true, false, true)]
    [InlineData(AnimatedCoversMode.Off, true, false, true, false, false)]
    [InlineData(AnimatedCoversMode.Always, true, true, true, false, false)] // jogo aberto
    [InlineData(AnimatedCoversMode.Always, true, false, false, false, false)] // janela fora de foco
    [InlineData(AnimatedCoversMode.Always, true, false, true, true, false)] // reduzir animações
    public void Animation_runs_only_when_allowed(AnimatedCoversMode mode, bool focused, bool inGame, bool active, bool reduced, bool expected) =>
        Assert.Equal(expected, AnimationPolicy.ShouldAnimate(mode, focused, inGame, active, reduced));

    [Fact]
    public void Mode_cycles_and_parses_with_focused_only_as_default()
    {
        Assert.Equal(AnimatedCoversMode.FocusedOnly, AnimationPolicy.Parse(null));
        Assert.Equal(AnimatedCoversMode.FocusedOnly, AnimationPolicy.Parse("qualquer"));
        Assert.Equal(AnimatedCoversMode.Always, AnimationPolicy.Parse("Always"));
        Assert.Equal(AnimatedCoversMode.Always, AnimationPolicy.Next(AnimatedCoversMode.FocusedOnly));
        Assert.Equal(AnimatedCoversMode.Off, AnimationPolicy.Next(AnimatedCoversMode.Always));
        Assert.Equal("Só na capa em foco", AnimationPolicy.Label(AnimatedCoversMode.FocusedOnly));
    }

    [Fact]
    public void Apng_frames_become_standalone_pngs_with_delays_and_ops()
    {
        using var file = new MemoryStream();
        file.Write([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        AnimatedImageParser.WriteChunk(file, "IHDR", Ihdr(2, 1));
        AnimatedImageParser.WriteChunk(file, "acTL", [0, 0, 0, 2, 0, 0, 0, 0]);
        AnimatedImageParser.WriteChunk(file, "PLTE", [255, 0, 0]);
        AnimatedImageParser.WriteChunk(file, "fcTL", FrameControl(0, 2, 1, 0, 0, num: 1, den: 4, dispose: 1, blend: 0));
        AnimatedImageParser.WriteChunk(file, "IDAT", [1, 2, 3]);
        AnimatedImageParser.WriteChunk(file, "fcTL", FrameControl(1, 1, 1, 1, 0, num: 0, den: 0, dispose: 2, blend: 1));
        AnimatedImageParser.WriteChunk(file, "fdAT", [0, 0, 0, 2, 4, 5]);
        AnimatedImageParser.WriteChunk(file, "IEND", []);

        var info = AnimatedImageParser.Parse(file.ToArray())!;

        Assert.Equal(AnimatedFormat.Png, info.Format);
        Assert.True(info.IsAnimated);
        Assert.Equal((2, 1), (info.Width, info.Height));
        Assert.Equal([250, 100], info.Frames.Select(f => f.DelayMs)); // 1/4 s; 0 vira 100 ms
        Assert.Equal([FrameDispose.Background, FrameDispose.Previous], info.Frames.Select(f => f.Dispose));
        Assert.Equal([false, true], info.Frames.Select(f => f.Blend));
        Assert.Equal((1, 0, 1, 1), (info.Frames[1].X, info.Frames[1].Y, info.Frames[1].Width, info.Frames[1].Height));

        // O quadro sozinho é um PNG válido (CRCs certos), do tamanho do quadro e com a paleta.
        var still = AnimatedImageParser.Parse(info.Frames[1].Still!)!;
        Assert.False(still.IsAnimated);
        Assert.Equal((1, 1), (still.Width, still.Height));
        Assert.Contains("PLTE"u8.ToArray(), Chunks(info.Frames[1].Still!));
    }

    [Fact]
    public void Static_png_is_not_animated()
    {
        using var file = new MemoryStream();
        file.Write([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        AnimatedImageParser.WriteChunk(file, "IHDR", Ihdr(4, 4));
        AnimatedImageParser.WriteChunk(file, "IDAT", [1]);
        AnimatedImageParser.WriteChunk(file, "IEND", []);

        Assert.False(AnimatedImageParser.Parse(file.ToArray())!.IsAnimated);
        Assert.Null(AnimatedImageParser.Parse([1, 2, 3]));
        Assert.Null(AnimatedImageParser.Parse(file.ToArray()[..20])); // cortado
    }

    [Fact]
    public void Gif_frames_keep_delays_and_disposal()
    {
        byte[] gif =
        [
            .. "GIF89a"u8.ToArray(), 2, 0, 2, 0, 0x00, 0, 0, // tela 2×2, sem paleta global
            0x21, 0xF9, 4, 0x08, 5, 0, 0, 0, // controle: descarte = fundo, 50 ms
            0x2C, 0, 0, 0, 0, 2, 0, 2, 0, 0x00, 2, 1, 0, 0, // quadro 2×2
            0x21, 0xFE, 3, (byte)'o', (byte)'i', (byte)'!', 0, // comentário (ignorado)
            0x21, 0xF9, 4, 0x0C, 0, 0, 0, 0, // descarte = anterior, 0 → 100 ms
            0x2C, 1, 0, 1, 0, 1, 0, 1, 0, 0x00, 2, 1, 0, 0, // quadro 1×1 em (1,1)
            0x3B,
        ];

        var info = AnimatedImageParser.Parse(gif)!;

        Assert.Equal(AnimatedFormat.Gif, info.Format);
        Assert.Equal([50, 100], info.Frames.Select(f => f.DelayMs));
        Assert.Equal([FrameDispose.Background, FrameDispose.Previous], info.Frames.Select(f => f.Dispose));
        Assert.Equal((1, 1), (info.Frames[1].X, info.Frames[1].Y));
        Assert.Null(info.Frames[0].Still);
        Assert.Equal(150, info.TotalMs);
    }

    [Fact]
    public void Animated_webp_frames_become_standalone_webps()
    {
        using var body = new MemoryStream();
        body.Write("WEBP"u8);
        var vp8x = new byte[10];
        vp8x[0] = 0x12; // animação + alfa
        AnimatedImageParser.Write24(vp8x.AsSpan(4), 3 - 1);
        AnimatedImageParser.Write24(vp8x.AsSpan(7), 2 - 1);
        AnimatedImageParser.WriteRiffChunk(body, "VP8X", vp8x);
        AnimatedImageParser.WriteRiffChunk(body, "ANIM", [0, 0, 0, 0, 0, 0]);
        AnimatedImageParser.WriteRiffChunk(body, "ANMF", Anmf(0, 0, 3, 2, 80, flags: 0x02, alpha: false));
        AnimatedImageParser.WriteRiffChunk(body, "ANMF", Anmf(2, 0, 1, 1, 5, flags: 0x01, alpha: true));
        using var file = new MemoryStream();
        file.Write("RIFF"u8);
        file.Write(BitConverter.GetBytes((uint)body.Length));
        file.Write(body.ToArray());

        var info = AnimatedImageParser.Parse(file.ToArray())!;

        Assert.Equal(AnimatedFormat.WebP, info.Format);
        Assert.Equal((3, 2), (info.Width, info.Height));
        Assert.Equal([80, 100], info.Frames.Select(f => f.DelayMs));
        Assert.Equal([false, true], info.Frames.Select(f => f.Blend));
        Assert.Equal([FrameDispose.None, FrameDispose.Background], info.Frames.Select(f => f.Dispose));
        Assert.Equal(2, info.Frames[1].X);
        Assert.Equal(AnimatedFormat.WebP, AnimatedImageParser.Sniff(info.Frames[0].Still!));
        Assert.Equal("VP8L"u8.ToArray(), info.Frames[0].Still![12..16]); // sem alfa: só o VP8L
        Assert.Equal("VP8X"u8.ToArray(), info.Frames[1].Still![12..16]); // com alfa: VP8X + ALPH + VP8
    }

    [Fact]
    public void Composer_blends_and_disposes_like_the_formats_say()
    {
        var composer = new FrameComposer(2, 1);
        byte[] red = [0, 0, 255, 255, 0, 0, 255, 255];
        byte[] halfBlue = [255, 0, 0, 128];

        var first = composer.Draw(new AnimatedFrame(0, 0, 2, 1, 100, FrameDispose.None, Blend: false, null), red);
        Assert.Equal(red, first);

        var second = composer.Draw(new AnimatedFrame(1, 0, 1, 1, 100, FrameDispose.Background, Blend: true, null), halfBlue);
        Assert.Equal([0, 0, 255, 255], second[..4]);
        Assert.Equal(255, second[7]); // por cima do vermelho opaco continua opaco
        Assert.InRange(second[4], 120, 135); // metade azul
        Assert.InRange(second[6], 120, 135); // metade vermelho

        var third = composer.Draw(new AnimatedFrame(0, 0, 1, 1, 100, FrameDispose.Previous, Blend: true, null), [0, 255, 0, 0]);
        Assert.Equal([0, 0, 0, 0], third[4..]); // o quadro 2 pediu fundo: a área dele ficou vazia
        Assert.Equal([0, 0, 255, 255], third[..4]); // pixel transparente com mistura não apaga o de baixo

        var fourth = composer.Draw(new AnimatedFrame(1, 0, 1, 1, 100, FrameDispose.None, Blend: false, null), [0, 255, 0, 255]);
        Assert.Equal([0, 0, 255, 255, 0, 255, 0, 255], fourth); // "anterior" voltou ao estado de antes do quadro 3
    }

    private static byte[] Ihdr(int width, int height)
    {
        var data = new byte[13];
        BinaryPrimitives.WriteUInt32BigEndian(data, (uint)width);
        BinaryPrimitives.WriteUInt32BigEndian(data.AsSpan(4), (uint)height);
        data[8] = 8;
        data[9] = 3; // paleta
        return data;
    }

    private static byte[] FrameControl(int sequence, int width, int height, int x, int y, int num, int den, byte dispose, byte blend)
    {
        var data = new byte[26];
        BinaryPrimitives.WriteUInt32BigEndian(data, (uint)sequence);
        BinaryPrimitives.WriteUInt32BigEndian(data.AsSpan(4), (uint)width);
        BinaryPrimitives.WriteUInt32BigEndian(data.AsSpan(8), (uint)height);
        BinaryPrimitives.WriteUInt32BigEndian(data.AsSpan(12), (uint)x);
        BinaryPrimitives.WriteUInt32BigEndian(data.AsSpan(16), (uint)y);
        BinaryPrimitives.WriteUInt16BigEndian(data.AsSpan(20), (ushort)num);
        BinaryPrimitives.WriteUInt16BigEndian(data.AsSpan(22), (ushort)den);
        data[24] = dispose;
        data[25] = blend;
        return data;
    }

    private static byte[] Anmf(int x, int y, int width, int height, int duration, byte flags, bool alpha)
    {
        using var output = new MemoryStream();
        var header = new byte[16];
        AnimatedImageParser.Write24(header, x / 2);
        AnimatedImageParser.Write24(header.AsSpan(3), y / 2);
        AnimatedImageParser.Write24(header.AsSpan(6), width - 1);
        AnimatedImageParser.Write24(header.AsSpan(9), height - 1);
        AnimatedImageParser.Write24(header.AsSpan(12), duration);
        header[15] = flags;
        output.Write(header);
        if (alpha)
        {
            AnimatedImageParser.WriteRiffChunk(output, "ALPH", [0, 1]);
            AnimatedImageParser.WriteRiffChunk(output, "VP8 ", [1, 2, 3]);
        }
        else
        {
            AnimatedImageParser.WriteRiffChunk(output, "VP8L", [0x2F, 1, 2, 3, 4]);
        }

        return output.ToArray();
    }

    private static List<byte[]> Chunks(byte[] png)
    {
        var list = new List<byte[]>();
        for (int pos = 8; pos + 12 <= png.Length;)
        {
            int length = (int)BinaryPrimitives.ReadUInt32BigEndian(png.AsSpan(pos));
            list.Add(png[(pos + 4)..(pos + 8)]);
            pos += 12 + length;
        }

        return list;
    }
}
