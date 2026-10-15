using System.IO;
using System.Windows;
using System.Windows.Data;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace Launcher.PlayniteCompat.P10.Markup;

/// <summary>
/// <c>{ThemeFile 'Images/x.png'}</c>: arquivo relativo à raiz do tema. Nunca sai da pasta do tema.
/// Devolve Uri, ImageSource ou texto conforme a propriedade de destino.
/// </summary>
[MarkupExtensionReturnType(typeof(object))]
public sealed class ThemeFileExtension(string path) : MarkupExtension
{
    public string Path { get; } = path;

    public override object? ProvideValue(IServiceProvider serviceProvider)
    {
        var baseUri = (serviceProvider.GetService(typeof(IUriContext)) as IUriContext)?.BaseUri;
        string? root = baseUri is { IsFile: true } ? FindThemeRoot(System.IO.Path.GetDirectoryName(baseUri.LocalPath)) : null;
        if (root is null)
        {
            return null;
        }

        string full = System.IO.Path.GetFullPath(System.IO.Path.Combine(root, Path));
        if (!full.StartsWith(root + System.IO.Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase) || !File.Exists(full))
        {
            return null;
        }

        var uri = new Uri(full);
        var target = (serviceProvider.GetService(typeof(IProvideValueTarget)) as IProvideValueTarget)?.TargetProperty as DependencyProperty;
        if (target?.PropertyType == typeof(Uri))
        {
            return uri;
        }

        if (target is not null && typeof(ImageSource).IsAssignableFrom(target.PropertyType))
        {
            var image = new BitmapImage(uri);
            image.Freeze();
            return image;
        }

        return target?.PropertyType == typeof(string) ? full : uri;
    }

    private static string? FindThemeRoot(string? directory)
    {
        for (var d = directory; d is not null; d = System.IO.Path.GetDirectoryName(d))
        {
            if (File.Exists(System.IO.Path.Combine(d, "theme.yaml")))
            {
                return System.IO.Path.GetFullPath(d);
            }
        }

        return null;
    }
}

/// <summary>
/// <c>{Settings Chave}</c>: configurações do Playnite que os temas leem. Valores padrão do modo Fullscreen;
/// a ponte com as preferências reais do launcher entra com o adaptador completo (MVP-018).
/// </summary>
[MarkupExtensionReturnType(typeof(object))]
public sealed class SettingsExtension(string path) : MarkupExtension
{
    private static readonly Dictionary<string, object> Defaults = new(StringComparer.Ordinal)
    {
        ["CoverArtStretch"] = Stretch.UniformToFill,
        ["Fullscreen.ShowGameTitles"] = false,
        ["Fullscreen.DarkenUninstalledGamesGrid"] = true,
        ["FullscreenItemSpacingMargin"] = new Thickness(8),
    };

    public string Path { get; } = path;

    public override object? ProvideValue(IServiceProvider serviceProvider)
    {
        Defaults.TryGetValue(Path, out object? value);
        var target = serviceProvider.GetService(typeof(IProvideValueTarget)) as IProvideValueTarget;

        // Em Condition.Binding (MultiDataTrigger) o Playnite fornece um Binding.
        if (target?.TargetProperty is System.Reflection.PropertyInfo { PropertyType: var bindingType } && typeof(BindingBase).IsAssignableFrom(bindingType))
        {
            return new Binding { Source = value };
        }

        if (target?.TargetProperty is DependencyProperty property && property.PropertyType == typeof(Visibility) && value is bool visible)
        {
            return visible ? Visibility.Visible : Visibility.Collapsed;
        }

        return value ?? DependencyProperty.UnsetValue;
    }
}
