using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace Launcher.UI.Wpf.Controls;

/// <summary>Ícone em grade 24x24: partes com traço (Stroke) e partes preenchidas (Fill).</summary>
public sealed class IconData
{
    public Geometry? Stroke { get; set; }

    public Geometry? Fill { get; set; }
}

/// <summary>Desenha um <see cref="IconData"/> na cor do <c>Foreground</c>; o traço escala com o tamanho. Template em Resources/Styles.xaml.</summary>
public sealed class Icon : Control
{
    public static readonly DependencyProperty DataProperty =
        DependencyProperty.Register(nameof(Data), typeof(IconData), typeof(Icon), new FrameworkPropertyMetadata(null, FrameworkPropertyMetadataOptions.AffectsRender));

    /// <summary>Chave do recurso (ex.: "Icon.Home"), alternativa a <see cref="Data"/>.</summary>
    public static readonly DependencyProperty KeyProperty =
        DependencyProperty.Register(nameof(Key), typeof(string), typeof(Icon), new PropertyMetadata(null, OnKeyChanged));

    public IconData? Data
    {
        get => (IconData?)GetValue(DataProperty);
        set => SetValue(DataProperty, value);
    }

    public string? Key
    {
        get => (string?)GetValue(KeyProperty);
        set => SetValue(KeyProperty, value);
    }

    private static void OnKeyChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var icon = (Icon)d;
        icon.Data = e.NewValue is string key ? icon.TryFindResource(key) as IconData : null;
    }
}
