namespace Launcher.Core.Platform;

/// <summary>
/// Linha de comando gravada em <c>HKCU\Software\Microsoft\Windows\CurrentVersion\Run</c> para abrir o Tako ao entrar no
/// Windows. Só conta como ligada quando aponta para o executável atual: um valor antigo (portátil movido, outra cópia)
/// abriria outro programa ou nada.
/// </summary>
public static class StartupCommand
{
    public static string For(string executablePath)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(executablePath);
        return $"\"{executablePath}\"";
    }

    /// <summary>Caminho do executável numa linha de comando (com ou sem aspas, ignorando argumentos).</summary>
    public static string? ExecutableOf(string? command)
    {
        string text = command?.Trim() ?? string.Empty;
        if (text.Length == 0)
        {
            return null;
        }

        if (text[0] == '"')
        {
            int close = text.IndexOf('"', 1);
            return close > 1 ? text[1..close] : null;
        }

        int exe = text.IndexOf(".exe", StringComparison.OrdinalIgnoreCase);
        return exe > 0 ? text[..(exe + 4)] : text.Split(' ')[0];
    }

    public static bool PointsTo(string? command, string executablePath) =>
        ExecutableOf(command) is { } exe && string.Equals(Normalize(exe), Normalize(executablePath), StringComparison.OrdinalIgnoreCase);

    private static string Normalize(string path) => path.Trim().Replace('/', '\\').TrimEnd('\\');
}
