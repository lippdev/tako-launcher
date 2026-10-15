using System.ComponentModel;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Página feita de linhas de configuração (Configurações, Modo Console e o detalhe de cada recurso). Enquanto está na
/// tela, relê as linhas quando as preferências, a Shell ou as ferramentas da sessão mudam; fora da tela, não escuta
/// nada (as páginas do histórico não ficam presas aos serviços).
/// </summary>
public abstract class RowsPageViewModel(ShellViewModel shell) : PageViewModel(shell)
{
    private bool _listening;

    /// <summary>Todas as linhas da página (inclusive as escondidas e as de outras categorias).</summary>
    protected abstract IEnumerable<SettingRowViewModel> AllRows { get; }

    public override void OnShown()
    {
        if (!_listening)
        {
            _listening = true;
            Shell.PropertyChanged += OnStateChanged;
            Shell.Settings.PropertyChanged += OnStateChanged;
            Shell.SettingsChanged += OnStateChanged;
        }

        Refresh();
    }

    public override void OnHidden()
    {
        if (_listening)
        {
            _listening = false;
            Shell.PropertyChanged -= OnStateChanged;
            Shell.Settings.PropertyChanged -= OnStateChanged;
            Shell.SettingsChanged -= OnStateChanged;
        }
    }

    public override void Refresh()
    {
        foreach (var row in AllRows)
        {
            row.Refresh();
        }
    }

    private void OnStateChanged(object? sender, EventArgs e)
    {
        // O relógio e o foco mudam a Shell o tempo todo; só o que pode mudar uma linha precisa reler.
        if (e is PropertyChangedEventArgs { PropertyName: nameof(ShellViewModel.Clock) or nameof(ShellViewModel.IsSidebarExpanded) or nameof(ShellViewModel.Backdrop) or nameof(ShellViewModel.ToastMessage) or nameof(ShellViewModel.IsToastVisible) })
        {
            return;
        }

        Refresh();
    }
}
