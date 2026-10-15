using System.Globalization;
using System.Windows;
using System.Windows.Data;

namespace Launcher.UI.Wpf.Controls;

/// <summary>Visível quando o valor não é nulo nem texto vazio. Parâmetro "invert" inverte.</summary>
public sealed class NotNullToVisibilityConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        bool visible = value is string s ? s.Length > 0 : value is not null;
        if (parameter is "invert")
        {
            visible = !visible;
        }

        return visible ? Visibility.Visible : Visibility.Collapsed;
    }

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>bool → Visibility. Parâmetro "invert" inverte.</summary>
public sealed class BoolToVisibilityConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        bool visible = value is true;
        if (parameter is "invert")
        {
            visible = !visible;
        }

        return visible ? Visibility.Visible : Visibility.Collapsed;
    }

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>Converte uma fração 0..1 em largura proporcional ao parâmetro (largura total).</summary>
public sealed class FractionToWidthConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        double total = parameter is string p ? double.Parse(p, CultureInfo.InvariantCulture) : 100;
        return value is double fraction ? Math.Clamp(fraction, 0, 1) * total : 0d;
    }

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>Texto em caixa alta (títulos de seção / overline).</summary>
public sealed class UpperCaseConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        (value as string)?.ToUpper(CultureInfo.GetCultureInfo("pt-BR")) ?? string.Empty;

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}
