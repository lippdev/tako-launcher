using System.Text.Json;

namespace Launcher.Core.Library;

/// <summary>
/// Biblioteca do usuário em JSON (<c>%LOCALAPPDATA%\Tako\library.json</c>): jogos adicionados e estado por jogo.
/// Gravação atômica (arquivo temporário + troca). SQLite com migrações continua planejado (MVP-005).
/// </summary>
public sealed class LibraryStore(string path)
{
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    public string FilePath { get; } = path;

    public List<ManualGameRecord> Manual { get; private set; } = [];

    public Dictionary<string, GameStateRecord> State { get; private set; } = [];

    public void Load()
    {
        if (!File.Exists(FilePath))
        {
            return;
        }

        try
        {
            using var stream = File.OpenRead(FilePath);
            var document = JsonSerializer.Deserialize<Document>(stream, Options);
            Manual = document?.Manual ?? [];
            State = document?.State ?? [];
        }
        catch (JsonException)
        {
            // Arquivo corrompido: guarda uma cópia e começa vazio, sem derrubar o launcher.
            File.Copy(FilePath, FilePath + ".corrompido", overwrite: true);
            Manual = [];
            State = [];
        }
    }

    public void Save()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(FilePath)!);
        string temp = FilePath + ".tmp";
        using (var stream = File.Create(temp))
        {
            JsonSerializer.Serialize(stream, new Document(1, Manual, State), Options);
        }

        File.Move(temp, FilePath, overwrite: true);
    }

    private sealed record Document(int Version, List<ManualGameRecord>? Manual, Dictionary<string, GameStateRecord>? State);
}
