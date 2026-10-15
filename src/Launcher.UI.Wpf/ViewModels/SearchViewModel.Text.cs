namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// O mesmo teclado virtual para digitar outros textos (UX-003/007): nome de coleção, argumentos extras de um jogo. O
/// título e a dica mudam; abaixo do campo aparece a regra do texto em vez da contagem de jogos. B fecha sem salvar.
/// </summary>
public sealed partial class SearchViewModel
{
    private Func<string, string>? _describe;

    public string Title { get; private init; } = "Buscar";

    public string Placeholder { get; private init; } = "Nome do jogo";

    /// <summary>Teclado para digitar um texto. <paramref name="done"/> recebe o texto em Pronto/☰/Enter.</summary>
    /// <param name="describe">Linha abaixo do campo para o texto atual (regra, erro ou como concluir).</param>
    public static SearchViewModel ForText(string title, string placeholder, string text, Func<string, string> describe, Action<string> done)
    {
        ArgumentNullException.ThrowIfNull(done);
        SearchViewModel? entry = null;
        entry = new SearchViewModel(text, _ => 0, () => done(entry!.Text))
        {
            Title = title,
            Placeholder = placeholder,
            _describe = describe,
        };
        return entry;
    }
}
