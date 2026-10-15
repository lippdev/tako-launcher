namespace Launcher.Core.Library;

/// <summary>Biblioteca local de jogos (MVP-023). No Marco 0 é a biblioteca fictícia em memória; persistência é o MVP-005.</summary>
public interface IGameLibrary
{
    event EventHandler? Changed;

    IReadOnlyList<Game> Games { get; }

    Game? Find(string id);

    void ToggleFavorite(string id);

    /// <summary>Registra o fim de uma sessão: o jogo vai para o topo de "Jogados recentemente".</summary>
    void RecordSession(string id, TimeSpan duration);
}
