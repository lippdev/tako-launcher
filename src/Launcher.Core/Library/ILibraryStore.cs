namespace Launcher.Core.Library;

/// <summary>
/// Onde a biblioteca do usuário fica guardada: jogos adicionados e estado por jogo. O app usa
/// <see cref="Persistence.SqliteLibraryStore"/>; <see cref="LibraryStore"/> (JSON) é o formato antigo, só importado.
/// </summary>
public interface ILibraryStore
{
    List<ManualGameRecord> Manual { get; }

    Dictionary<string, GameStateRecord> State { get; }

    /// <summary>Carrega tudo; dado corrompido não lança exceção (recomeça vazio e guarda uma cópia).</summary>
    void Load();

    /// <summary>Grava <see cref="Manual"/> e <see cref="State"/> de forma atômica.</summary>
    void Save();
}
