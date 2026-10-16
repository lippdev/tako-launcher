namespace Launcher.Core.Emulation;

/// <summary>
/// Disco atrás de interface para a detecção de emuladores e a leitura das pastas de ROMs: os testes usam pastas em
/// memória (fixtures) e rodam no Mac/Linux com caminhos do Windows.
/// </summary>
public interface IEmulationFileSystem
{
    bool FileExists(string path);

    bool DirectoryExists(string path);

    /// <summary>Arquivos (caminhos completos) direto na pasta; vazio se ela não existe.</summary>
    IReadOnlyList<string> Files(string directory);

    /// <summary>Subpastas (caminhos completos) direto na pasta; vazio se ela não existe.</summary>
    IReadOnlyList<string> Directories(string directory);

    /// <summary>Texto do arquivo; nulo se não existe ou não dá para ler.</summary>
    string? ReadText(string path);

    /// <summary>Última modificação (arquivo ou pasta); nula se não existe.</summary>
    DateTimeOffset? LastWrite(string path);
}

/// <summary>Disco de verdade. Pasta sem permissão ou que sumiu no meio = vazia.</summary>
public sealed class PhysicalEmulationFileSystem : IEmulationFileSystem
{
    public bool FileExists(string path) => File.Exists(path);

    public bool DirectoryExists(string path) => Directory.Exists(path);

    public IReadOnlyList<string> Files(string directory) => Safe(() => Directory.GetFiles(directory));

    public IReadOnlyList<string> Directories(string directory) => Safe(() => Directory.GetDirectories(directory));

    public string? ReadText(string path)
    {
        try
        {
            return File.Exists(path) ? File.ReadAllText(path) : null;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    public DateTimeOffset? LastWrite(string path)
    {
        try
        {
            return File.Exists(path) ? File.GetLastWriteTimeUtc(path)
                : Directory.Exists(path) ? Directory.GetLastWriteTimeUtc(path)
                : null;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    private static string[] Safe(Func<string[]> read)
    {
        try
        {
            return read();
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            return [];
        }
    }
}

/// <summary>
/// Pastas do Windows usadas nas definições (<c>{ProgramFiles}</c>, <c>{LocalAppData}</c>...). Nos testes, valores fixos.
/// </summary>
public sealed record EmulationPaths(
    string ProgramFiles,
    string ProgramFilesX86,
    string LocalAppData,
    string AppData,
    string UserProfile,
    string Documents,
    string? Scoop = null,
    string? ScoopGlobal = null)
{
    /// <summary>Pastas deste PC (Scoop pela variável <c>SCOOP</c>, senão <c>%USERPROFILE%\scoop</c>).</summary>
    public static EmulationPaths FromEnvironment()
    {
        string profile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        string? scoop = Environment.GetEnvironmentVariable("SCOOP");
        string? scoopGlobal = Environment.GetEnvironmentVariable("SCOOP_GLOBAL");
        return new EmulationPaths(
            Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
            Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            profile,
            Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments),
            string.IsNullOrWhiteSpace(scoop) ? WinPath.Combine(profile, "scoop") : scoop,
            string.IsNullOrWhiteSpace(scoopGlobal) ? WinPath.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData), "scoop") : scoopGlobal);
    }

    /// <summary>Troca os marcadores; <c>{exe}</c> vira <paramref name="exeFolder"/>. Marcador sem valor = nulo (pasta ignorada).</summary>
    public string? Expand(string template, string? exeFolder = null)
    {
        ArgumentNullException.ThrowIfNull(template);
        var values = new (string Token, string? Value)[]
        {
            ("{ProgramFiles}", ProgramFiles), ("{ProgramFilesX86}", ProgramFilesX86), ("{LocalAppData}", LocalAppData),
            ("{AppData}", AppData), ("{UserProfile}", UserProfile), ("{Documents}", Documents), ("{Scoop}", Scoop), ("{exe}", exeFolder),
        };
        string result = template;
        foreach (var (token, value) in values)
        {
            if (!result.Contains(token, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (string.IsNullOrWhiteSpace(value))
            {
                return null;
            }

            result = result.Replace(token, value.TrimEnd('\\', '/'), StringComparison.OrdinalIgnoreCase);
        }

        return result;
    }
}

/// <summary>
/// Caminhos do Windows sem depender do sistema em que o código roda (<see cref="Path"/> no Linux não entende "\"):
/// aceita "\" e "/", junta com "\".
/// </summary>
public static class WinPath
{
    private static readonly char[] Separators = ['\\', '/'];

    public static string Combine(string folder, string name)
    {
        ArgumentNullException.ThrowIfNull(folder);
        ArgumentNullException.ThrowIfNull(name);
        return folder.Length == 0 ? name : folder.TrimEnd(Separators) + "\\" + name.TrimStart(Separators);
    }

    /// <summary>Pasta do caminho; vazia se não há separador.</summary>
    public static string Directory(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        string trimmed = path.TrimEnd(Separators);
        int slash = trimmed.LastIndexOfAny(Separators);
        return slash <= 0 ? string.Empty : trimmed[..slash];
    }

    public static string FileName(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        string trimmed = path.TrimEnd(Separators);
        int slash = trimmed.LastIndexOfAny(Separators);
        return slash < 0 ? trimmed : trimmed[(slash + 1)..];
    }

    /// <summary>Extensão em minúsculas, com ponto (".sfc"); vazia se não há.</summary>
    public static string Extension(string path)
    {
        string name = FileName(path);
        int dot = name.LastIndexOf('.');
        return dot <= 0 ? string.Empty : name[dot..].ToLowerInvariant();
    }

    public static string NameWithoutExtension(string path)
    {
        string name = FileName(path);
        int dot = name.LastIndexOf('.');
        return dot <= 0 ? name : name[..dot];
    }

    /// <summary>Caminho relativo (de um .m3u ou .cue) resolvido contra a pasta; absoluto fica como está.</summary>
    public static string Resolve(string folder, string relative)
    {
        ArgumentNullException.ThrowIfNull(relative);
        string clean = relative.Trim().Trim('"');
        bool absolute = clean.StartsWith('\\') || clean.StartsWith('/') || (clean.Length > 1 && clean[1] == ':');
        if (absolute)
        {
            return clean;
        }

        string result = folder;
        foreach (string part in clean.Split(Separators, StringSplitOptions.RemoveEmptyEntries))
        {
            result = part switch
            {
                "." => result,
                ".." => Directory(result),
                _ => Combine(result, part),
            };
        }

        return result;
    }

    /// <summary>Forma para comparar caminhos: "\" e sem diferenciar maiúsculas.</summary>
    public static string Normalize(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        return path.Replace('/', '\\').TrimEnd('\\').ToUpperInvariant();
    }

    public static bool SamePath(string a, string b) => Normalize(a) == Normalize(b);
}
