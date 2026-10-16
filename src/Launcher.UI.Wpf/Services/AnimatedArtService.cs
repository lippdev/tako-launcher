using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Launcher.Core.Presentation;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>Quadros prontos de uma capa animada, no tamanho de exibição, e quanto tempo cada um fica (ms).</summary>
public sealed record AnimatedFrames(IReadOnlyList<BitmapSource> Frames, IReadOnlyList<int> Delays);

/// <summary>
/// Capas animadas (ADR-0023): o WPF mostra só o primeiro quadro de WebP, APNG e GIF. Este serviço lê a estrutura do
/// arquivo (<see cref="AnimatedImageParser"/>), decodifica cada quadro pelo WIC do Windows (WebP pela extensão de imagem
/// WebP do Windows; sem ela, a capa fica estática), compõe (<see cref="FrameComposer"/>) e reduz ao tamanho de exibição,
/// tudo fora da thread da UI. Cada animação cabe em um orçamento de memória: com quadros demais, alguns são pulados
/// (somando o tempo). Só as animações recentes ficam guardadas.
/// </summary>
public sealed partial class AnimatedArtService(ArtService art, ILogger<AnimatedArtService> logger)
{
    /// <summary>Memória máxima de uma animação já decodificada.</summary>
    public const long FrameBudgetBytes = 16L * 1024 * 1024;

    private const long CacheBytes = 48L * 1024 * 1024;
    private readonly object _gate = new();
    private readonly SizedLruCache<(Uri, int), AnimatedFrames?> _cache = new(CacheBytes);
    private readonly Dictionary<(Uri, int), Task<AnimatedFrames?>> _pending = [];

    /// <summary>Só arquivos que podem ser animados são lidos (JPEG nunca é).</summary>
    public static bool MightAnimate(Uri? uri) =>
        uri is not null && (uri.AbsolutePath.EndsWith(".webp", StringComparison.OrdinalIgnoreCase)
            || uri.AbsolutePath.EndsWith(".png", StringComparison.OrdinalIgnoreCase)
            || uri.AbsolutePath.EndsWith(".apng", StringComparison.OrdinalIgnoreCase)
            || uri.AbsolutePath.EndsWith(".gif", StringComparison.OrdinalIgnoreCase));

    /// <summary>Quadros da animação em <paramref name="displayWidth"/> unidades do palco; nulo se for estática ou falhar.</summary>
    public Task<AnimatedFrames?> LoadAsync(Uri? uri, int displayWidth)
    {
        if (!MightAnimate(uri))
        {
            return Task.FromResult<AnimatedFrames?>(null);
        }

        int width = (int)Math.Ceiling(displayWidth * Math.Max(1, art.RenderScale));
        var key = (uri!, width);
        lock (_gate)
        {
            if (_cache.TryGet(key, out var cached))
            {
                return Task.FromResult(cached);
            }

            if (!_pending.TryGetValue(key, out var task))
            {
                task = LoadAndCacheAsync(uri!, width);
                _pending[key] = task;
            }

            return task;
        }
    }

    private async Task<AnimatedFrames?> LoadAndCacheAsync(Uri uri, int width)
    {
        AnimatedFrames? frames = null;
        try
        {
            if (await art.GetFileAsync(uri).ConfigureAwait(false) is { } path)
            {
                frames = await Task.Run(() => Decode(File.ReadAllBytes(path), width)).ConfigureAwait(false);
            }
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or NotSupportedException or FileFormatException or ArgumentException or InvalidOperationException or System.Runtime.InteropServices.COMException)
        {
            LogAnimationFailed(logger, uri, ex.Message); // fica a capa estática
        }

        lock (_gate)
        {
            _pending.Remove((uri, width));
            long cost = frames?.Frames.Sum(f => (long)f.PixelWidth * f.PixelHeight * 4) ?? 1024;
            _cache.Add((uri, width), frames, cost);
        }

        return frames;
    }

    /// <summary>Lê, compõe e reduz os quadros (sem tocar na UI). Público para o teste de fumaça e o benchmark.</summary>
    public static AnimatedFrames? Decode(byte[] data, int decodeWidth)
    {
        if (AnimatedImageParser.Parse(data) is not { IsAnimated: true } info || info.Width <= 0 || info.Height <= 0)
        {
            return null;
        }

        double scale = Math.Min(1.0, decodeWidth / (double)info.Width);
        int outWidth = Math.Max(1, (int)Math.Round(info.Width * scale));
        int outHeight = Math.Max(1, (int)Math.Round(info.Height * scale));
        long perFrame = (long)outWidth * outHeight * 4;
        int keep = (int)Math.Clamp(FrameBudgetBytes / perFrame, 2, info.Frames.Count);
        double step = info.Frames.Count / (double)keep;

        GifBitmapDecoder? gif = info.Format == AnimatedFormat.Gif
            ? new GifBitmapDecoder(new MemoryStream(data), BitmapCreateOptions.PreservePixelFormat, BitmapCacheOption.OnLoad)
            : null;
        var composer = new FrameComposer(info.Width, info.Height);
        var frames = new List<BitmapSource>(keep);
        var delays = new List<int>(keep);
        double next = 0;
        int pendingDelay = 0;
        for (int i = 0; i < info.Frames.Count; i++)
        {
            var frame = info.Frames[i];
            BitmapSource source = gif is not null
                ? gif.Frames[Math.Min(i, gif.Frames.Count - 1)]
                : BitmapDecoder.Create(new MemoryStream(frame.Still!), BitmapCreateOptions.PreservePixelFormat, BitmapCacheOption.OnLoad).Frames[0];
            var bgra = new FormatConvertedBitmap(source, PixelFormats.Bgra32, null, 0);
            var pixels = new byte[bgra.PixelWidth * bgra.PixelHeight * 4];
            bgra.CopyPixels(pixels, bgra.PixelWidth * 4, 0);
            var canvas = composer.Draw(frame with { Width = bgra.PixelWidth, Height = bgra.PixelHeight }, pixels);
            pendingDelay += frame.DelayMs;
            if (i + 1 < next && i != info.Frames.Count - 1)
            {
                continue; // quadro pulado: o tempo dele vai para o próximo guardado
            }

            next += step;
            frames.Add(Scale(canvas, info.Width, info.Height, outWidth, outHeight));
            delays.Add(pendingDelay);
            pendingDelay = 0;
            if (frames.Count == keep)
            {
                break;
            }
        }

        return frames.Count > 1 ? new AnimatedFrames(frames, delays) : null;
    }

    private static BitmapSource Scale(byte[] canvas, int width, int height, int outWidth, int outHeight)
    {
        BitmapSource full = BitmapSource.Create(width, height, 96, 96, PixelFormats.Bgra32, null, canvas, width * 4);
        if (outWidth != width || outHeight != height)
        {
            var scaled = new TransformedBitmap(full, new ScaleTransform(outWidth / (double)width, outHeight / (double)height));
            var pixels = new byte[scaled.PixelWidth * scaled.PixelHeight * 4];
            scaled.CopyPixels(pixels, scaled.PixelWidth * 4, 0);
            full = BitmapSource.Create(scaled.PixelWidth, scaled.PixelHeight, 96, 96, PixelFormats.Bgra32, null, pixels, scaled.PixelWidth * 4);
        }

        full.Freeze();
        return full;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Capa animada mostrada parada ({Uri}): {Reason}")]
    private static partial void LogAnimationFailed(ILogger logger, Uri uri, string reason);
}
