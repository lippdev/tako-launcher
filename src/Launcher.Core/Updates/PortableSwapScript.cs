using System.Globalization;
using System.Text;

namespace Launcher.Core.Updates;

/// <summary>
/// Script <c>.cmd</c> que troca o <c>Tako.exe</c> portátil: um .exe em uso não pode ser sobrescrito, então ele espera
/// todos os processos que usam o arquivo saírem (a UI <b>e</b> o supervisor, ADR-0012), move o novo arquivo por cima e,
/// se pedido, abre o Tako de novo (que volta a iniciar pelo supervisor).
/// </summary>
public static class PortableSwapScript
{
    public static string Build(string downloadedFile, string executable, IEnumerable<int> waitForProcessIds, bool relaunch)
    {
        ArgumentException.ThrowIfNullOrEmpty(downloadedFile);
        ArgumentException.ThrowIfNullOrEmpty(executable);
        ArgumentNullException.ThrowIfNull(waitForProcessIds);
        var script = new StringBuilder("@echo off\r\n");
        int label = 0;
        foreach (int pid in waitForProcessIds.Distinct())
        {
            string id = pid.ToString(CultureInfo.InvariantCulture);
            // ping como espera: timeout.exe falha sem console interativo.
            script.Append(CultureInfo.InvariantCulture, $":wait{label}\r\ntasklist /fi \"PID eq {id}\" | find \"{id}\" >nul && (ping -n 2 127.0.0.1 >nul & goto wait{label})\r\n");
            label++;
        }

        script.Append(CultureInfo.InvariantCulture, $"move /y \"{downloadedFile}\" \"{executable}\" >nul\r\n");
        if (relaunch)
        {
            script.Append(CultureInfo.InvariantCulture, $"start \"\" \"{executable}\"\r\n");
        }

        return script.ToString();
    }
}
