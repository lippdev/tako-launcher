using System.Windows;
using System.Windows.Media;

namespace Launcher.UI.Wpf.Controls;

/// <summary>
/// Esmaece as bordas esquerda e direita de uma fileira que rola (Central de Controle): o cartão cortado pela borda
/// some aos poucos, indicando que a fileira continua. A largura do esmaecimento é fixa em unidades de layout, qualquer
/// que seja a largura do palco (16:9, 21:9, 4:3). O item focado nunca fica na faixa esmaecida (folga do RowReveal).
/// </summary>
public static class EdgeFade
{
    public static readonly DependencyProperty WidthProperty =
        DependencyProperty.RegisterAttached("Width", typeof(double), typeof(EdgeFade), new PropertyMetadata(0d, OnWidthChanged));

    public static double GetWidth(DependencyObject d) => (double)d.GetValue(WidthProperty);

    public static void SetWidth(DependencyObject d, double value) => d.SetValue(WidthProperty, value);

    private static void OnWidthChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        if (d is not FrameworkElement element)
        {
            return;
        }

        element.SizeChanged -= OnSizeChanged;
        if (e.NewValue is double width && width > 0)
        {
            element.SizeChanged += OnSizeChanged;
            Apply(element);
        }
        else
        {
            element.ClearValue(UIElement.OpacityMaskProperty);
        }
    }

    private static void OnSizeChanged(object sender, SizeChangedEventArgs e) => Apply((FrameworkElement)sender);

    private static void Apply(FrameworkElement element)
    {
        double total = element.ActualWidth;
        double fade = GetWidth(element);
        if (total <= fade * 2)
        {
            element.ClearValue(UIElement.OpacityMaskProperty);
            return;
        }

        double edge = fade / total;
        var mask = new LinearGradientBrush { StartPoint = new Point(0, 0), EndPoint = new Point(1, 0) };
        mask.GradientStops.Add(new GradientStop(Colors.Transparent, 0));
        mask.GradientStops.Add(new GradientStop(Colors.Black, edge));
        mask.GradientStops.Add(new GradientStop(Colors.Black, 1 - edge));
        mask.GradientStops.Add(new GradientStop(Colors.Transparent, 1));
        mask.Freeze();
        element.OpacityMask = mask;
    }
}
