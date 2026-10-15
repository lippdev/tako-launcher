using System.IO;
using Launcher.Core.Library;
using Launcher.Core.Library.Xbox;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>Fonte Xbox da biblioteca: procura quando ligada nas Configurações e guarda o resultado para a tela.</summary>
public sealed partial class XboxLibraryService(IXboxSystem system, SettingsService settings, ILogger<XboxLibraryService> logger)
{
    private XboxScanResult? _last;

    /// <summary>Jogos do app Xbox / Microsoft Store; vazio se a fonte estiver desligada.</summary>
    public IReadOnlyList<Game> Discover()
    {
        if (!settings.XboxLibraryEnabled)
        {
            _last = null;
            return [];
        }

        try
        {
            _last = XboxLibraryScanner.Scan(system);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Uma pasta que some no meio da leitura não derruba a biblioteca: Steam e adicionados continuam.
            LogScanFailed(logger, ex);
            _last = new XboxScanResult([], XboxAvailability.Unknown);
        }

        LogScanned(logger, _last.Games.Count, _last.Availability);
        return [.. _last.Games.Select(XboxGames.ToGame)];
    }

    /// <summary>Linha de ajuda nas Configurações (inclui o aviso de app Xbox ou Serviços de Jogos ausentes).</summary>
    public string Description
    {
        get
        {
            if (!settings.XboxLibraryEnabled)
            {
                return XboxGameRules.Describe(enabled: false, XboxAvailability.Ready, 0);
            }

            var availability = _last?.Availability ?? XboxGameRules.Availability(SafeFamilies(), Safe(system.GamingServicesInstalled));
            return XboxGameRules.Describe(enabled: true, availability, _last?.Games.Count ?? 0);
        }
    }

    private HashSet<string>? SafeFamilies()
    {
        var packages = Safe(system.UserPackages);
        if (packages is null)
        {
            return null;
        }

        var families = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var package in packages)
        {
            if (PackageIdentity.TryFamilyFromFullName(package.FullName, out string family))
            {
                families.Add(family);
            }
        }

        return families;
    }

    private static T? Safe<T>(Func<T> read)
    {
        try
        {
            return read();
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            return default;
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Xbox: {Count} jogos encontrados ({Availability})")]
    private static partial void LogScanned(ILogger logger, int count, XboxAvailability availability);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Falha ao procurar jogos do app Xbox")]
    private static partial void LogScanFailed(ILogger logger, Exception exception);
}
