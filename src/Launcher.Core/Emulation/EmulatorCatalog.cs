using System.Text.Json;
using System.Text.Json.Serialization;

namespace Launcher.Core.Emulation;

/// <summary>
/// Um emulador conhecido (EMU-001, ADR-0024): como achá-lo no PC e como abrir um jogo nele. Dados de
/// <c>emulators.json</c>, embutido no Tako e testado.
/// </summary>
public sealed record EmulatorDefinition
{
    /// <summary>Chave estável ("retroarch", "pcsx2"), guardada nas escolhas do usuário.</summary>
    public required string Id { get; init; }

    public required string Name { get; init; }

    /// <summary>Executáveis aceitos, em ordem de preferência (o primeiro achado vence).</summary>
    public IReadOnlyList<string> Executables { get; init; } = [];

    /// <summary>Sistemas (chaves de <see cref="Library.Metadata.ScreenScraper.ScreenScraperSystems"/>); o RetroArch tira os dele dos cores.</summary>
    public IReadOnlyList<string> Systems { get; init; } = [];

    /// <summary>RetroArch: os sistemas vêm dos cores instalados.</summary>
    public bool UsesCores { get; init; }

    /// <summary>Argumentos com <c>{rom}</c>, <c>{romdir}</c>, <c>{romname}</c>, <c>{core}</c>, <c>{fullscreen}</c> e <c>{config}</c>.</summary>
    public string Arguments { get; init; } = "{rom}";

    /// <summary>O que entra no lugar de <c>{fullscreen}</c> (vazio quando o emulador não tem opção de linha de comando).</summary>
    public string Fullscreen { get; init; } = string.Empty;

    /// <summary>Argumento da configuração extra por jogo (RetroArch: <c>--appendconfig {config}</c>); vazio = não suporta.</summary>
    public string ConfigArgument { get; init; } = string.Empty;

    /// <summary>Tecla que o próprio emulador usa para sair (passo 2 do fechamento gracioso); nula = só fechar a janela.</summary>
    public string? ExitKey { get; init; }

    /// <summary>Quantas vezes apertar a tecla (o RetroArch pede duas por padrão).</summary>
    public int ExitKeyPresses { get; init; } = 1;

    /// <summary>Pastas padrão, com <c>{ProgramFiles}</c>, <c>{LocalAppData}</c>... (ver <see cref="EmulationPaths"/>).</summary>
    public IReadOnlyList<string> Folders { get; init; } = [];

    /// <summary>Nomes do app no Scoop (<c>scoop\apps\&lt;nome&gt;\current</c>).</summary>
    public IReadOnlyList<string> Scoop { get; init; } = [];

    /// <summary>Ids do pacote no winget (<c>WinGet\Packages\&lt;id&gt;_*</c>).</summary>
    public IReadOnlyList<string> Winget { get; init; } = [];

    /// <summary>Começo do nome em "Aplicativos instalados" (registro Uninstall) para achar a pasta.</summary>
    public IReadOnlyList<string> UninstallNames { get; init; } = [];
}

/// <summary>Core do RetroArch sugerido para um sistema (o primeiro instalado da lista vira o padrão).</summary>
public sealed record CoreSuggestion(string Core, string Name, IReadOnlyList<string> Systems);

/// <summary>Uma opção do <c>retroarch.cfg</c> que o Tako pode aplicar sem sobrescrever o que o usuário escolheu.</summary>
/// <param name="Unset">Valores tratados como "não configurado" (o padrão do RetroArch); outro valor é do usuário e fica.</param>
public sealed record RetroArchSetting(string Key, string Value, IReadOnlyList<string> Unset);

/// <summary>BIOS ou firmware que um emulador exige: onde ele procura e como reconhecer o arquivo.</summary>
/// <param name="Folders">Pastas, com <c>{exe}</c> (pasta do emulador) e os marcadores de <see cref="EmulationPaths"/>.</param>
/// <param name="Patterns">Nome exato ("prod.keys") ou extensão (".bin"): basta um arquivo que combine.</param>
public sealed record BiosRequirement(string Emulator, string System, string Label, IReadOnlyList<string> Folders, IReadOnlyList<string> Patterns);

/// <summary>Tabela de emuladores, cores sugeridos, pastas por sistema e BIOS conhecidas (ADR-0024).</summary>
public sealed record EmulatorCatalog
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNameCaseInsensitive = true,
        ReadCommentHandling = JsonCommentHandling.Skip,
        AllowTrailingCommas = true,
    };

    private static readonly Lazy<EmulatorCatalog> Embedded = new(LoadEmbedded);

    public int Version { get; init; }

    public IReadOnlyList<EmulatorDefinition> Emulators { get; init; } = [];

    /// <summary>Nomes de pasta por sistema (inclui os do ES-DE): "psx" → ps1.</summary>
    public IReadOnlyDictionary<string, IReadOnlyList<string>> SystemFolders { get; init; } = new Dictionary<string, IReadOnlyList<string>>();

    /// <summary>Ordem de preferência dos emuladores por sistema (sem entrada = ordem da tabela).</summary>
    public IReadOnlyDictionary<string, IReadOnlyList<string>> Preferred { get; init; } = new Dictionary<string, IReadOnlyList<string>>();

    public IReadOnlyList<CoreSuggestion> RetroArchCores { get; init; } = [];

    /// <summary>Banco de dados do <c>.info</c> do core ("Sony - PlayStation") → sistema.</summary>
    public IReadOnlyDictionary<string, string> RetroArchDatabases { get; init; } = new Dictionary<string, string>();

    public IReadOnlyList<RetroArchSetting> RetroArchHotkeys { get; init; } = [];

    public IReadOnlyList<BiosRequirement> Bios { get; init; } = [];

    /// <summary>A tabela embutida no Tako.</summary>
    public static EmulatorCatalog Default => Embedded.Value;

    public EmulatorDefinition? Find(string? id) =>
        id is null ? null : Emulators.FirstOrDefault(e => string.Equals(e.Id, id, StringComparison.OrdinalIgnoreCase));

    /// <summary>Sistema pelo nome da pasta ("PSX", "snes", "Genesis"); nulo se não é uma pasta de sistema conhecida.</summary>
    public string? SystemForFolder(string folderName)
    {
        ArgumentNullException.ThrowIfNull(folderName);
        string name = folderName.Trim();
        foreach (var (system, aliases) in SystemFolders)
        {
            if (string.Equals(system, name, StringComparison.OrdinalIgnoreCase) || aliases.Any(a => string.Equals(a, name, StringComparison.OrdinalIgnoreCase)))
            {
                return system;
            }
        }

        return null;
    }

    /// <summary>Lê a tabela (JSON); lança <see cref="JsonException"/> se o formato estiver errado.</summary>
    public static EmulatorCatalog Parse(string json)
    {
        ArgumentNullException.ThrowIfNull(json);
        var raw = JsonSerializer.Deserialize<RawCatalog>(json, Options) ?? throw new JsonException("Tabela de emuladores vazia.");
        return new EmulatorCatalog
        {
            Version = raw.Version,
            Emulators = raw.Emulators ?? [],
            SystemFolders = (raw.SystemFolders ?? []).ToDictionary(p => p.Key, p => (IReadOnlyList<string>)p.Value, StringComparer.OrdinalIgnoreCase),
            Preferred = (raw.Preferred ?? []).ToDictionary(p => p.Key, p => (IReadOnlyList<string>)p.Value, StringComparer.OrdinalIgnoreCase),
            RetroArchCores = raw.RetroArchCores ?? [],
            RetroArchDatabases = new Dictionary<string, string>(raw.RetroArchDatabases ?? [], StringComparer.OrdinalIgnoreCase),
            RetroArchHotkeys = raw.RetroArchHotkeys ?? [],
            Bios = raw.Bios ?? [],
        };
    }

    private static EmulatorCatalog LoadEmbedded()
    {
        using var stream = typeof(EmulatorCatalog).Assembly.GetManifestResourceStream("Launcher.Core.Emulation.emulators.json")
            ?? throw new InvalidOperationException("emulators.json não está embutido no Launcher.Core.");
        using var reader = new StreamReader(stream);
        return Parse(reader.ReadToEnd());
    }

    private sealed record RawCatalog
    {
        public int Version { get; init; }

        public List<EmulatorDefinition>? Emulators { get; init; }

        public Dictionary<string, List<string>>? SystemFolders { get; init; }

        public Dictionary<string, List<string>>? Preferred { get; init; }

        public List<CoreSuggestion>? RetroArchCores { get; init; }

        public Dictionary<string, string>? RetroArchDatabases { get; init; }

        public List<RetroArchSetting>? RetroArchHotkeys { get; init; }

        [JsonPropertyName("bios")]
        public List<BiosRequirement>? Bios { get; init; }
    }
}
