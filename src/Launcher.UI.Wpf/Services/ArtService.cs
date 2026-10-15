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
/// Baixa capas e artes para um cache local (%LOCALAPPDATA%\Tako\cache\art) e entrega imagens prontas. Aceita
/// endereços http(s), arquivos locais (capas escolhidas pelo usuário) e <c>steam-art://</c> (vários candidatos,
/// do melhor ao pior). Sem arte ou sem rede, a UI usa o fallback (gradiente + título).
/// </summary>
public sealed partial class ArtService
{
    private const string SteamArtScheme = "steam-art";
    private const int LargeImageWidth = 1000;
    private const int MaxLargeImages = 8;
    private readonly Dictionary<(Uri, int), Task<ImageSource?>> _memory = [];
    private readonly Queue<(Uri, int)> _large = new();
    private readonly ILogger<ArtService> _logger;
    private readonly HttpClient _http;
    private readonly SteamArtResolver _steam;
    private readonly string _cacheDirectory;

    public ArtService(ILogger<ArtService> logger, HttpClient http, SteamArtResolver steam)
    {
        _logger = logger;
        _http = http;
        _steam = steam;
        _cacheDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Tako", "cache", "art");
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

    private async Task<ImageSource?> LoadAsync(Uri uri, int decodeWidth)
    {
        try
        {
            if (await LocateAsync(uri).ConfigureAwait(false) is not string path)
            {
                return null;
            }

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

    /// <summary>Arquivo local da imagem: o próprio arquivo, o cache, ou o primeiro candidato que baixar.</summary>
    private async Task<string?> LocateAsync(Uri uri)
    {
        if (uri.IsFile)
        {
            return File.Exists(uri.LocalPath) ? uri.LocalPath : null;
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

    private string CachePath(Uri key)
    {
        string name = Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(key.AbsoluteUri)))[..32];
        string extension = key.Scheme == SteamArtScheme ? ".img" : Path.GetExtension(key.AbsolutePath);
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
