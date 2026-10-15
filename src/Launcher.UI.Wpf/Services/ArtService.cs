using System.IO;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Baixa capas e artes para um cache local (%LOCALAPPDATA%\Tako\cache\art) e entrega imagens prontas.
/// Arte de jogo nunca vai para o repositório; sem rede, a UI usa o fallback (gradiente + título).
/// </summary>
public sealed partial class ArtService : IDisposable
{
    private readonly HttpClient _http = new() { Timeout = TimeSpan.FromSeconds(20) };
    private const int LargeImageWidth = 1000;
    private const int MaxLargeImages = 8;
    private readonly Dictionary<(Uri, int), Task<ImageSource?>> _memory = [];
    private readonly Queue<(Uri, int)> _large = new();
    private readonly ILogger<ArtService> _logger;
    private readonly string _cacheDirectory;

    public ArtService(ILogger<ArtService> logger)
    {
        _logger = logger;
        _cacheDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Tako", "cache", "art");
        // Wikipedia e outros CDNs exigem User-Agent identificável.
        _http.DefaultRequestHeaders.UserAgent.ParseAdd("Tako/0.1 (+https://github.com/lippdev/ultralauncher)");
    }

    /// <summary>
    /// Pixels físicos por unidade do palco de 1080 de altura (1 em 1080p, 2 em 4K). Começa pela altura da tela principal
    /// e a janela atualiza conforme tamanho e DPI reais; as imagens são decodificadas nessa densidade.
    /// </summary>
    public double RenderScale { get; set; } = InitialScale();

    /// <summary>
    /// Imagem para exibir com <paramref name="displayWidth"/> unidades do palco, decodificada na densidade da tela
    /// (nunca acima da resolução original), ou nulo se não houver arte ou rede.
    /// </summary>
    public Task<ImageSource?> GetAsync(Uri? uri, int displayWidth)
    {
        if (uri is null)
        {
            return Task.FromResult<ImageSource?>(null);
        }

        int decodeWidth = (int)Math.Ceiling(displayWidth * Math.Max(1, RenderScale));

        lock (_memory)
        {
            if (!_memory.TryGetValue((uri, decodeWidth), out var task))
            {
                task = LoadAsync(uri, decodeWidth);
                _memory[(uri, decodeWidth)] = task;
                if (decodeWidth >= LargeImageWidth)
                {
                    // Artes de fundo em 4K ocupam ~19 MB cada: guarda só as mais recentes.
                    _large.Enqueue((uri, decodeWidth));
                    if (_large.Count > MaxLargeImages)
                    {
                        _memory.Remove(_large.Dequeue());
                    }
                }
            }

            return task;
        }
    }

    public void Dispose() => _http.Dispose();

    private async Task<ImageSource?> LoadAsync(Uri uri, int decodeWidth)
    {
        try
        {
            string path = await EnsureCachedAsync(uri).ConfigureAwait(false);
            return await Task.Run(() =>
            {
                int sourceWidth = BitmapDecoder.Create(new Uri(path), BitmapCreateOptions.DelayCreation, BitmapCacheOption.None).Frames[0].PixelWidth;
                var image = new BitmapImage();
                image.BeginInit();
                image.CacheOption = BitmapCacheOption.OnLoad;
                image.UriSource = new Uri(path);
                image.DecodePixelWidth = Math.Min(decodeWidth, sourceWidth);
                image.EndInit();
                image.Freeze();
                return (ImageSource)image;
            }).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is HttpRequestException or IOException or TaskCanceledException or NotSupportedException or UnauthorizedAccessException)
        {
            LogArtFailed(_logger, uri, ex.Message);
            return null;
        }
    }

    private async Task<string> EnsureCachedAsync(Uri uri)
    {
        string name = Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(uri.AbsoluteUri)))[..32];
        string path = Path.Combine(_cacheDirectory, name + Path.GetExtension(uri.AbsolutePath));
        if (File.Exists(path))
        {
            return path;
        }

        Directory.CreateDirectory(_cacheDirectory);
        byte[] bytes = await _http.GetByteArrayAsync(uri).ConfigureAwait(false);
        string temp = path + ".tmp";
        await File.WriteAllBytesAsync(temp, bytes).ConfigureAwait(false);
        File.Move(temp, path, overwrite: true);
        return path;
    }

    private static double InitialScale()
    {
        const int ScreenHeight = 1; // SM_CYSCREEN: pixels físicos da tela principal (processo PerMonitorV2).
        int height = GetSystemMetrics(ScreenHeight);
        return height > 0 ? Math.Max(1, height / 1080.0) : 1;
    }

    [LibraryImport("user32.dll")]
    private static partial int GetSystemMetrics(int index);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Arte indisponível ({Uri}): {Reason}")]
    private static partial void LogArtFailed(ILogger logger, Uri uri, string reason);
}
