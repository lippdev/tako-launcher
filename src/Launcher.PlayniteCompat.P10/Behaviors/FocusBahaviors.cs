using System.Windows;
using System.Windows.Threading;

namespace Launcher.PlayniteCompat.P10.Behaviors;

#pragma warning disable CA1724 // O nome (com o erro de grafia) é o que os temas referenciam: pbeh:FocusBahaviors.

/// <summary>
/// <c>pbeh:FocusBahaviors</c> do Playnite (<c>Playnite.Behaviors</c>), liberado por nome no adaptador (MVP-018):
/// <see cref="GetFocusBinding">FocusBinding</see> foca o elemento quando o valor fica verdadeiro e
/// <see cref="GetOnVisibilityFocus">OnVisibilityFocus</see> foca quando ele aparece. Só move o foco do teclado dentro da
/// própria tela do tema; não executa nada.
/// </summary>
public static class FocusBahaviors
{
    public static readonly DependencyProperty FocusBindingProperty =
        DependencyProperty.RegisterAttached("FocusBinding", typeof(bool), typeof(FocusBahaviors), new PropertyMetadata(false, OnFocusBindingChanged));

    public static readonly DependencyProperty OnVisibilityFocusProperty =
        DependencyProperty.RegisterAttached("OnVisibilityFocus", typeof(bool), typeof(FocusBahaviors), new PropertyMetadata(false, OnVisibilityFocusChanged));

    public static bool GetFocusBinding(DependencyObject element)
    {
        ArgumentNullException.ThrowIfNull(element);
        return (bool)element.GetValue(FocusBindingProperty);
    }

    public static void SetFocusBinding(DependencyObject element, bool value)
    {
        ArgumentNullException.ThrowIfNull(element);
        element.SetValue(FocusBindingProperty, value);
    }

    public static bool GetOnVisibilityFocus(DependencyObject element)
    {
        ArgumentNullException.ThrowIfNull(element);
        return (bool)element.GetValue(OnVisibilityFocusProperty);
    }

    public static void SetOnVisibilityFocus(DependencyObject element, bool value)
    {
        ArgumentNullException.ThrowIfNull(element);
        element.SetValue(OnVisibilityFocusProperty, value);
    }

    private static void OnFocusBindingChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        if (d is UIElement element && e.NewValue is true && element.IsVisible)
        {
            FocusLater(element);
        }
    }

    private static void OnVisibilityFocusChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        if (d is not UIElement element)
        {
            return;
        }

        element.IsVisibleChanged -= OnIsVisibleChanged;
        if (e.NewValue is true)
        {
            element.IsVisibleChanged += OnIsVisibleChanged;
        }
    }

    private static void OnIsVisibleChanged(object sender, DependencyPropertyChangedEventArgs e)
    {
        if (sender is UIElement element && e.NewValue is true)
        {
            FocusLater(element);
        }
    }

    private static void FocusLater(UIElement element) =>
        element.Dispatcher.BeginInvoke(() => element.Focus(), DispatcherPriority.Input);
}
