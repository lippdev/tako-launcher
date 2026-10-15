using System.Collections;
using System.Globalization;
using System.IO;
using System.Windows;
using System.Windows.Data;
using System.Windows.Media.Imaging;
using Launcher.Core.Presentation;

namespace Launcher.PlayniteCompat.P10.Markup;

/// <summary>
/// Conversores que o Playnite registra nos recursos do app e que os temas usam por <c>{StaticResource}</c>.
/// Aqui só convertem dados do launcher; nenhum lê valores vindos do tema.
/// </summary>
public static class PlayniteConverters
{
    public static ResourceDictionary Resources() => new()
    {
        ["True"] = true,
        ["False"] = false,
        ["NullToVisibilityConverter"] = new NullToVisibilityConverter(),
        ["ImageStringToImageConverter"] = new ImageStringToImageConverter(),
        ["ListToStringConverter"] = new ListToStringConverter(),
        ["PlayTimeToStringConverter"] = new PlayTimeToStringConverter(),
        ["DateTimeToLastPlayedConverter"] = new DateTimeToLastPlayedConverter(),
    };
}

/// <summary>Nulo → Collapsed; qualquer valor → Visible.</summary>
public sealed class NullToVisibilityConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value is null ? Visibility.Collapsed : Visibility.Visible;

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => Binding.DoNothing;
}

/// <summary>
/// Ícone de menu (caminho de arquivo local ou recurso do app) → imagem. Endereços de rede são ignorados:
/// a arte remota passa pelo cache do launcher, não por um tema.
/// </summary>
public sealed class ImageStringToImageConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is not string text || text.Length == 0)
        {
            return null;
        }

        Uri? uri = text.StartsWith("pack://application:,,,/", StringComparison.OrdinalIgnoreCase)
            ? new Uri(text)
            : Path.IsPathFullyQualified(text) && File.Exists(text) ? new Uri(text) : null;
        if (uri is null)
        {
            return null;
        }

        try
        {
            var image = new BitmapImage();
            image.BeginInit();
            image.CacheOption = BitmapCacheOption.OnLoad;
            image.UriSource = uri;
            image.EndInit();
            image.Freeze();
            return image;
        }
        catch (Exception ex) when (ex is IOException or NotSupportedException or UriFormatException or FileFormatException)
        {
            return null;
        }
    }

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => Binding.DoNothing;
}

/// <summary>Lista (gêneros, plataformas...) → "A, B, C".</summary>
public sealed class ListToStringConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture) => value switch
    {
        string text => text,
        IEnumerable items => string.Join(", ", items.Cast<object?>().Where(i => i is not null)),
        _ => string.Empty,
    };

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => Binding.DoNothing;
}

/// <summary>Tempo de jogo (segundos, como no Playnite, ou TimeSpan) → "12 h".</summary>
public sealed class PlayTimeToStringConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture) => value switch
    {
        TimeSpan time => GameText.PlayTime(time),
        ulong seconds => GameText.PlayTime(TimeSpan.FromSeconds(seconds)),
        long seconds when seconds >= 0 => GameText.PlayTime(TimeSpan.FromSeconds(seconds)),
        int seconds when seconds >= 0 => GameText.PlayTime(TimeSpan.FromSeconds(seconds)),
        _ => GameText.PlayTime(TimeSpan.Zero),
    };

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => Binding.DoNothing;
}

/// <summary>Última vez jogado → "Há 3 dias".</summary>
public sealed class DateTimeToLastPlayedConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture) => value switch
    {
        DateTimeOffset at => GameText.LastPlayed(at, DateTimeOffset.Now),
        DateTime at => GameText.LastPlayed(new DateTimeOffset(at), DateTimeOffset.Now),
        _ => GameText.LastPlayed(null, DateTimeOffset.Now),
    };

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => Binding.DoNothing;
}
