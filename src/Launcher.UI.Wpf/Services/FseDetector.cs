using System.IO;
using System.Security;
using Launcher.Core.Platform;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Detecta o suporte ao Xbox mode / Full Screen Experience (MVP-008) com os sinais do spike MVP-007 (lógica copiada de
/// <c>tools/Launcher.FseProbe</c>, sem depender dela). Só lê o registro e pastas do usuário; a classificação fica em
/// <see cref="FseClassifier"/>. O Tako continua em tela cheia convencional: virar app inicial do Xbox mode depende de
/// autorização da Microsoft.
/// </summary>
public sealed class FseDetector
{
    private const string XboxAppFamily = "Microsoft.GamingApp_8wekyb3d8bbwe";
    private const string PackageRepository = @"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";

    private readonly Lazy<FseAssessment> _assessment = new(() => FseClassifier.Classify(ReadSignals()));

    public FseAssessment Assessment => _assessment.Value;

    public static FseSignals ReadSignals() => new(ReadBuild(), ReadXboxAppInstalled(), ReadDeviceForm());

    private static int? ReadBuild()
    {
        // Environment.OSVersion.Version.Build também serve: o .NET 5+ não mente a versão (sem manifesto de compatibilidade).
        object? value = Read(() => Registry.GetValue(@"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuild", null));
        return value is string text && int.TryParse(text, out int build) ? build
            : OperatingSystem.IsWindows() ? Environment.OSVersion.Version.Build : null;
    }

    private static int? ReadDeviceForm() =>
        Read(() => Registry.GetValue(@"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\OEM", "DeviceForm", null)) is int form ? form : null;

    /// <summary>
    /// App Xbox instalado para o usuário: pasta de dados do pacote ou entrada no repositório de pacotes do AppModel.
    /// Sem conseguir ler nenhum dos dois, não sabemos (nulo).
    /// </summary>
    private static bool? ReadXboxAppInstalled()
    {
        string packages = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Packages", XboxAppFamily);
        if (Directory.Exists(packages))
        {
            return true;
        }

        return Read(() =>
        {
            using var repository = Registry.CurrentUser.OpenSubKey(PackageRepository);
            return repository?.GetSubKeyNames().Any(name => name.StartsWith("Microsoft.GamingApp_", StringComparison.OrdinalIgnoreCase));
        });
    }

    private static T? Read<T>(Func<T?> read)
    {
        try
        {
            return read();
        }
        catch (Exception ex) when (ex is SecurityException or UnauthorizedAccessException or IOException)
        {
            return default;
        }
    }
}
