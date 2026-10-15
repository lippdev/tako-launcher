using System.IO;
using System.Net.Http;
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
    private readonly Dictionary<(Uri, int), Task<ImageSource?>> _memory = [];
    private readonly ILogger<ArtService> _logger;
    private readonly string _cacheDirectory;

    public ArtService(ILogger<ArtService> logger)
    {
        _logger = logger;
        _cacheDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Tako", "cache", "art");
        // Wikipedia e outros CDNs exigem User-Agent identificável.
        _http.DefaultRequestHeaders.UserAgent.ParseAdd("Tako/0.1 (+https://github.com/lippdev/ultralauncher)");
    }

    /// <summary>Imagem decodificada na largura pedida, ou nulo se não houver arte ou rede.</summary>
    public Task<ImageSource?> GetAsync(Uri? uri, int decodeWidth)
    {
        if (uri is null)
        {
            return Task.FromResult<ImageSource?>(null);
        }

        lock (_memory)
        {
            if (!_memory.TryGetValue((uri, decodeWidth), out var task))
            {
                task = LoadAsync(uri, decodeWidth);
                _memory[(uri, decodeWidth)] = task;
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
                var image = new BitmapImage();
                image.BeginInit();
                image.CacheOption = BitmapCacheOption.OnLoad;
                image.UriSource = new Uri(path);
                image.DecodePixelWidth = decodeWidth;
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

    [LoggerMessage(Level = LogLevel.Warning, Message = "Arte indisponível ({Uri}): {Reason}")]
    private static partial void LogArtFailed(ILogger logger, Uri uri, string reason);
}
