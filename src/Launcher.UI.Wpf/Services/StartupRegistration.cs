using System.IO;
using System.Security;
using Launcher.Core.Platform;
using Microsoft.Win32;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// "Abrir em tela cheia ao iniciar o Windows": valor <c>Tako</c> em <c>HKCU\...\CurrentVersion\Run</c> apontando para o
/// executável real (<see cref="Environment.ProcessPath"/>). Só nas versões publicadas (instalada e portátil): rodando pelo
/// <c>dotnet run</c>, o caminho é de uma pasta de build. O registro é a fonte da verdade; a opção na tela só o reflete.
/// </summary>
public sealed class StartupRegistration(string keyPath = StartupRegistration.RunKey, string valueName = StartupRegistration.ValueName)
{
    public const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run";
    public const string ValueName = "Tako";

    public static bool IsSupported => UpdateService.IsPublishedBuild && Environment.ProcessPath is not null;

    /// <summary>Ligado só quando o valor aponta para este executável.</summary>
    public bool IsEnabled(string executablePath)
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(keyPath);
            return StartupCommand.PointsTo(key?.GetValue(valueName) as string, executablePath);
        }
        catch (Exception ex) when (ex is SecurityException or UnauthorizedAccessException or IOException)
        {
            return false;
        }
    }

    /// <summary>Grava ou remove o valor. Falha de permissão vira exceção para a UI avisar e manter o estado real.</summary>
    public void Set(bool enabled, string executablePath)
    {
        try
        {
            using var key = Registry.CurrentUser.CreateSubKey(keyPath, writable: true);
            if (enabled)
            {
                key.SetValue(valueName, StartupCommand.For(executablePath), RegistryValueKind.String);
            }
            else
            {
                key.DeleteValue(valueName, throwOnMissingValue: false);
            }
        }
        catch (SecurityException ex)
        {
            throw new UnauthorizedAccessException(ex.Message, ex);
        }
    }

    /// <summary>Exceções de registro que a UI trata como "não foi possível alterar".</summary>
    public static bool IsExpectedFailure(Exception ex) => ex is UnauthorizedAccessException or IOException;
}
