namespace Launcher.Core.Presentation;

/// <summary>Como a barra de dicas cabe na largura: <see cref="Compact"/> usa os textos curtos; <see cref="Hidden"/> some.</summary>
public sealed record HintLayout(bool Compact, IReadOnlySet<int> Hidden)
{
    public static HintLayout Full { get; } = new(false, new HashSet<int>());
}

/// <summary>
/// Barra de dicas que se adapta à largura (ADR-0020): primeiro abrevia (textos curtos e menos espaço entre as dicas),
/// depois esconde as dicas menos importantes. Nunca corta uma dica no meio, em nenhuma proporção de tela.
/// </summary>
public static class HintFit
{
    /// <param name="full">Largura de cada dica com o texto inteiro (inclui o espaço antes dela).</param>
    /// <param name="compact">Largura de cada dica abreviada.</param>
    /// <param name="priority">Importância de cada dica: a de menor valor some primeiro; no empate, a mais à direita.</param>
    /// <param name="available">Largura disponível na barra.</param>
    public static HintLayout Plan(IReadOnlyList<double> full, IReadOnlyList<double> compact, IReadOnlyList<int> priority, double available)
    {
        ArgumentNullException.ThrowIfNull(full);
        ArgumentNullException.ThrowIfNull(compact);
        ArgumentNullException.ThrowIfNull(priority);
        if (full.Count != compact.Count || full.Count != priority.Count)
        {
            throw new ArgumentException("As listas de larguras e prioridades precisam ter o mesmo tamanho.", nameof(priority));
        }

        if (full.Sum() <= available)
        {
            return HintLayout.Full;
        }

        var hidden = new HashSet<int>();
        double total = compact.Sum();
        var order = Enumerable.Range(0, full.Count).OrderBy(i => priority[i]).ThenByDescending(i => i).ToList();

        // A dica mais importante fica sempre, mesmo que sozinha não caiba.
        foreach (int index in order.Take(Math.Max(0, order.Count - 1)))
        {
            if (total <= available)
            {
                break;
            }

            hidden.Add(index);
            total -= compact[index];
        }

        return new HintLayout(true, hidden);
    }
}
