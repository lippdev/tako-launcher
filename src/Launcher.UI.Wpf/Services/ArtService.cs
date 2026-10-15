using System.IO;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.Core.Presentation;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Baixa capas e artes para um cache local (%LOCALAPPDATA%\Tako\cache\art) e entrega imagens prontas. Aceita
/// endereços http(s), arquivos locais (capas escolhidas pelo usuário), <c>steam-art://</c> (vários candidatos,
/// do melhor ao pior) e <c>ssmedia://</c> (mídia do ScreenScraper, baixada com as credenciais só na hora, ADR-0022). Sem arte ou sem rede, a UI usa o fallback (gradiente + título).
/// </summary>
public sealed partial class ArtService : IDisposable
{
    private const string SteamArtScheme = "steam-art";
    private const int LargeImageWidth = 1000;

    /// <summary>
    /// Capas decodificadas guardadas (PERF-004): ~90 capas em 1080p, ~23 em 4K. As que estão na tela ficam vivas pela
    /// própria imagem; o cache só evita decodificar de novo ao rolar de volta um pouco.
    /// </summary>
    private const long SmallCacheBytes = 32L * 1024 * 1024;

    /// <summary>Artes de fundo: ~4 em 1080p, 1 em 4K (cada uma ocupa 8 a 33 MB decodificada).</summary>
    private const long LargeCacheBytes = 32L * 1024 * 1024;

    /// <summary>Custo nominal de "sem arte" no cache (não ocupa imagem, mas não pode ficar para sempre).</summary>
    private const long MissingCost = 1024;

    private readonly object _gate = new();
    private readonly Dictionary<(Uri, int), Task<ImageSource?>> _pending = [];
    private readonly SizedLruCache<(Uri, int), ImageSource?> _small = new(SmallCacheBytes);
    private readonly SizedLruCache<(Uri, int), ImageSource?> _large = new(LargeCacheBytes);

    /// <summary>Decodificações em paralelo: rolar rápido não ocupa todos os núcleos (nem atrasa a thread da UI).</summary>
    private readonly SemaphoreSlim _decoders = new(Math.Clamp(Environment.ProcessorCount / 2, 1, 4));
    private readonly ILogger<ArtService> _logger;
    private readonly HttpClient _http;
    private readonly SteamArtResolver _steam;
    private readonly ScreenScraperClient _screenScraper;
    private readonly string _cacheDirectory;

    public ArtService(ILogger<ArtService> logger, HttpClient http, SteamArtResolver steam, ScreenScraperClient screenScraper)
    {
        _logger = logger;
        _http = http;
        _steam = steam;
        _screenScraper = screenScraper;
        _cacheDirectory = AppPaths.ArtCache;
    }

    /// <summary>
    /// Pixels físicos por unidade do palco de 1080 de altura (1 em 1080p, 2 em 4K). Começa pela altura da tela principal
    /// e a janela atualiza conforme tamanho e DPI reais; as imagens são decodificadas nessa densidade.
    /// </summary>
    public double RenderScale { get; set; } = InitialScale();

    /// <summary>
    /// Imagem para exibir com <paramref name="displayWidth"/> unidades do palco, decodificada na densidade da tela
    /// (nunca acima da resolução original) fora da thread da UI, ou nulo se não houver arte ou rede. Pedidos iguais em
    /// andamento são compartilhados; as prontas ficam num cache LRU limitado em bytes.
    /// </summary>
    public Task<ImageSource?> GetAsync(Uri? uri, int displayWidth)
    {
        if (uri is null)
        {
            return Task.FromResult<ImageSource?>(null);
        }

        var key = (uri, DecodeWidth(displayWidth));
        lock (_gate)
        {
            if (CacheFor(key).TryGet(key, out var cached))
            {
                return Task.FromResult(cached);
            }

            if (!_pending.TryGetValue(key, out var task))
            {
                task = LoadAndCacheAsync(key);
                if (!task.IsCompleted)
                {
                    _pending[key] = task;
                }
            }

            return task;
        }
    }

    /// <summary>A imagem já decodificada, sem esperar (nulo também quando já se sabe que não há arte).</summary>
    public bool TryGetCached(Uri? uri, int displayWidth, out ImageSource? image)
    {
        image = null;
        if (uri is null)
        {
            return true;
        }

        var key = (uri, DecodeWidth(displayWidth));
        lock (_gate)
        {
            return CacheFor(key).TryGet(key, out image);
        }
    }

    public void Dispose() => _decoders.Dispose();

    private int DecodeWidth(int displayWidth) => (int)Math.Ceiling(displayWidth * Math.Max(1, RenderScale));

    private SizedLruCache<(Uri, int), ImageSource?> CacheFor((Uri Uri, int Width) key) => key.Width >= LargeImageWidth ? _large : _small;

    private async Task<ImageSource?> LoadAndCacheAsync((Uri Uri, int Width) key)
    {
        var image = await LoadAsync(key.Uri, key.Width).ConfigureAwait(false);
        lock (_gate)
        {
            _pending.Remove(key);
            long cost = image is BitmapSource bitmap ? (long)bitmap.PixelWidth * bitmap.PixelHeight * 4 : MissingCost;
            CacheFor(key).Add(key, image, cost);
        }

        return image;
    }

    private async Task<ImageSource?> LoadAsync(Uri uri, int decodeWidth)
    {
        try
        {
            if (await LocateAsync(uri).ConfigureAwait(false) is not string path)
            {
                return null;
            }

            await _decoders.WaitAsync().ConfigureAwait(false);
            try
            {
                // Arquivo local resolve na hora: sem o Task.Run a decodificação rodaria na thread de quem pediu (a UI).
                return await Task.Run<ImageSource?>(() => Decode(path, decodeWidth)).ConfigureAwait(false);
            }
            finally
            {
                _decoders.Release();
            }
        }
        catch (Exception ex) when (ex is HttpRequestException or IOException or TaskCanceledException or NotSupportedException or UnauthorizedAccessException or FileFormatException)
        {
            LogArtFailed(_logger, uri, ex.Message);
            return null;
        }
    }

    /// <summary>Decodifica já no tamanho de exibição e congela (pode ir para a thread da UI).</summary>
    private static BitmapImage Decode(string path, int decodeWidth)
    {
        int sourceWidth = BitmapDecoder.Create(new Uri(path), BitmapCreateOptions.DelayCreation, BitmapCacheOption.None).Frames[0].PixelWidth;
        var image = new BitmapImage();
        image.BeginInit();
        image.CacheOption = BitmapCacheOption.OnLoad;
        image.UriSource = new Uri(path);
        image.DecodePixelWidth = Math.Min(decodeWidth, sourceWidth);
        image.EndInit();
        image.Freeze();
        return image;
    }

    /// <summary>Arquivo local da imagem: o próprio arquivo, o cache, ou o primeiro candidato que baixar.</summary>
    private async Task<string?> LocateAsync(Uri uri)
    {
        if (uri.IsFile)
        {
            return File.Exists(uri.LocalPath) ? uri.LocalPath : null;
        }

        if (uri.Scheme == ScreenScraperMedia.Scheme)
        {
            return await EnsureScreenScraperCachedAsync(uri).ConfigureAwait(false);
        }

        if (uri.Scheme != SteamArtScheme)
        {
            return await EnsureCachedAsync(uri, uri).ConfigureAwait(false);
        }

        // Artes da Steam: o cache vale para o endereço lógico, então a próxima abertura não consulta a rede.
        string cached = CachePath(uri);
        if (File.Exists(cached))
        {
            return cached;
        }

        foreach (var candidate in await _steam.ResolveAsync(uri).ConfigureAwait(false))
        {
            if (candidate.IsFile)
            {
                return candidate.LocalPath; // cache da própria Steam
            }

            try
            {
                return await EnsureCachedAsync(uri, candidate).ConfigureAwait(false);
            }
            catch (HttpRequestException)
            {
                // 404: próximo candidato.
            }
        }

        return null;
    }

    /// <summary>Mídia do ScreenScraper: o cache vale para o endereço sem credenciais; baixa pela fila da conta do usuário.</summary>
    private async Task<string?> EnsureScreenScraperCachedAsync(Uri uri)
    {
        string path = CachePath(uri);
        if (File.Exists(path))
        {
            return path;
        }

        if (await _screenScraper.DownloadMediaAsync(uri, CancellationToken.None).ConfigureAwait(false) is not { } bytes)
        {
            return null;
        }

        Directory.CreateDirectory(_cacheDirectory);
        string temp = path + ".tmp";
        await File.WriteAllBytesAsync(temp, bytes).ConfigureAwait(false);
        File.Move(temp, path, overwrite: true);
        return path;
    }

    private string CachePath(Uri key)
    {
        string name = Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(key.AbsoluteUri)))[..32];
        string extension = key.Scheme == SteamArtScheme || key.Scheme == ScreenScraperMedia.Scheme ? ".img" : Path.GetExtension(key.AbsolutePath);
        return Path.Combine(_cacheDirectory, name + extension);
    }

    private async Task<string> EnsureCachedAsync(Uri key, Uri source)
    {
        string path = CachePath(key);
        if (File.Exists(path))
        {
            return path;
        }

        Directory.CreateDirectory(_cacheDirectory);
        byte[] bytes = await _http.GetByteArrayAsync(source).ConfigureAwait(false);
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
