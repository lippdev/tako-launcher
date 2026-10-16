using System.Windows.Input;
using System.Windows.Media;
using CommunityToolkit.Mvvm.ComponentModel;
using Launcher.Core.Input;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Sobreposição que trata botões além de A/B (LB/RB, X, Y, ☰): o seletor de imagens e "Fontes e IDs" (ADR-0023).
/// Devolve verdadeiro quando tratou; senão, a janela segue o padrão.
/// </summary>
public interface IOverlayInput
{
    bool HandleCommand(AppCommand command);
}

/// <summary>Sobreposição com dicas próprias no rodapé (botões, rótulo, comando), relidas a cada mudança de foco.</summary>
public interface IOverlayHints
{
    IReadOnlyList<(string Buttons, string Label, AppCommand Command)> Hints { get; }
}

/// <summary>Aba ou chip de um modal (tipo de imagem, fonte, filtro): rótulo, se está escolhido e o que faz.</summary>
public sealed partial class ModalTab(string key, string label, ICommand command) : ObservableObject, INavKeyed
{
    [ObservableProperty]
    private bool _isSelected;

    [ObservableProperty]
    private string _label = label;

    public string Key { get; } = key;

    public ICommand Command { get; } = command;

    string INavKeyed.NavKey => Key;
}

/// <summary>
/// Miniatura carregada só quando aparece (como as capas da grade, PERF-004): enquanto carrega, fica nula e a tela mostra
/// o marcador; o <see cref="ArtService"/> decodifica no tamanho exibido e guarda o arquivo no cache.
/// </summary>
public abstract partial class LazyThumb : ObservableObject
{
    private readonly ArtService _art;
    private readonly Uri? _uri;
    private readonly int _width;
    private ImageSource? _image;
    private bool _requested;

    protected LazyThumb(ArtService art, Uri? uri, int width)
    {
        _art = art;
        _uri = uri;
        _width = width;
    }

    public ImageSource? Thumb
    {
        get
        {
            if (!_requested && _uri is not null)
            {
                _requested = true;
                _ = LoadAsync();
            }

            return _image;
        }
    }

    /// <summary>Ainda carregando (marcador na tela).</summary>
    public bool IsLoading => _image is null && _uri is not null && !Failed;

    public bool Failed { get; private set; }

    private async Task LoadAsync()
    {
        await Task.Yield(); // nunca avisa no meio da leitura que pediu a miniatura
        _image = await _art.GetAsync(_uri, _width).ConfigureAwait(true);
        Failed = _image is null;
        OnPropertyChanged(nameof(Thumb));
        OnPropertyChanged(nameof(IsLoading));
        OnPropertyChanged(nameof(Failed));
    }
}
