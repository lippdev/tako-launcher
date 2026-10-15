using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Navigation;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Como uma linha de configuração se mostra e reage ao A (ADR-0020).</summary>
public enum SettingKind
{
    /// <summary>Abre algo (diálogo, página, pasta): valor e seta à direita.</summary>
    Action,

    /// <summary>Liga e desliga: chave à direita.</summary>
    Toggle,

    /// <summary>Troca de valor a cada A: valor e setas ‹ › à direita.</summary>
    Choice,

    /// <summary>Só informa; A pode explicar mais num aviso.</summary>
    Info,

    /// <summary>Título de um grupo de linhas (não recebe foco).</summary>
    Header,
}

/// <summary>
/// Uma linha das Configurações ou do detalhe de um recurso do Modo Console. Título, descrição e valor são lidos do
/// estado real a cada <see cref="Refresh"/> (a tela nunca guarda uma cópia que possa ficar velha).
/// </summary>
public sealed partial class SettingRowViewModel : ObservableObject, INavKeyed
{
    private readonly Func<string?>? _readDescription;
    private readonly Func<string?>? _readValue;
    private readonly Func<bool>? _readIsOn;
    private readonly Func<bool>? _readIsVisible;
    private readonly Func<bool>? _readIsDimmed;
    private readonly Func<FeatureState?>? _readState;

    /// <summary>Descrição atual (até duas linhas, em linguagem de usuário).</summary>
    [ObservableProperty]
    private string? _description;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasValue))]
    private string? _valueText;

    [ObservableProperty]
    private bool _isOn;

    [ObservableProperty]
    private bool _isShown = true;

    /// <summary>Esmaecida, mas focável para explicar (ex.: precisa de teclado e mouse, ADR-0017).</summary>
    [ObservableProperty]
    private bool _isDimmed;

    /// <summary>Selo de estado (cartões e linhas de status do Modo Console).</summary>
    [ObservableProperty]
    private FeatureState? _state;

    public SettingRowViewModel(
        string key,
        SettingKind kind,
        string title,
        ICommand? command = null,
        Func<string?>? description = null,
        Func<string?>? value = null,
        Func<bool>? isOn = null,
        Func<bool>? visible = null,
        Func<bool>? dimmed = null,
        Func<FeatureState?>? state = null,
        string? icon = null)
    {
        Key = key;
        Kind = kind;
        Title = title;
        Command = command;
        IconKey = icon;
        _readDescription = description;
        _readValue = value;
        _readIsOn = isOn;
        _readIsVisible = visible;
        _readIsDimmed = dimmed;
        _readState = state;
        Refresh();
    }

    /// <summary>Chave de foco (<c>Nav.Key</c>): estável entre versões; os testes e "voltar" usam ela.</summary>
    public string Key { get; }

    public SettingKind Kind { get; }

    public string Title { get; }

    public string? IconKey { get; }

    public ICommand? Command { get; }

    public bool IsHeader => Kind == SettingKind.Header;

    /// <summary>Primeira linha da lista: → (ou ↓) vindo da coluna de categorias entra por ela (<c>Nav.IsGroupEntry</c>).</summary>
    public bool IsEntry { get; set; }

    public bool HasValue => !string.IsNullOrEmpty(ValueText);

    string INavKeyed.NavKey => Key;

    /// <summary>Relê o estado e avisa a tela só do que mudou.</summary>
    public void Refresh()
    {
        Description = _readDescription?.Invoke();
        ValueText = _readValue?.Invoke();
        IsOn = _readIsOn?.Invoke() ?? false;
        IsShown = _readIsVisible?.Invoke() ?? true;
        IsDimmed = _readIsDimmed?.Invoke() ?? false;
        State = _readState?.Invoke();
    }
}

/// <summary>Uma categoria da coluna da esquerda das Configurações (Geral, Biblioteca e lojas, Aparência...).</summary>
public sealed partial class SettingsCategoryViewModel(string id, string title, string iconKey, string description, IReadOnlyList<SettingRowViewModel> rows) : ObservableObject, INavKeyed
{
    [ObservableProperty]
    private bool _isSelected;

    public string Id { get; } = id;

    public string Title { get; } = title;

    public string IconKey { get; } = iconKey;

    /// <summary>Uma frase no topo da coluna de conteúdo.</summary>
    public string Description { get; } = description;

    public IReadOnlyList<SettingRowViewModel> Rows { get; } = rows;

    public string NavKey => $"settings-cat:{Id}";

    string INavKeyed.NavKey => NavKey;
}
