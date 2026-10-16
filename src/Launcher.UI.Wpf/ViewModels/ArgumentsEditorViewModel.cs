using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Emulation;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Botão de um marcador no editor ("{rom}" com a explicação): A acrescenta o marcador ao fim do texto.</summary>
public sealed record PlaceholderViewModel(string Token, string Label, System.Windows.Input.ICommand Command)
{
    /// <summary>Chave de foco ("args:{rom}").</summary>
    public string NavKey => "args:" + Token;
}

/// <summary>
/// Editor de argumentos de emulador, de sistema ou de jogo (EMU-004, ADR-0024), feito para o controle: os marcadores
/// entram com A (sem digitar chaves), "Colar" traz um texto pronto, "Padrão" volta ao da tabela e a prévia mostra a
/// linha de comando com uma ROM de exemplo. O teclado continua funcionando no campo.
/// </summary>
public sealed partial class ArgumentsEditorViewModel : ObservableObject, IOverlayViewModel
{
    private readonly Func<string?> _readClipboard;
    private readonly Action<string?> _save;
    private readonly Action _cancel;
    private readonly Func<string, string> _preview;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Preview))]
    private string _text;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasError))]
    private string? _error;

    /// <param name="defaultText">O que "Padrão" põe de volta (os argumentos da tabela; vazio = nenhum).</param>
    /// <param name="preview">Linha de comando de exemplo para o texto atual.</param>
    /// <param name="save">Recebe o texto, ou nulo para voltar ao padrão.</param>
    public ArgumentsEditorViewModel(string title, string description, string current, string defaultText, Func<string, string> preview, Func<string?> readClipboard, Action<string?> save, Action cancel)
    {
        Title = title;
        Description = description;
        DefaultText = defaultText;
        _text = current;
        _preview = preview;
        _readClipboard = readClipboard;
        _save = save;
        _cancel = cancel;
        Placeholders =
        [
            Token(LaunchArguments.Rom, "arquivo do jogo"),
            Token(LaunchArguments.FullscreenToken, "tela cheia"),
            Token(LaunchArguments.RomDir, "pasta do jogo"),
            Token(LaunchArguments.RomName, "nome sem extensão"),
            Token(LaunchArguments.Core, "core do RetroArch"),
            Token(LaunchArguments.Config, ".cfg do jogo"),
        ];
    }

    public string Title { get; }

    public string Description { get; }

    public string DefaultText { get; }

    public IReadOnlyList<PlaceholderViewModel> Placeholders { get; }

    public bool HasError => Error is not null;

    /// <summary>"Exemplo: retroarch.exe -L "…snes9x_libretro.dll" "…\Jogo.sfc" -f".</summary>
    public string Preview => "Exemplo: " + _preview(Text);

    [RelayCommand]
    private void Paste()
    {
        if (_readClipboard() is { Length: > 0 } text)
        {
            Text = text.Replace('\r', ' ').Replace('\n', ' ').Trim();
            Error = null;
        }
        else
        {
            Error = "A área de transferência não tem texto.";
        }
    }

    [RelayCommand]
    private void Reset()
    {
        Text = DefaultText;
        Error = null;
    }

    [RelayCommand]
    private void Clear()
    {
        Text = string.Empty;
        Error = null;
    }

    [RelayCommand]
    private void Save()
    {
        string text = Text.Trim();
        if (LaunchArguments.Unknown(text) is { Count: > 0 } unknown)
        {
            Error = $"Marcador desconhecido: {string.Join(", ", unknown)}. Use os botões acima.";
            return;
        }

        _save(text == DefaultText.Trim() ? null : text);
    }

    [RelayCommand]
    private void Cancel() => _cancel();

    private PlaceholderViewModel Token(string token, string label) =>
        new(token, $"{token} · {label}", new RelayCommand(() =>
        {
            Text = Text.Length == 0 ? token : Text.TrimEnd() + " " + token;
            Error = null;
        }));
}
