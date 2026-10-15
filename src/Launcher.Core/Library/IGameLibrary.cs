namespace Launcher.Core.Library;

/// <summary>Biblioteca local de jogos (MVP-023): jogos encontrados nas lojas + adicionados pelo usuário.</summary>
public interface IGameLibrary
{
    event EventHandler? Changed;

    IReadOnlyList<Game> Games { get; }

    Game? Find(string id);

    ManualGameRecord? FindManual(string id);

    void ToggleFavorite(string id);

    /// <summary>Registra o fim de uma sessão: o jogo vai para o topo de "Jogados recentemente".</summary>
    void RecordSession(string id, TimeSpan duration);

    /// <summary>O jogo foi aberto pelo launcher agora (a duração real chega com o acompanhamento de processos, MVP-026).</summary>
    void RecordLaunch(string id);

    Game AddManual(ManualGameRecord record);

    void UpdateManual(ManualGameRecord record);

    void RemoveManual(string id);

    /// <summary>Procura jogos instalados de novo.</summary>
    void Refresh();
}
