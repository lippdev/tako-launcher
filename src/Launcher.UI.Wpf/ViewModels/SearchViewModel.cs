using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Presentation;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Tecla do teclado virtual na tela (largura em unidades do palco).</summary>
public sealed record VirtualKeyViewModel(VirtualKey Key, double Width, ICommand Command)
{
    public string Label => Key.Label;

    public string NavKey => $"key:{Key.Label}";

    public bool IsDefault => Key.Label == "Q";

    public bool IsAction => Key.Kind != VirtualKeyKind.Character;

    public bool IsDone => Key.Kind == VirtualKeyKind.Done;
}

/// <summary>
/// Busca com teclado virtual (UX-001). O resultado aparece ao vivo atrás do painel: cada tecla chama
/// <c>apply</c>, que filtra a lista (biblioteca do Tako ou a grade do tema Playnite) e devolve quantos jogos sobraram.
/// O teclado físico digita direto (a janela encaminha o texto); o controle usa a grade de teclas, X apaga, Y põe espaço e
/// ☰ conclui. B fecha mantendo a busca.
/// </summary>
public sealed partial class SearchViewModel : ObservableObject, IOverlayViewModel
{
    /// <summary>Largura de uma coluna do teclado e o espaço entre teclas, no palco de 1080.</summary>
    private const double KeyUnit = 60;
    private const double KeyGap = 10;

    private readonly Func<string, int> _apply;
    private readonly Action _done;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasText), nameof(ResultText))]
    private string _text;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(ResultText))]
    private int _resultCount;

    /// <param name="text">Busca atual (reabrir continua de onde parou).</param>
    /// <param name="apply">Aplica o texto e devolve quantos jogos aparecem.</param>
    /// <param name="done">Pronto/Enter: a Shell fecha o teclado e leva o foco ao primeiro resultado.</param>
    public SearchViewModel(string text, Func<string, int> apply, Action done)
    {
        _text = text ?? string.Empty;
        _apply = apply;
        _done = done;
        _resultCount = apply(_text);
        Rows = [.. VirtualKeyboard.Rows.Select(row => (IReadOnlyList<VirtualKeyViewModel>)[.. row.Select(Key)])];
    }

    public IReadOnlyList<IReadOnlyList<VirtualKeyViewModel>> Rows { get; }

    public bool HasText => Text.Length > 0;

    public string ResultText => _describe?.Invoke(Text)
        ?? (HasText ? LibraryText.GameCount(ResultCount) : "Digite com o teclado ou escolha as letras com o controle");

    /// <summary>Teclado físico: texto digitado (letras, números, espaço).</summary>
    public void Type(string typed) => Text = VirtualKeyboard.Type(Text, typed);

    /// <summary>X do controle ou Backspace.</summary>
    public void Backspace() => Text = VirtualKeyboard.Backspace(Text);

    /// <summary>Y do controle.</summary>
    public void Space() => Text = VirtualKeyboard.Type(Text, " ");

    /// <summary>Pronto, ☰ ou Enter: fecha o teclado com os resultados na tela.</summary>
    public void Finish() => _done();

    partial void OnTextChanged(string value) => ResultCount = _apply(value);

    private VirtualKeyViewModel Key(VirtualKey key) =>
        new(key, (key.Width * KeyUnit) + ((key.Width - 1) * KeyGap), new RelayCommand(() => Press(key)));

    private void Press(VirtualKey key)
    {
        if (key.Kind == VirtualKeyKind.Done)
        {
            Finish();
            return;
        }

        Text = VirtualKeyboard.Apply(Text, key);
    }
}
