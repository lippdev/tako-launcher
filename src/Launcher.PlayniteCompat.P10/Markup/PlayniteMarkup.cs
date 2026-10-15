using System.ComponentModel;
using System.Globalization;
using System.IO;
using System.Windows;
using System.Windows.Data;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Launcher.PlayniteCompat.P10.Data;

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
        string? full = ThemeFiles.Resolve(ThemeFiles.RootOf(serviceProvider), Path);
        if (full is null)
        {
            return null;
        }

        var target = (serviceProvider.GetService(typeof(IProvideValueTarget)) as IProvideValueTarget)?.TargetProperty as DependencyProperty;
        return ThemeFiles.As(full, target?.PropertyType);
    }
}

/// <summary>
/// <c>{ThemeFileBinding Library, PathFormat='Icons/Library/{0}.png'}</c> (MVP-018): binding a um dado do jogo que vira
/// um arquivo do tema. Mesma proteção do <c>{ThemeFile}</c>: o caminho montado com o dado precisa continuar dentro da
/// pasta do tema e existir; senão o valor é nulo (o tema costuma esconder a imagem nesse caso).
/// </summary>
[MarkupExtensionReturnType(typeof(object))]
public sealed class ThemeFileBindingExtension : MarkupExtension
{
    public ThemeFileBindingExtension()
    {
    }

    public ThemeFileBindingExtension(string path)
    {
        Path = path;
    }

    /// <summary>Caminho do dado (ex.: <c>Library</c>, <c>Game.Platforms[0].Name</c>).</summary>
    public string? Path { get; set; }

    /// <summary>Formato do arquivo; <c>{0}</c> recebe o dado. Sem formato, o dado é o próprio caminho relativo.</summary>
    public string? PathFormat { get; set; }

    public object? FallbackValue { get; set; }

    public override object? ProvideValue(IServiceProvider serviceProvider)
    {
        var target = (serviceProvider.GetService(typeof(IProvideValueTarget)) as IProvideValueTarget)?.TargetProperty as DependencyProperty;
        var binding = new Binding(Path ?? string.Empty)
        {
            Mode = BindingMode.OneWay,
            Converter = new ThemeFileConverter(ThemeFiles.RootOf(serviceProvider), PathFormat, target?.PropertyType),
        };
        if (FallbackValue is not null)
        {
            binding.FallbackValue = FallbackValue;
        }

        return binding.ProvideValue(serviceProvider);
    }

    private sealed class ThemeFileConverter(string? root, string? format, Type? targetType) : IValueConverter
    {
        public object? Convert(object? value, Type targetType2, object? parameter, CultureInfo culture)
        {
            string? data = System.Convert.ToString(value, CultureInfo.InvariantCulture);
            if (string.IsNullOrWhiteSpace(data))
            {
                return null;
            }

            string relative = format is null ? data : string.Format(CultureInfo.InvariantCulture, format, data);
            string? full = ThemeFiles.Resolve(root, relative);
            return full is null ? null : ThemeFiles.As(full, targetType ?? targetType2);
        }

        public object ConvertBack(object? value, Type targetType2, object? parameter, CultureInfo culture) => Binding.DoNothing;
    }
}

/// <summary>Arquivos do tema: raiz pelo <c>BaseUri</c> do XAML e a proteção de caminho comum às extensões.</summary>
internal static class ThemeFiles
{
    public static string? RootOf(IServiceProvider serviceProvider)
    {
        var baseUri = (serviceProvider.GetService(typeof(IUriContext)) as IUriContext)?.BaseUri;
        return baseUri is { IsFile: true } ? FindThemeRoot(System.IO.Path.GetDirectoryName(baseUri.LocalPath)) : null;
    }

    /// <summary>Caminho absoluto de um arquivo existente dentro do tema; nulo se sair da pasta ou não existir.</summary>
    public static string? Resolve(string? root, string relative)
    {
        if (root is null || string.IsNullOrWhiteSpace(relative) || relative.IndexOfAny(System.IO.Path.GetInvalidPathChars()) >= 0 || relative.Contains(':', StringComparison.Ordinal))
        {
            return null;
        }

        string full;
        try
        {
            full = System.IO.Path.GetFullPath(System.IO.Path.Combine(root, relative.TrimStart('/', '\\')));
        }
        catch (Exception ex) when (ex is ArgumentException or NotSupportedException or PathTooLongException)
        {
            return null;
        }

        return full.StartsWith(root + System.IO.Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase) && File.Exists(full) ? full : null;
    }

    /// <summary>O arquivo no formato da propriedade de destino: Uri, imagem ou o caminho.</summary>
    public static object? As(string full, Type? propertyType)
    {
        var uri = new Uri(full);
        if (propertyType == typeof(Uri))
        {
            return uri;
        }

        if (propertyType is not null && typeof(ImageSource).IsAssignableFrom(propertyType))
        {
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
            catch (Exception ex) when (ex is IOException or NotSupportedException or FileFormatException or UriFormatException or InvalidOperationException)
            {
                return null;
            }
        }

        return propertyType == typeof(string) ? full : uri;
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
/// Valor fixo de uma extensão do Playnite: valor direto numa propriedade de dependência; nos demais destinos
/// (<c>DataTrigger.Binding</c>, <c>Condition.Binding</c>, <c>Setter.Value</c>), um binding OneWay para o valor.
/// </summary>
internal static class StaticValue
{
    public static object? Provide(IServiceProvider serviceProvider, object? value)
    {
        var target = serviceProvider.GetService(typeof(IProvideValueTarget)) as IProvideValueTarget;
        if (target?.TargetProperty is not DependencyProperty property)
        {
            return new Binding { Source = value, Mode = BindingMode.OneWay };
        }

        if (property.PropertyType == typeof(Visibility) && value is bool visible)
        {
            return visible ? Visibility.Visible : Visibility.Collapsed;
        }

        // FallbackValue chega como texto do XAML; extensões não passam pelo conversor de tipo do parser.
        if (value is string text && property.PropertyType != typeof(string) && property.PropertyType != typeof(object))
        {
            try
            {
                return TypeDescriptor.GetConverter(property.PropertyType).ConvertFromInvariantString(text) ?? DependencyProperty.UnsetValue;
            }
            catch (Exception ex) when (ex is NotSupportedException or FormatException or ArgumentException)
            {
                return DependencyProperty.UnsetValue;
            }
        }

        return value is null || !property.PropertyType.IsInstanceOfType(value) ? DependencyProperty.UnsetValue : value;
    }
}

/// <summary>Configurações do Playnite que os temas leem, com os valores que o Tako usa (MVP-018).</summary>
internal static class P10SettingsValues
{
    private static readonly Dictionary<string, object> Values = new(StringComparer.Ordinal)
    {
        ["CoverArtStretch"] = Stretch.UniformToFill,
        ["GridItemHeightRatio"] = 1.5,
        ["FullscreenItemSpacingMargin"] = new Thickness(8),
        ["Fullscreen.ShowGameTitles"] = false,
        ["Fullscreen.DarkenUninstalledGamesGrid"] = true,
        ["Fullscreen.HorizontalLayout"] = false,
        ["Fullscreen.Columns"] = 7,
        ["Fullscreen.Rows"] = 2,
        ["Fullscreen.ShowBattery"] = false,
        ["Fullscreen.ShowBatteryPercentage"] = false,
        ["Fullscreen.ShowClock"] = true,
        ["Fullscreen.MainBackgroundImageDarkAmount"] = 0.4,
        ["Fullscreen.EnableMainBackgroundImage"] = true,
        ["Fullscreen.IsMusicMuted"] = true,
        ["Fullscreen.EnableGameSounds"] = false,
    };

    public static object? Get(string? path) => path is not null && Values.TryGetValue(path, out object? value) ? value : null;
}

/// <summary>
/// <c>{Settings Chave}</c>: configurações do Playnite que os temas leem, com os valores do Tako. <c>Mode</c> é aceito
/// para compatibilidade (ex.: <c>Mode=TwoWay</c> num CheckBox), mas nada é gravado nas configurações.
/// </summary>
[MarkupExtensionReturnType(typeof(object))]
public sealed class SettingsExtension : MarkupExtension
{
    public SettingsExtension()
    {
    }

    public SettingsExtension(string path)
    {
        Path = path;
    }

    public string? Path { get; set; }

    public BindingMode Mode { get; set; }

    public object? FallbackValue { get; set; }

    public override object? ProvideValue(IServiceProvider serviceProvider) =>
        StaticValue.Provide(serviceProvider, P10SettingsValues.Get(Path) ?? FallbackValue);
}

/// <summary>
/// <c>{SettingsBinding Fullscreen.IsMusicMuted, Mode=TwoWay}</c> (MVP-018): binding para uma cópia dos valores do Tako.
/// Uma escrita do tema (TwoWay) muda só essa cópia, nunca as configurações do launcher.
/// </summary>
[MarkupExtensionReturnType(typeof(object))]
public sealed class SettingsBindingExtension : MarkupExtension
{
    public SettingsBindingExtension()
    {
    }

    public SettingsBindingExtension(string path)
    {
        Path = path;
    }

    public string? Path { get; set; }

    public BindingMode Mode { get; set; } = BindingMode.OneWay;

    public object? FallbackValue { get; set; }

    public override object? ProvideValue(IServiceProvider serviceProvider)
    {
        var binding = new Binding(nameof(SettingValue.Value))
        {
            Source = new SettingValue(P10SettingsValues.Get(Path) ?? FallbackValue),
            Mode = Mode,
        };
        return binding.ProvideValue(serviceProvider);
    }

    /// <summary>Valor isolado por uso da extensão.</summary>
    public sealed class SettingValue(object? value)
    {
        public object? Value { get; set; } = value;
    }
}

/// <summary>
/// <c>{PluginSettings Plugin=X, Path=Y}</c> (MVP-018): configurações de extensões do Playnite. O Tako não roda
/// extensões, então o valor é o mesmo do Playnite com a extensão ausente: <c>FallbackValue</c> ou nulo.
/// </summary>
[MarkupExtensionReturnType(typeof(object))]
public sealed class PluginSettingsExtension : MarkupExtension
{
    public string? Plugin { get; set; }

    public string? Path { get; set; }

    public object? FallbackValue { get; set; }

    public override object? ProvideValue(IServiceProvider serviceProvider) => StaticValue.Provide(serviceProvider, FallbackValue);
}

/// <summary><c>{PluginStatus Plugin=X, Status=Installed}</c>: nenhuma extensão do Playnite está instalada no Tako.</summary>
[MarkupExtensionReturnType(typeof(object))]
public sealed class PluginStatusExtension : MarkupExtension
{
    public string? Plugin { get; set; }

    public string? Status { get; set; }

    public override object? ProvideValue(IServiceProvider serviceProvider) => StaticValue.Provide(serviceProvider, false);
}

/// <summary>
/// <c>{Api Notifications.Count}</c>, <c>{Api MainModel.SelectedGame.Name}</c> (MVP-018): leitura da <see cref="P10Api"/>
/// da tela do tema (<see cref="P10Context"/>). O binding é sempre OneWay e a API não tem nada gravável nem métodos.
/// </summary>
[MarkupExtensionReturnType(typeof(object))]
public sealed class ApiExtension : MarkupExtension
{
    public ApiExtension()
    {
    }

    public ApiExtension(string path)
    {
        Path = path;
    }

    public string? Path { get; set; }

    public object? FallbackValue { get; set; }

    public override object? ProvideValue(IServiceProvider serviceProvider)
    {
        string path = string.IsNullOrWhiteSpace(Path) ? "(0)" : $"(0).{Path.Trim()}";
        var binding = new Binding
        {
            Path = new PropertyPath(path, P10Context.ApiProperty),
            RelativeSource = RelativeSource.Self,
            Mode = BindingMode.OneWay,
        };
        if (FallbackValue is not null)
        {
            binding.FallbackValue = FallbackValue;
        }

        return binding.ProvideValue(serviceProvider);
    }
}
