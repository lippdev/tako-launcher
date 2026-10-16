using System.Text.Json;
using ConsoleMode.Runtime.Tv;

namespace Launcher.Core.Library.Metadata.SteamGridDb;

/// <summary>
/// SteamGridDB ligado ou não e a chave da API do próprio usuário (ADR-0023), como a IGDB e o ScreenScraper: nenhuma
/// chave no repositório nem no executável. <see cref="AutoFill"/> usa o SteamGridDB para jogos que ficaram sem capa;
/// <see cref="Animated"/> mostra também as capas animadas no seletor.
/// </summary>
public sealed record SteamGridDbSettings(bool Enabled, string ApiKey)
{
    public static readonly SteamGridDbSettings Off = new(false, string.Empty);

    /// <summary>"Usar o SteamGridDB quando faltar capa" (desligado por padrão).</summary>
    public bool AutoFill { get; init; }

    /// <summary>Mostrar capas animadas (webp/apng) no seletor (desligado por padrão: só estáticas).</summary>
    public bool Animated { get; init; }

    public bool HasKey => !string.IsNullOrWhiteSpace(ApiKey);

    /// <summary>Ligado e com chave: só então o Tako consulta o SteamGridDB.</summary>
    public bool IsActive => Enabled && HasKey;

    /// <summary>Texto colado costuma trazer espaços e quebras de linha: saem todos.</summary>
    public static string Clean(string? value) => string.Concat((value ?? string.Empty).Where(c => !char.IsWhiteSpace(c)));

    /// <summary>O que parece errado na chave, antes de ir à rede; nulo = parece certa (32 letras e números).</summary>
    public string? Problem()
    {
        if (ApiKey.Length == 0)
        {
            return "Cole a chave da API do SteamGridDB (passo 3).";
        }

        return ApiKey.Length is >= 20 and <= 64 && ApiKey.All(char.IsAsciiLetterOrDigit)
            ? null
            : "A chave parece incompleta: ela tem 32 letras e números. Copie de novo em Preferências › API.";
    }

    /// <summary>Nunca mostra a chave (logs, depuração).</summary>
    public override string ToString() => $"SteamGridDbSettings {{ Enabled = {Enabled}, ApiKey = {(ApiKey.Length == 0 ? "vazia" : "***")}, AutoFill = {AutoFill}, Animated = {Animated} }}";
}

/// <summary>
/// Guarda <see cref="SteamGridDbSettings"/> em <c>steamgriddb.dat</c>, o arquivo inteiro protegido com DPAPI, como a IGDB.
/// Arquivo ilegível conta como desligado.
/// </summary>
public sealed class SteamGridDbSettingsStore(string path, ISecretProtector protector)
{
    private readonly object _gate = new();
    private SteamGridDbSettings? _current;

    public event EventHandler? Changed;

    public SteamGridDbSettings Current
    {
        get
        {
            lock (_gate)
            {
                return _current ??= Load();
            }
        }
    }

    public void Save(SteamGridDbSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var clean = settings with { ApiKey = settings.ApiKey.Trim() };
        lock (_gate)
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(path))!);
            File.WriteAllText(path, protector.Protect(JsonSerializer.Serialize(new Stored(clean.Enabled, clean.ApiKey, clean.AutoFill, clean.Animated))));
            _current = clean;
        }

        Changed?.Invoke(this, EventArgs.Empty);
    }

    private SteamGridDbSettings Load()
    {
        try
        {
            if (File.Exists(path) && protector.TryUnprotect(File.ReadAllText(path).Trim(), out string json) && json.Length > 0
                && JsonSerializer.Deserialize<Stored>(json) is { } stored)
            {
                return new SteamGridDbSettings(stored.Enabled, stored.ApiKey ?? string.Empty) { AutoFill = stored.AutoFill, Animated = stored.Animated };
            }
        }
        catch (Exception ex) when (ex is JsonException or IOException or UnauthorizedAccessException)
        {
            // Ilegível: o usuário informa a chave de novo.
        }

        return SteamGridDbSettings.Off;
    }

    private sealed record Stored(bool Enabled, string? ApiKey, bool AutoFill, bool Animated);
}
