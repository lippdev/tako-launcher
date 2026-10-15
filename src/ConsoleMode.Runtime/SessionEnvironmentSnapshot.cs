using System.Text.Json;
using System.Text.Json.Serialization;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;

namespace ConsoleMode.Runtime;

/// <summary>Falha da plataforma ao ler ou alterar monitores ou áudio. Nunca deve derrubar a sessão.</summary>
public sealed class RuntimeEnvironmentException : Exception
{
    public RuntimeEnvironmentException()
    {
    }

    public RuntimeEnvironmentException(string message)
        : base(message)
    {
    }

    public RuntimeEnvironmentException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}

/// <summary>
/// Estado de monitores e áudio registrado antes de uma sessão (ex.: ao abrir um jogo), para restaurar depois.
/// <see cref="Display"/> ou <see cref="Audio"/> ficam nulos quando a leitura daquela parte falhou: a outra parte
/// continua valendo.
/// </summary>
public sealed record SessionEnvironmentSnapshot(
    int SchemaVersion,
    DateTimeOffset CapturedAt,
    string? Context,
    DisplayTopology? Display,
    AudioState? Audio)
{
    public const int CurrentSchemaVersion = 1;

    public static SessionEnvironmentSnapshot Create(DateTimeOffset capturedAt, string? context, DisplayTopology? display, AudioState? audio) =>
        new(CurrentSchemaVersion, capturedAt, context, display, audio);

    /// <summary>Monitores e áudio iguais (as partes ausentes em qualquer um dos lados não são comparadas).</summary>
    public bool IsEquivalentTo(SessionEnvironmentSnapshot other)
    {
        ArgumentNullException.ThrowIfNull(other);
        bool displaySame = Display is null || other.Display is null || Display.IsEquivalentTo(other.Display);
        bool audioSame = Audio is null || other.Audio is null || Audio.IsEquivalentTo(other.Audio);
        return displaySame && audioSame;
    }
}

/// <summary>Formato JSON do snapshot (arquivo em disco e, no futuro, IPC). Versões desconhecidas são recusadas.</summary>
public static class SessionEnvironmentSnapshotJson
{
    private static readonly JsonSerializerOptions Options = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        Converters = { new JsonStringEnumConverter() },
    };

    public static string Serialize(SessionEnvironmentSnapshot snapshot)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        return JsonSerializer.Serialize(snapshot, Options);
    }

    /// <summary>Lê um snapshot; texto inválido, versão desconhecida ou dados incompletos devolvem nulo.</summary>
    public static SessionEnvironmentSnapshot? TryDeserialize(string json)
    {
        try
        {
            var snapshot = JsonSerializer.Deserialize<SessionEnvironmentSnapshot>(json, Options);
            return snapshot is { SchemaVersion: SessionEnvironmentSnapshot.CurrentSchemaVersion } && IsComplete(snapshot) ? snapshot : null;
        }
        catch (JsonException)
        {
            return null;
        }
    }

    // O desserializador aceita listas e textos ausentes como nulos; um arquivo assim não serve para restaurar.
    private static bool IsComplete(SessionEnvironmentSnapshot snapshot) =>
        (snapshot.Display is null || (snapshot.Display.Monitors is not null && snapshot.Display.Monitors.All(m => m is { Id: not null, Name: not null })))
        && (snapshot.Audio is null || (snapshot.Audio.Devices is not null && snapshot.Audio.Defaults is not null && snapshot.Audio.Levels is not null
            && snapshot.Audio.Defaults.All(d => d is { DeviceId: not null }) && snapshot.Audio.Levels.All(l => l is { DeviceId: not null })));
}
