using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32;
using Windows.Management.Deployment;

namespace Launcher.FseProbe;

/// <summary>Coleta os sinais de ambiente relevantes para o FSE / Xbox mode (MVP-007 e MVP-008).</summary>
internal static class EnvironmentReport
{
    private const string GamingAppExtension = "windows.gamingApp";
    private const string GamingHomeCapability = "Microsoft.appCategory.gamingHome_8wekyb3d8bbwe";

    public static int Run()
    {
        var report = new StringBuilder();
        void Line(string text) => report.AppendLine(text);

        Line($"Tako FseProbe — {DateTimeOffset.Now:yyyy-MM-dd HH:mm:ss zzz}");
        Line(string.Empty);

        using (var nt = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Microsoft\Windows NT\CurrentVersion"))
        {
            Line("## Sistema");
            Line($"Build: {nt?.GetValue("CurrentBuild")}.{nt?.GetValue("UBR")}  Versão: {nt?.GetValue("DisplayVersion")}  Edição: {nt?.GetValue("EditionID")}");
        }

        // Valor citado por ferramentas da comunidade: 46 = "Gaming handheld" (não documentado pela Microsoft).
        object? deviceForm = Registry.GetValue(@"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\OEM", "DeviceForm", null);
        Line($"DeviceForm (OEM): {deviceForm ?? "(ausente)"}");

        object? devMode = Registry.GetValue(@"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock", "AllowDevelopmentWithoutDevLicense", null);
        Line($"Modo de Desenvolvedor: {(devMode is int value && value == 1 ? "ligado" : "desligado")}");
        Line(string.Empty);

        Line($"## Pacotes com a extensão {GamingAppExtension} (candidatos a home app)");
        try
        {
            int found = 0;
            foreach (var package in new PackageManager().FindPackagesForUser(string.Empty))
            {
                string? manifest = ReadManifest(package);
                bool isGamingHomeCandidate = manifest?.Contains(GamingAppExtension, StringComparison.Ordinal) == true;
                bool isXboxApp = package.Id.FamilyName.StartsWith("Microsoft.GamingApp", StringComparison.Ordinal);
                if (!isGamingHomeCandidate && !isXboxApp)
                {
                    continue;
                }

                found++;
                bool hasCapability = manifest?.Contains(GamingHomeCapability, StringComparison.Ordinal) == true;
                Line($"- {package.Id.FamilyName} v{package.Id.Version.Major}.{package.Id.Version.Minor}.{package.Id.Version.Build}.{package.Id.Version.Revision}");
                Line($"  extensão gamingApp: {isGamingHomeCandidate}; capability gamingHome: {hasCapability}; assinatura: {package.SignatureKind}; modo dev: {package.IsDevelopmentMode}");
            }

            if (found == 0)
            {
                Line("(nenhum)");
            }
        }
        catch (Exception ex) when (ex is UnauthorizedAccessException or COMException or InvalidOperationException)
        {
            Line($"Falha ao listar pacotes: {ex.Message}");
        }

        Line(string.Empty);
        Line("## Processos relacionados");
        var related = Process.GetProcesses()
            .Select(p => p.ProcessName)
            .Where(n => n.Contains("Xbox", StringComparison.OrdinalIgnoreCase)
                || n.Contains("Gaming", StringComparison.OrdinalIgnoreCase)
                || n.Contains("FseProbe", StringComparison.OrdinalIgnoreCase))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Order(StringComparer.OrdinalIgnoreCase)
            .ToList();
        Line(related.Count > 0 ? string.Join(", ", related) : "(nenhum)");

        string text = report.ToString();
        string path = Path.Combine(Program.LogDirectory, $"report-{DateTime.Now.ToString("yyyyMMdd-HHmmss", CultureInfo.InvariantCulture)}.txt");
        File.WriteAllText(path, text);
        Console.WriteLine(text);
        Console.WriteLine($"Relatório salvo em {path}");
        return 0;
    }

    private static string? ReadManifest(Windows.ApplicationModel.Package package)
    {
        try
        {
            string path = Path.Combine(package.InstalledLocation.Path, "AppxManifest.xml");
            return File.Exists(path) ? File.ReadAllText(path) : null;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or COMException or FileNotFoundException)
        {
            // Pacotes de sistema podem negar acesso; não são candidatos relevantes.
            return null;
        }
    }
}
