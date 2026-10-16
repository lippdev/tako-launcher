using System.Text.Json;
using System.Text.Json.Serialization;

namespace Launcher.Core.Emulation;

/// <summary>Emulador escolhido (e o core, no RetroArch).</summary>
public sealed record EmulatorChoice(string EmulatorId, string? Core = null);

/// <summary>Pasta de ROMs (EMU-003): de um sistema, ou raiz com subpastas por sistema (<see cref="System"/> nulo).</summary>
public sealed record RomFolder(string Path, string? System = null);

/// <summary>Emulador cadastrado pelo usuário (EMU-002).</summary>
public sealed record CustomEmulator
{
    public required string Id { get; init; }

    public required string Name { get; init; }

    public required string ExePath { get; init; }

    public IReadOnlyList<string> Systems { get; init; } = [];

    public string Arguments { get; init; } = "{rom}";

    public string Fullscreen { get; init; } = string.Empty;

    public static CustomEmulator Create(string exePath, IReadOnlyList<string> systems) => new()
    {
        Id = $"custom:{Guid.NewGuid():N}",
        Name = WinPath.NameWithoutExtension(exePath),
        ExePath = exePath,
        Systems = systems,
    };

    public EmulatorDefinition ToDefinition() => new()
    {
        Id = Id,
        Name = Name,
        Executables = [WinPath.FileName(ExePath)],
        Systems = Systems,
        Arguments = Arguments,
        Fullscreen = Fullscreen,
    };
}

/// <summary>O que vale só para um jogo (EMU-004/005): outro emulador ou core, outros argumentos, um .cfg extra do RetroArch.</summary>
public sealed record GameEmulation(EmulatorChoice? Choice = null, string? Arguments = null, string? ConfigFile = null)
{
    public bool IsEmpty => Choice is null && string.IsNullOrWhiteSpace(Arguments) && string.IsNullOrWhiteSpace(ConfigFile);
}

/// <summary>
/// Configurações › Emulação (ADR-0024), em <c>emulation.json</c> na pasta de dados do Tako. As coleções são
/// trocadas inteiras (<c>with</c>) e salvas por <see cref="EmulationConfigStore"/>.
/// </summary>
public sealed record EmulationConfig
{
    public static EmulationConfig Default { get; } = new();

    public IReadOnlyList<RomFolder> RomFolders { get; init; } = [];

    public IReadOnlyList<CustomEmulator> CustomEmulators { get; init; } = [];

    /// <summary>Executável escolhido à mão para um emulador da tabela (id → caminho).</summary>
    public IReadOnlyDictionary<string, string> EmulatorPaths { get; init; } = new Dictionary<string, string>();

    /// <summary>Emulador (e core) por sistema; sem escolha, vale a primeira opção sugerida.</summary>
    public IReadOnlyDictionary<string, EmulatorChoice> SystemChoices { get; init; } = new Dictionary<string, EmulatorChoice>();

    /// <summary>Argumentos no lugar dos da tabela, por emulador.</summary>
    public IReadOnlyDictionary<string, string> EmulatorArguments { get; init; } = new Dictionary<string, string>();

    /// <summary>Argumentos extras por sistema (somados aos do emulador).</summary>
    public IReadOnlyDictionary<string, string> SystemArguments { get; init; } = new Dictionary<string, string>();

    /// <summary>Por jogo (id do jogo no Tako).</summary>
    public IReadOnlyDictionary<string, GameEmulation> Games { get; init; } = new Dictionary<string, GameEmulation>();

    /// <summary>Abrir os emuladores em tela cheia (<c>{fullscreen}</c>).</summary>
    public bool Fullscreen { get; init; } = true;

    /// <summary>Segurar View + Menu (Select + Start) fecha o emulador e volta ao Tako (EMU-006).</summary>
    public bool ExitChord { get; init; } = true;

    public EmulationConfig WithGame(string gameId, GameEmulation emulation)
    {
        var games = new Dictionary<string, GameEmulation>(Games, StringComparer.Ordinal);
        if (emulation.IsEmpty)
        {
            games.Remove(gameId);
        }
        else
        {
            games[gameId] = emulation;
        }

        return this with { Games = games };
    }

    public EmulationConfig WithSystemChoice(string system, EmulatorChoice? choice) =>
        this with { SystemChoices = Set(SystemChoices, system, choice) };

    public EmulationConfig WithEmulatorArguments(string emulatorId, string? arguments) =>
        this with { EmulatorArguments = Set(EmulatorArguments, emulatorId, string.IsNullOrWhiteSpace(arguments) ? null : arguments.Trim()) };

    public EmulationConfig WithEmulatorPath(string emulatorId, string? exePath) =>
        this with { EmulatorPaths = Set(EmulatorPaths, emulatorId, string.IsNullOrWhiteSpace(exePath) ? null : exePath) };

    private static Dictionary<string, T> Set<T>(IReadOnlyDictionary<string, T> source, string key, T? value)
        where T : class
    {
        var copy = new Dictionary<string, T>(source, StringComparer.OrdinalIgnoreCase);
        if (value is null)
        {
            copy.Remove(key);
        }
        else
        {
            copy[key] = value;
        }

        return copy;
    }
}

/// <summary>Lê e grava <c>emulation.json</c>. Arquivo ausente ou inválido = configuração padrão (o inválido é guardado de lado).</summary>
public sealed class EmulationConfigStore(string path)
{
    private static readonly JsonSerializerOptions Options = new()
    {
        WriteIndented = true,
        PropertyNameCaseInsensitive = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public string FilePath { get; } = path;

    public EmulationConfig Load()
    {
        try
        {
            if (!File.Exists(FilePath))
            {
                return EmulationConfig.Default;
            }

            return JsonSerializer.Deserialize<EmulationConfig>(File.ReadAllText(FilePath), Options) ?? EmulationConfig.Default;
        }
        catch (JsonException)
        {
            TryMoveAside();
            return EmulationConfig.Default;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return EmulationConfig.Default;
        }
    }

    /// <summary>Grava (arquivo temporário + troca). Lança <see cref="IOException"/> se o disco falhar.</summary>
    public void Save(EmulationConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);
        string? folder = Path.GetDirectoryName(FilePath);
        if (!string.IsNullOrEmpty(folder))
        {
            Directory.CreateDirectory(folder);
        }

        string temp = FilePath + ".tmp";
        File.WriteAllText(temp, JsonSerializer.Serialize(config, Options));
        File.Move(temp, FilePath, overwrite: true);
    }

    private void TryMoveAside()
    {
        try
        {
            File.Move(FilePath, FilePath + ".invalido", overwrite: true);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Fica como está; a configuração padrão vale nesta sessão.
        }
    }
}
