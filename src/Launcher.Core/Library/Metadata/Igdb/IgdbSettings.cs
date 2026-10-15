using System.Text.Json;
using ConsoleMode.Runtime.Tv;

namespace Launcher.Core.Library.Metadata.Igdb;

/// <summary>
/// IGDB ligada ou não e a chave da Twitch do próprio usuário (ADR-0021). O Tako é open source: nenhum segredo vai no
/// repositório nem no executável; cada pessoa cria o seu app em dev.twitch.tv.
/// </summary>
public sealed record IgdbSettings(bool Enabled, string ClientId, string ClientSecret)
{
    public static readonly IgdbSettings Off = new(false, string.Empty, string.Empty);

    public bool HasKey => !string.IsNullOrWhiteSpace(ClientId) && !string.IsNullOrWhiteSpace(ClientSecret);

    /// <summary>Ligada e com chave: só então o provedor consulta a IGDB.</summary>
    public bool IsActive => Enabled && HasKey;

    /// <summary>Texto colado costuma trazer espaços e quebras de linha: saem todos.</summary>
    public static string Clean(string? value) => string.Concat((value ?? string.Empty).Where(c => !char.IsWhiteSpace(c)));

    /// <summary>
    /// O que falta ou parece errado na chave, em português, antes de ir à rede; nulo = parece certo. Client ID e Client
    /// Secret da Twitch têm 30 letras minúsculas e números; aceita uma folga e avisa colagem pela metade.
    /// </summary>
    public string? Problem()
    {
        if (ClientId.Length == 0)
        {
            return "Cole o Client ID do seu app da Twitch (passo 3).";
        }

        if (ClientSecret.Length == 0)
        {
            return "Cole o Client Secret do seu app da Twitch (passo 4).";
        }

        if (!LooksLikeKey(ClientId))
        {
            return "O Client ID parece incompleto: ele tem cerca de 30 letras e números. Copie de novo em \"Gerenciar\".";
        }

        return LooksLikeKey(ClientSecret) ? null : "O Client Secret parece incompleto: ele tem cerca de 30 letras e números. Gere um novo e cole de novo.";
    }

    /// <summary>Nunca mostra o segredo (logs, depuração).</summary>
    public override string ToString() => $"IgdbSettings {{ Enabled = {Enabled}, ClientId = {ClientId}, ClientSecret = {(ClientSecret.Length == 0 ? "vazio" : "***")} }}";

    private static bool LooksLikeKey(string value) => value.Length is >= 20 and <= 64 && value.All(char.IsAsciiLetterOrDigit);
}

/// <summary>
/// Guarda <see cref="IgdbSettings"/> num arquivo da pasta de dados, inteiro protegido (<see cref="ISecretProtector"/>,
/// DPAPI no Windows), como as chaves das TVs. Arquivo ilegível (outro usuário/PC, corrompido) conta como IGDB desligada.
/// </summary>
public sealed class IgdbSettingsStore(string path, ISecretProtector protector)
{
    private readonly object _gate = new();
    private IgdbSettings? _current;

    public event EventHandler? Changed;

    public IgdbSettings Current
    {
        get
        {
            lock (_gate)
            {
                return _current ??= Load();
            }
        }
    }

    public void Save(IgdbSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var clean = settings with { ClientId = settings.ClientId.Trim(), ClientSecret = settings.ClientSecret.Trim() };
        lock (_gate)
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(path))!);
            File.WriteAllText(path, protector.Protect(JsonSerializer.Serialize(new Stored(clean.Enabled, clean.ClientId, clean.ClientSecret))));
            _current = clean;
        }

        Changed?.Invoke(this, EventArgs.Empty);
    }

    private IgdbSettings Load()
    {
        try
        {
            if (File.Exists(path) && protector.TryUnprotect(File.ReadAllText(path).Trim(), out string json) && json.Length > 0
                && JsonSerializer.Deserialize<Stored>(json) is { } stored)
            {
                return new IgdbSettings(stored.Enabled, stored.ClientId ?? string.Empty, stored.ClientSecret ?? string.Empty);
            }
        }
        catch (Exception ex) when (ex is JsonException or IOException or UnauthorizedAccessException)
        {
            // Ilegível: o usuário informa a chave de novo.
        }

        return IgdbSettings.Off;
    }

    private sealed record Stored(bool Enabled, string? ClientId, string? ClientSecret);
}
