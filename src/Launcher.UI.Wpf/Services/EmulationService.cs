using System.IO;
using Launcher.Core.Emulation;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.Core.Library.Sources;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Emulação (épico EMU, ADR-0024): a configuração (<c>emulation.json</c>), os emuladores achados neste PC, a fonte
/// "Emulação" da biblioteca e a linha de comando de cada ROM. A detecção só lê disco e registro e é refeita sob
/// pedido ("Procurar emuladores") ou quando a configuração muda.
/// </summary>
public sealed partial class EmulationService
{
    private readonly EmulationConfigStore _store;
    private readonly EmulatorDetector _detector;
    private readonly IEmulationFileSystem _disk;
    private readonly ILogger<EmulationService> _logger;
    private readonly string? _scanCacheFile;
    private readonly RomScanCache _scanCache;
    private EmulatorInventory? _inventory;

    public EmulationService(EmulationConfigStore store, IRegistryReader registry, IEmulationFileSystem disk, EmulationPaths paths, ILogger<EmulationService> logger, string? scanCacheFile = null)
    {
        _store = store;
        _disk = disk;
        Paths = paths;
        _logger = logger;
        _scanCacheFile = scanCacheFile;
        _scanCache = scanCacheFile is null ? new RomScanCache() : RomScanCache.Load(scanCacheFile);
        _detector = new EmulatorDetector(disk, registry, paths, Catalog);
        Config = store.Load();
        Source = new SavingSource(new EmulationLibrarySource(() => Config, new RomScanner(disk, Catalog, _scanCache), () => Inventory), SaveScanCache);
    }

    /// <summary>A configuração mudou (pastas, escolhas, emuladores): a tela relê; a biblioteca é lida de novo por quem mudou.</summary>
    public event EventHandler? Changed;

    public static EmulatorCatalog Catalog => EmulatorCatalog.Default;

    public EmulationPaths Paths { get; }

    public EmulationConfig Config { get; private set; }

    /// <summary>A fonte "Emulação" da biblioteca (ADR-0019); guarda a lista de arquivos das pastas depois de cada leitura.</summary>
    public ILibrarySource Source { get; }

    /// <summary>Emuladores achados (detectados na primeira vez que alguém pergunta).</summary>
    public EmulatorInventory Inventory => _inventory ??= Detect();

    /// <summary>Lê de novo onde estão os emuladores e os cores do RetroArch.</summary>
    public EmulatorInventory Redetect()
    {
        _inventory = Detect();
        Changed?.Invoke(this, EventArgs.Empty);
        return _inventory;
    }

    /// <summary>Muda e grava a configuração. Falha de disco vira falso (a mudança vale nesta sessão).</summary>
    public bool Update(Func<EmulationConfig, EmulationConfig> change)
    {
        ArgumentNullException.ThrowIfNull(change);
        var before = Config;
        Config = change(Config);
        bool saved = true;
        try
        {
            _store.Save(Config);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogSaveFailed(_logger, ex, _store.FilePath);
            saved = false;
        }

        // Executáveis escolhidos e emuladores personalizados mudam o que foi achado.
        if (!ReferenceEquals(before.EmulatorPaths, Config.EmulatorPaths) || !ReferenceEquals(before.CustomEmulators, Config.CustomEmulators))
        {
            _inventory = Detect();
        }

        Changed?.Invoke(this, EventArgs.Empty);
        return saved;
    }

    /// <summary>Como abrir a ROM agora (ou por que não dá).</summary>
    public EmulatorResolution Resolve(Game game) => EmulatorResolver.Resolve(game, Config, Inventory, _disk, Paths);

    /// <summary>Opções para o sistema do jogo, na ordem sugerida.</summary>
    public IReadOnlyList<EmulatorOption> OptionsFor(Game game) =>
        game.Platform is { } system ? Inventory.OptionsFor(system, game.RomPath is { } rom ? WinPath.Extension(rom) : null) : [];

    /// <summary>"RetroArch · Snes9x", "Nenhum emulador" (detalhes do jogo).</summary>
    public string Describe(Game game) => Resolve(game) switch
    {
        EmulatorLaunch launch => launch.Option.Label,
        EmulatorUnavailable { Title: "Arquivo não encontrado" } => "Arquivo não encontrado",
        _ => "Nenhum emulador",
    };

    /// <summary>Opção em uso para o sistema (escolhida ou sugerida); nula se nenhum emulador abre o sistema.</summary>
    public EmulatorOption? CurrentFor(string system)
    {
        var options = Inventory.OptionsFor(system);
        return EmulatorResolver.Pick(options, Config.SystemChoices.GetValueOrDefault(system)) ?? (options.Count > 0 ? options[0] : null);
    }

    /// <summary>Avisos de BIOS que faltam nos emuladores em uso para os sistemas pedidos.</summary>
    public IReadOnlyList<string> MissingBios(IEnumerable<string> systems) =>
        [.. systems.Distinct(StringComparer.OrdinalIgnoreCase)
            .Select(s => (System: s, Option: CurrentFor(s)))
            .Where(p => p.Option is not null)
            .SelectMany(p => BiosCheck.Missing(p.Option!, p.System, _disk, Paths, Catalog))
            .Distinct(StringComparer.Ordinal)];

    /// <summary>Sistemas que a pasta tem (ela mesma com nome de sistema, ou as subpastas reconhecidas).</summary>
    public IReadOnlyList<string> SystemsInFolder(string path) =>
        [.. new RomScanner(_disk, Catalog).SystemFolders(new RomFolder(path)).Select(f => f.System.Key).Distinct(StringComparer.OrdinalIgnoreCase)];

    /// <summary>Nome do sistema ("SNES"), ou a chave quando desconhecido.</summary>
    public static string SystemName(string system) => ScreenScraperSystems.ForKey(system)?.Name ?? system;

    /// <summary>Guarda a lista de arquivos das pastas de ROMs para a próxima leitura (chamado depois de ler a biblioteca).</summary>
    public void SaveScanCache()
    {
        if (_scanCacheFile is not null)
        {
            _scanCache.Save(_scanCacheFile);
        }
    }

    /// <summary>
    /// Atalhos do RetroArch (EMU-006): Select + Start fecha o RetroArch. Só troca opções que o usuário não mudou e
    /// guarda uma cópia do <c>retroarch.cfg</c> antes (<c>retroarch.cfg.tako-backup</c>, a primeira cópia nunca é
    /// sobrescrita).
    /// </summary>
    public string ApplyRetroArchHotkeys()
    {
        if (Inventory.Find("retroarch") is not { } retroArch)
        {
            return "RetroArch não encontrado neste PC";
        }

        string? file = RetroArch.ConfigFile(_disk, retroArch.Folder, Paths);
        if (file is null)
        {
            return "Abra o RetroArch uma vez (ele cria o retroarch.cfg) e tente de novo";
        }

        try
        {
            string text = File.ReadAllText(file);
            var change = RetroArch.Merge(text, Catalog.RetroArchHotkeys);
            if (!change.Changed)
            {
                return change.KeptUserValue.Count > 0
                    ? "O RetroArch já tem atalhos seus; nada foi mudado"
                    : "Os atalhos do RetroArch já estavam aplicados";
            }

            string backup = file + ".tako-backup";
            if (!File.Exists(backup))
            {
                File.Copy(file, backup);
            }

            File.WriteAllText(file, change.Text);
            LogRetroArchHotkeys(_logger, file, change.Applied.Count);
            return change.KeptUserValue.Count > 0
                ? "Atalhos aplicados; os que você já tinha mudado ficaram como estavam"
                : "Atalhos aplicados: segure Select + Start para sair do RetroArch";
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogSaveFailed(_logger, ex, file);
            return $"Não foi possível mudar o retroarch.cfg: {ex.Message}";
        }
    }

    private EmulatorInventory Detect()
    {
        try
        {
            var inventory = _detector.Detect(Config);
            string found = string.Join(", ", inventory.Emulators.Select(e => $"{e.Id} ({e.Origin})"));
            LogDetected(_logger, inventory.Emulators.Count, found);
            return inventory;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            LogDetectFailed(_logger, ex);
            return new EmulatorInventory([], Catalog);
        }
    }

    private sealed class SavingSource(ILibrarySource inner, Action afterScan) : ILibrarySource
    {
        public GameStore Store => inner.Store;

        public LibraryScan Scan()
        {
            var scan = inner.Scan();
            afterScan();
            return scan;
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Emuladores encontrados: {Count} ({Emulators})")]
    private static partial void LogDetected(ILogger logger, int count, string emulators);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Falha ao procurar emuladores")]
    private static partial void LogDetectFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Falha ao gravar {File}")]
    private static partial void LogSaveFailed(ILogger logger, Exception exception, string file);

    [LoggerMessage(Level = LogLevel.Information, Message = "Atalhos do RetroArch aplicados em {File} ({Count} opções)")]
    private static partial void LogRetroArchHotkeys(ILogger logger, string file, int count);
}
