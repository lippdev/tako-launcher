namespace Launcher.UI.Wpf.Navigation;

/// <summary>Item de lista com a mesma chave que o elemento dele recebe em <c>Nav.Key</c> (ex.: o Id do jogo).</summary>
public interface INavKeyed
{
    string NavKey { get; }
}

/// <summary>
/// Lista virtualizada (PERF-004): só os itens perto da tela existem como elementos, então o
/// <see cref="FocusNavigator"/> não acha pela árvore visual um item fora dela (ex.: o jogo focado antes de abrir os
/// detalhes, lá embaixo na grade). Pedir a chave cria o elemento do item, se a lista tiver esse item.
/// </summary>
public interface IVirtualNavHost
{
    /// <summary>Cria (no próximo layout) o elemento do item com a chave pedida (<c>grupo:chave</c> ou só a chave).</summary>
    bool RealizeKey(string key);
}

/// <summary>
/// Página com partes escondidas (ex.: categorias das Configurações): pedir a chave de um elemento escondido mostra a
/// parte dele. Ligada à navegação pelo <see cref="Controls.RevealHost"/>.
/// </summary>
public interface IKeyRevealer
{
    /// <summary>Mostra a parte com a chave pedida; verdadeiro se algo mudou e a tela precisa de um novo layout.</summary>
    bool RevealKey(string key);
}
