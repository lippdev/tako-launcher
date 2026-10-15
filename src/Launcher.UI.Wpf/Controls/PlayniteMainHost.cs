using System.ComponentModel;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Launcher.PlayniteCompat.P10.Controls;
using Launcher.PlayniteCompat.P10.Data;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.ViewModels;

namespace Launcher.UI.Wpf.Controls;

/// <summary>
/// Hospeda a tela principal de um tema Playnite (<see cref="Main"/>) na janela do Tako (MVP-018/019/021): recursos do
/// tema só aqui dentro (<see cref="ThemeScope"/>), a API do adaptador herdada pela árvore, os elementos <c>PART_</c>
/// ligados à navegação por controle/teclado e uma rede de segurança no layout: um erro do template do tema não derruba
/// o launcher, volta à UI padrão com a tela de recuperação.
/// </summary>
public sealed class PlayniteMainHost : Decorator
{
    public static readonly DependencyProperty SessionProperty =
        DependencyProperty.Register(nameof(Session), typeof(PlayniteUiViewModel), typeof(PlayniteMainHost), new PropertyMetadata(null, (d, e) => ((PlayniteMainHost)d).Build(e.NewValue as PlayniteUiViewModel)));

    private readonly List<WeakReference<FrameworkElement>> _games = [];
    private P10MainModel? _model;
    private bool _failed;

    public PlayniteMainHost()
    {
        Focusable = false;
        AddHandler(P10Navigation.NavigableEvent, new EventHandler<P10NavigableEventArgs>(OnNavigable));
    }

    /// <summary>Tema em uso; nulo esvazia o host.</summary>
    public PlayniteUiViewModel? Session
    {
        get => (PlayniteUiViewModel?)GetValue(SessionProperty);
        set => SetValue(SessionProperty, value);
    }

    protected override Size MeasureOverride(Size constraint)
    {
        if (_failed)
        {
            return default;
        }

        try
        {
            return base.MeasureOverride(constraint);
        }
        catch (Exception ex) when (ex is not (OutOfMemoryException or StackOverflowException))
        {
            Fail(ex);
            return default;
        }
    }

    protected override Size ArrangeOverride(Size arrangeSize)
    {
        if (_failed)
        {
            return arrangeSize;
        }

        try
        {
            return base.ArrangeOverride(arrangeSize);
        }
        catch (Exception ex) when (ex is not (OutOfMemoryException or StackOverflowException))
        {
            Fail(ex);
            return arrangeSize;
        }
    }

    private void Build(PlayniteUiViewModel? session)
    {
        if (_model is not null)
        {
            _model.PropertyChanged -= OnModelChanged;
        }

        _games.Clear();
        _failed = false;
        _model = session?.Model;
        if (session is null)
        {
            Child = null;
            return;
        }

        var scope = new ThemeScope { ThemeResources = session.Resources };
        P10Context.SetApi(scope, session.Api);
        scope.Content = new Main { DataContext = session.Model };
        session.Model.PropertyChanged += OnModelChanged;
        Child = scope;
    }

    /// <summary>Elemento do tema anunciado pelo adaptador → propriedades de navegação do Tako.</summary>
    private void OnNavigable(object? sender, P10NavigableEventArgs e)
    {
        e.Handled = true;
        var element = e.Element;
        if (e.IsModalScope)
        {
            Nav.SetIsModalScope(element, true);
            return;
        }

        Nav.SetGroup(element, e.Group);
        Nav.SetKey(element, e.Key);
        Nav.SetIsDefault(element, e.IsDefault);
        if (e.Activate is not null)
        {
            Nav.SetActivateCommand(element, e.Activate);
            Nav.SetActivateParameter(element, e.ActivateParameter);
        }

        if (e.Group == Main.GamesGroup)
        {
            _games.Add(new WeakReference<FrameworkElement>(element));
            Nav.SetIsDefault(element, IsSelected(element));
        }
    }

    /// <summary>A capa do jogo selecionado é a que recebe o foco ao voltar para a tela do tema.</summary>
    private void OnModelChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName != nameof(P10MainModel.SelectedGame))
        {
            return;
        }

        _games.RemoveAll(r => !r.TryGetTarget(out _));
        foreach (var reference in _games)
        {
            if (reference.TryGetTarget(out var element))
            {
                Nav.SetIsDefault(element, IsSelected(element));
            }
        }
    }

    private bool IsSelected(FrameworkElement element) => _model?.SelectedGame is { } game && ReferenceEquals(element.DataContext, game);

    /// <summary>Erro no layout do tema: some com o tema (fora do passe de layout) e avisa a Shell.</summary>
    private void Fail(Exception error)
    {
        if (_failed)
        {
            return;
        }

        _failed = true;
        var session = Session;
        Dispatcher.BeginInvoke(() =>
        {
            Child = null;
            if (session is not null && DataContext is ShellViewModel shell && ReferenceEquals(shell.PlayniteUi, session))
            {
                shell.FailPlayniteUi(error.InnerException is { } inner ? $"{error.Message} ({inner.Message})" : error.Message);
            }
        }, DispatcherPriority.Send);
    }
}
