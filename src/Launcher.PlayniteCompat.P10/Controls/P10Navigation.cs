using System.Windows;
using System.Windows.Input;

namespace Launcher.PlayniteCompat.P10.Controls;

/// <summary>Um elemento do tema que entra na navegação por controle e teclado do launcher.</summary>
public sealed class P10NavigableEventArgs(RoutedEvent routedEvent, FrameworkElement element) : RoutedEventArgs(routedEvent, element)
{
    public FrameworkElement Element { get; } = element;

    /// <summary>Grupo de navegação (ex.: <c>p10-games</c>, <c>p10-actions</c>). Nulo para escopos.</summary>
    public string? Group { get; init; }

    /// <summary>Chave estável dentro do grupo (ex.: o Id do jogo).</summary>
    public string? Key { get; init; }

    /// <summary>Recebe o foco ao abrir a tela (ou o escopo).</summary>
    public bool IsDefault { get; init; }

    /// <summary>A do controle / Enter num elemento que não é botão (ex.: capa do jogo abre os detalhes).</summary>
    public ICommand? Activate { get; init; }

    public object? ActivateParameter { get; init; }

    /// <summary>Enquanto visível, só os elementos dentro dele são navegáveis (ex.: detalhes do jogo).</summary>
    public bool IsModalScope { get; init; }
}

/// <summary>
/// Ponte entre os controles <c>PART_</c> do adaptador e a navegação do launcher, sem o adaptador depender da UI do
/// launcher: cada elemento navegável anuncia o próprio papel num evento roteado, e o host do tema (no launcher) aplica
/// as propriedades de navegação dele.
/// </summary>
public static class P10Navigation
{
    public static readonly RoutedEvent NavigableEvent =
        EventManager.RegisterRoutedEvent("Navigable", RoutingStrategy.Bubble, typeof(EventHandler<P10NavigableEventArgs>), typeof(P10Navigation));

    private static readonly DependencyProperty AnnouncedProperty =
        DependencyProperty.RegisterAttached("Announced", typeof(bool), typeof(P10Navigation), new PropertyMetadata(false));

    /// <summary>Anuncia o elemento quando ele entra na árvore (uma vez por elemento).</summary>
    public static void Announce(FrameworkElement element, Func<FrameworkElement, P10NavigableEventArgs> create)
    {
        ArgumentNullException.ThrowIfNull(element);
        ArgumentNullException.ThrowIfNull(create);
        if ((bool)element.GetValue(AnnouncedProperty))
        {
            return;
        }

        element.SetValue(AnnouncedProperty, true);
        if (element.IsLoaded)
        {
            element.RaiseEvent(create(element));
            return;
        }

        void OnLoaded(object sender, RoutedEventArgs e)
        {
            element.Loaded -= OnLoaded;
            element.RaiseEvent(create(element));
        }

        element.Loaded += OnLoaded;
    }

    public static P10NavigableEventArgs Args(FrameworkElement element, string? group, string? key) =>
        new(NavigableEvent, element) { Group = group, Key = key };
}
