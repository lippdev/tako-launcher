using System.Reflection;
using System.Text.Json;
using ConsoleMode.Runtime.Tv;

namespace Launcher.Core.Library.Metadata.ScreenScraper;

/// <summary>
/// ScreenScraper ligado ou não e a conta do próprio usuário em screenscraper.fr (ADR-0022). Cada pessoa usa a sua conta:
/// a cota diária e o número de consultas simultâneas são dela.
/// </summary>
public sealed record ScreenScraperSettings(bool Enabled, string User, string Password)
{
    public static readonly ScreenScraperSettings Off = new(false, string.Empty, string.Empty);

    public bool HasAccount => !string.IsNullOrWhiteSpace(User) && !string.IsNullOrEmpty(Password);

    /// <summary>Ligado e com conta: só então o provedor consulta (se a versão tiver as credenciais do software).</summary>
    public bool IsActive => Enabled && HasAccount;

    /// <summary>O que falta, em português, antes de ir à rede; nulo = parece certo.</summary>
    public string? Problem() =>
        string.IsNullOrWhiteSpace(User) ? "Escreva ou cole o seu usuário do ScreenScraper (passo 2)."
        : string.IsNullOrEmpty(Password) ? "Escreva ou cole a sua senha do ScreenScraper (passo 2)."
        : null;

    /// <summary>Nunca mostra a senha (logs, depuração).</summary>
    public override string ToString() => $"ScreenScraperSettings {{ Enabled = {Enabled}, User = {User}, Password = {(Password.Length == 0 ? "vazia" : "***")} }}";
}

/// <summary>
/// Credenciais do software (devid, devpassword, softname) que o ScreenScraper dá aos mantenedores. NUNCA ficam no
/// repositório: o build de release as recebe de segredos do CI (<c>SCREENSCRAPER_DEVID</c>, <c>SCREENSCRAPER_DEVPASSWORD</c>,
/// <c>SCREENSCRAPER_SOFTNAME</c>) e grava em atributos <see cref="AssemblyMetadataAttribute"/> do Tako.exe. Sem elas
/// (compilação local, CI de PR), o ScreenScraper fica indisponível com uma mensagem clara e nada quebra.
/// </summary>
public sealed record ScreenScraperSoftware(string DevId, string DevPassword, string SoftName)
{
    public const string DevIdKey = "ScreenScraperDevId";
    public const string DevPasswordKey = "ScreenScraperDevPassword";
    public const string SoftNameKey = "ScreenScraperSoftName";

    public static readonly ScreenScraperSoftware Missing = new(string.Empty, string.Empty, "Tako");

    public bool IsAvailable => DevId.Length > 0 && DevPassword.Length > 0;

    /// <summary>Lê os atributos gravados no build de release; ausentes = <see cref="Missing"/>.</summary>
    public static ScreenScraperSoftware FromAssembly(Assembly? assembly)
    {
        if (assembly is null)
        {
            return Missing;
        }

        var values = assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
            .GroupBy(a => a.Key, StringComparer.Ordinal)
            .ToDictionary(g => g.Key, g => g.Last().Value ?? string.Empty, StringComparer.Ordinal);
        return FromValues(values.GetValueOrDefault(DevIdKey), values.GetValueOrDefault(DevPasswordKey), values.GetValueOrDefault(SoftNameKey));
    }

    public static ScreenScraperSoftware FromValues(string? devId, string? devPassword, string? softName) =>
        new((devId ?? string.Empty).Trim(), (devPassword ?? string.Empty).Trim(), string.IsNullOrWhiteSpace(softName) ? Missing.SoftName : softName.Trim());

    public override string ToString() => $"ScreenScraperSoftware {{ DevId = {(DevId.Length == 0 ? "ausente" : DevId)}, DevPassword = {(DevPassword.Length == 0 ? "ausente" : "***")}, SoftName = {SoftName} }}";
}

/// <summary>
/// Guarda <see cref="ScreenScraperSettings"/> num arquivo da pasta de dados, inteiro protegido (<see cref="ISecretProtector"/>,
/// DPAPI no Windows), como a chave da IGDB. Arquivo ilegível conta como ScreenScraper desligado.
/// </summary>
public sealed class ScreenScraperSettingsStore(string path, ISecretProtector protector)
{
    private readonly object _gate = new();
    private ScreenScraperSettings? _current;

    public event EventHandler? Changed;

    public ScreenScraperSettings Current
    {
        get
        {
            lock (_gate)
            {
                return _current ??= Load();
            }
        }
    }

    public void Save(ScreenScraperSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var clean = settings with { User = settings.User.Trim() };
        lock (_gate)
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(path))!);
            File.WriteAllText(path, protector.Protect(JsonSerializer.Serialize(new Stored(clean.Enabled, clean.User, clean.Password))));
            _current = clean;
        }

        Changed?.Invoke(this, EventArgs.Empty);
    }

    private ScreenScraperSettings Load()
    {
        try
        {
            if (File.Exists(path) && protector.TryUnprotect(File.ReadAllText(path).Trim(), out string json) && json.Length > 0
                && JsonSerializer.Deserialize<Stored>(json) is { } stored)
            {
                return new ScreenScraperSettings(stored.Enabled, stored.User ?? string.Empty, stored.Password ?? string.Empty);
            }
        }
        catch (Exception ex) when (ex is JsonException or IOException or UnauthorizedAccessException)
        {
            // Ilegível: o usuário informa a conta de novo.
        }

        return ScreenScraperSettings.Off;
    }

    private sealed record Stored(bool Enabled, string? User, string? Password);
}
