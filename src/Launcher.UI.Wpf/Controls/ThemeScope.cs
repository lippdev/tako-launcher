using System.Windows;
using System.Windows.Controls;
using Launcher.UI.Wpf.Navigation;

namespace Launcher.UI.Wpf.Controls;

/// <summary>Aplica um dicionário de tema só ao conteúdo (prévia isolada, sem afetar o resto do launcher).</summary>
public sealed class ThemeScope : ContentControl
{
    public static readonly DependencyProperty ThemeResourcesProperty =
        DependencyProperty.Register(nameof(ThemeResources), typeof(ResourceDictionary), typeof(ThemeScope), new PropertyMetadata(null, OnThemeChanged));

    public ThemeScope()
    {
        Focusable = false;
    }

    public ResourceDictionary? ThemeResources
    {
        get => (ResourceDictionary?)GetValue(ThemeResourcesProperty);
        set => SetValue(ThemeResourcesProperty, value);
    }

    private static void OnThemeChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var scope = (ThemeScope)d;
        scope.Resources.MergedDictionaries.Clear();
        if (e.NewValue is ResourceDictionary resources)
        {
            scope.Resources.MergedDictionaries.Add(resources);
        }
    }
}

/// <summary>
/// Lista cujos itens entram na navegação por controle. O visual dos itens vem do tema (ex.: ListGameItemStyle do
/// Playnite, que reage a IsFocused); aqui só marcamos os contêineres como navegáveis.
/// </summary>
public sealed class NavigableListBox : ListBox
{
    public static readonly DependencyProperty ItemGroupProperty =
        DependencyProperty.Register(nameof(ItemGroup), typeof(string), typeof(NavigableListBox), new PropertyMetadata("list"));

    public string ItemGroup
    {
        get => (string)GetValue(ItemGroupProperty);
        set => SetValue(ItemGroupProperty, value);
    }

    protected override void PrepareContainerForItemOverride(DependencyObject element, object item)
    {
        base.PrepareContainerForItemOverride(element, item);
        if (element is ListBoxItem container)
        {
            container.Focusable = true;
            Nav.SetGroup(container, ItemGroup);
            Nav.SetKey(container, $"{ItemGroup}:{Items.IndexOf(item)}");
            Nav.SetIsDefault(container, Items.IndexOf(item) == 0);
        }
    }
}
