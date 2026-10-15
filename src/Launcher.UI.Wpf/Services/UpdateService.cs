using System.Diagnostics;
using System.IO;
using System.Net.Http;
using System.Reflection;
using System.Text.Json;
using Launcher.Core.Updates;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Atualização automática pelas releases do GitHub (mesmo modelo do Console Mode): procura versão nova, baixa o arquivo
/// do tipo de instalação (instalador ou .exe portátil), confere o SHA-256 publicado pelo GitHub e aplica — o instalado roda
/// o setup em modo silencioso; o portátil troca o .exe por um script depois que o Tako fecha.
/// </summary>
public sealed partial class UpdateService(ILogger<UpdateService> logger) : IDisposable
{
    private static readonly TimeSpan CheckTimeout = TimeSpan.FromSeconds(15);
    private static readonly TimeSpan StallTimeout = TimeSpan.FromSeconds(30);

    // Sem tempo total: o limite é por falta de dados (StallTimeout), para não cortar downloads lentos que ainda progridem.
    private readonly HttpClient _http = CreateClient();

    public static string CurrentVersion { get; } =
        Assembly.GetEntryAssembly()?.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion.Split('+')[0] ?? "0.0.0";

    /// <summary>
    /// Só versões publicadas se atualizam: instalada (marcador do instalador) ou portátil (.exe único). Rodando pelo
    /// <c>dotnet run</c>/Visual Studio, o executável não pode ser trocado.
    /// </summary>
    public static bool IsPublishedBuild => AppPaths.IsInstalled || string.IsNullOrEmpty(typeof(UpdateService).Assembly.Location);

    public static string InstallKind => AppPaths.IsInstalled ? "instalada" : "portátil";

    public void Dispose() => _http.Dispose();

    public async Task<UpdateInfo?> CheckAsync(bool optedIntoPrereleases, CancellationToken cancellationToken = default)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(CheckTimeout);
        using var request = new HttpRequestMessage(HttpMethod.Get, $"https://api.github.com/repos/{ReleaseFeed.Repository}/releases?per_page=15");
        request.Headers.Accept.ParseAdd("application/vnd.github+json");
        using var response = await _http.SendAsync(request, timeout.Token).ConfigureAwait(false);
        response.EnsureSuccessStatusCode();
        await using var stream = await response.Content.ReadAsStreamAsync(timeout.Token).ConfigureAwait(false);
        using var document = await JsonDocument.ParseAsync(stream, cancellationToken: timeout.Token).ConfigureAwait(false);
        return ReleaseFeed.Pick(document.RootElement, SemVer.Parse(CurrentVersion), optedIntoPrereleases, AppPaths.IsInstalled);
    }

    /// <summary>Primeiro item da versão no <c>CHANGELOG.md</c> (português) da tag; sem ele, nulo (o aviso usa as notas em inglês).</summary>
    public async Task<string?> PortugueseHighlightAsync(UpdateInfo update, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(update);
        try
        {
            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            timeout.CancelAfter(CheckTimeout);
            string changelog = await _http.GetStringAsync(new Uri($"https://raw.githubusercontent.com/{ReleaseFeed.Repository}/v{update.Version}/CHANGELOG.md"), timeout.Token).ConfigureAwait(false);
            return ReleaseFeed.ChangelogSection(changelog, update.Version) is { } section ? ReleaseFeed.FirstNote(section) : null;
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException)
        {
            return null;
        }
    }

    /// <summary>Baixa para a pasta temporária e confere o SHA-256; arquivo que não confere é apagado.</summary>
    public async Task<string> DownloadAsync(UpdateInfo update, IProgress<double>? progress, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(update);
        if (!update.CanInstall)
        {
            throw new InvalidOperationException("A release não tem um arquivo verificável para este tipo de instalação.");
        }

        string directory = Path.Combine(Path.GetTempPath(), "TakoUpdate");
        Directory.CreateDirectory(directory);
        string file = Path.Combine(directory, update.AssetName!);
        if (File.Exists(file) && await ReleaseFeed.VerifyFileAsync(file, update.AssetDigest, cancellationToken).ConfigureAwait(false))
        {
            return file; // baixado numa abertura anterior
        }

        await DownloadFileAsync(new Uri(update.AssetUrl!), file, progress, cancellationToken).ConfigureAwait(false);
        if (!await ReleaseFeed.VerifyFileAsync(file, update.AssetDigest, cancellationToken).ConfigureAwait(false))
        {
            File.Delete(file);
            throw new InvalidDataException("O arquivo baixado não confere com o SHA-256 publicado no GitHub; nada foi instalado.");
        }

        LogDownloaded(logger, update.Version, file);
        return file;
    }

    /// <summary>
    /// Entrega para a atualização. Quem chama encerra o Tako em seguida (o setup / script espera o processo terminar).
    /// <paramref name="relaunch"/> falso = atualiza ao sair, sem reabrir.
    /// </summary>
    public void Apply(string file, bool relaunch)
    {
        LogApplying(logger, file, relaunch);
        if (AppPaths.IsInstalled)
        {
            // Inno Setup: fecha o Tako pelo Restart Manager e o reabre ao terminar (menos com /NOLAUNCH=1).
            Process.Start(new ProcessStartInfo
            {
                FileName = file,
                Arguments = "/SILENT /SUPPRESSMSGBOXES /NORESTART /CLOSEAPPLICATIONS" + (relaunch ? string.Empty : " /NOLAUNCH=1"),
                UseShellExecute = true,
            })?.Dispose();
            return;
        }

        // Portátil: um .exe em uso não pode ser sobrescrito; um script espera o Tako fechar, troca o arquivo e (se pedido) o reabre.
        string exe = Environment.ProcessPath ?? throw new InvalidOperationException("Caminho do executável desconhecido.");
        string script = Path.Combine(Path.GetDirectoryName(file)!, "swap.cmd");
        int pid = Environment.ProcessId;
        File.WriteAllText(
            script,
            "@echo off\r\n"
            // ping como espera: timeout.exe falha sem console interativo.
            + $":wait\r\ntasklist /fi \"PID eq {pid}\" | find \"{pid}\" >nul && (ping -n 2 127.0.0.1 >nul & goto wait)\r\n"
            + $"move /y \"{file}\" \"{exe}\" >nul\r\n"
            + (relaunch ? $"start \"\" \"{exe}\"\r\n" : string.Empty));
        Process.Start(new ProcessStartInfo
        {
            FileName = "cmd.exe",
            Arguments = $"/c \"{script}\"",
            CreateNoWindow = true,
            UseShellExecute = false,
            WorkingDirectory = Path.GetDirectoryName(file)!,
        })?.Dispose();
    }

    private async Task DownloadFileAsync(Uri url, string file, IProgress<double>? progress, CancellationToken cancellationToken)
    {
        using var stall = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        stall.CancelAfter(StallTimeout);
        try
        {
            using var response = await _http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, stall.Token).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            long total = response.Content.Headers.ContentLength ?? -1;
            await using var source = await response.Content.ReadAsStreamAsync(stall.Token).ConfigureAwait(false);
            await using var target = File.Create(file);
            byte[] buffer = new byte[81920];
            long done = 0;
            int read;
            while ((read = await source.ReadAsync(buffer, stall.Token).ConfigureAwait(false)) > 0)
            {
                stall.CancelAfter(StallTimeout);
                await target.WriteAsync(buffer.AsMemory(0, read), cancellationToken).ConfigureAwait(false);
                done += read;
                if (total > 0)
                {
                    progress?.Report((double)done / total);
                }
            }
        }
        catch (OperationCanceledException) when (stall.IsCancellationRequested && !cancellationToken.IsCancellationRequested)
        {
            throw new TimeoutException($"Download parado: nenhum dado por {StallTimeout.TotalSeconds:0} s.");
        }
    }

    private static HttpClient CreateClient()
    {
        var client = new HttpClient { Timeout = Timeout.InfiniteTimeSpan };
        // O GitHub recusa chamadas de API sem User-Agent.
        client.DefaultRequestHeaders.UserAgent.ParseAdd($"Tako/{CurrentVersion} (+https://github.com/{ReleaseFeed.Repository})");
        return client;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Aplicando atualização {File} (reabrir: {Relaunch})")]
    private static partial void LogApplying(ILogger logger, string file, bool relaunch);

    [LoggerMessage(Level = LogLevel.Information, Message = "Atualização {Version}: baixada e verificada em {File}")]
    private static partial void LogDownloaded(ILogger logger, string version, string file);
}

/// <summary>Preferências de atualização, salvas em <c>updates.json</c> na pasta de dados.</summary>
public sealed class UpdatePreferences
{
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    /// <summary>Procurar e baixar ao abrir (padrão: ligado).</summary>
    public bool AutoUpdate { get; set; } = true;

    /// <summary>Receber versões de teste (alpha/beta) numa versão estável.</summary>
    public bool Prereleases { get; set; }

    private static string FilePath => Path.Combine(AppPaths.DataRoot, "updates.json");

    public static UpdatePreferences Load()
    {
        try
        {
            return File.Exists(FilePath) ? JsonSerializer.Deserialize<UpdatePreferences>(File.ReadAllText(FilePath), Options) ?? new() : new();
        }
        catch (Exception ex) when (ex is JsonException or IOException)
        {
            return new();
        }
    }

    public void Save()
    {
        Directory.CreateDirectory(AppPaths.DataRoot);
        File.WriteAllText(FilePath, JsonSerializer.Serialize(this, Options));
    }
}
