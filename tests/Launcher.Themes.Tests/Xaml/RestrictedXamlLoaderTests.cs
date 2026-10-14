using System.IO;
using System.Runtime.ExceptionServices;
using System.Windows;
using System.Windows.Media;
using Launcher.Themes.Xaml;
using Xunit;

namespace Launcher.Themes.Tests.Xaml;

public sealed class RestrictedXamlLoaderTests : IDisposable
{
    private const string Namespaces =
        """xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" """;

    private readonly string _themeRoot = Directory.CreateTempSubdirectory("ultralauncher-theme-").FullName;
    private readonly RestrictedXamlLoader _loader = new(new XamlValidator(XamlSecurityPolicy.CreateDefault()));

    public void Dispose() => Directory.Delete(_themeRoot, recursive: true);

    private string WriteThemeFile(string relativePath, string content)
    {
        string path = Path.Combine(_themeRoot, relativePath);
        Directory.CreateDirectory(Path.GetDirectoryName(path)!); // Combine com arquivo sempre tem diretório.
        File.WriteAllText(path, content);
        return relativePath;
    }

    /// <summary>Objetos WPF exigem thread STA.</summary>
    private static T RunOnSta<T>(Func<T> action)
    {
        T result = default!; // Atribuído pela thread ou exceção relançada abaixo.
        ExceptionDispatchInfo? failure = null;
        var thread = new Thread(() =>
        {
            try
            {
                result = action();
            }
            catch (Exception ex) when (ex is not OutOfMemoryException)
            {
                failure = ExceptionDispatchInfo.Capture(ex);
            }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        thread.Join();
        failure?.Throw();
        return result;
    }

    [Fact]
    public void Loads_allowed_dictionary()
    {
        string file = WriteThemeFile("Views/Main.xaml", $$"""
            <ResourceDictionary {{Namespaces}} xmlns:sys="clr-namespace:System;assembly=mscorlib">
                <sys:Double x:Key="CoverWidth">240</sys:Double>
                <SolidColorBrush x:Key="Accent" Color="#FF3A7BD5" />
                <Style x:Key="Title" TargetType="{x:Type TextBlock}">
                    <Setter Property="FontSize" Value="{StaticResource CoverWidth}" />
                </Style>
            </ResourceDictionary>
            """);

        var (width, color, hasStyle) = RunOnSta(() =>
        {
            var dictionary = _loader.LoadResourceDictionary(_themeRoot, file);
            return ((double)dictionary["CoverWidth"], ((SolidColorBrush)dictionary["Accent"]).Color, dictionary["Title"] is Style);
        });

        Assert.Equal(240d, width);
        Assert.Equal(Color.FromArgb(0xFF, 0x3A, 0x7B, 0xD5), color);
        Assert.True(hasStyle);
    }

    [Fact]
    public void Rejects_ObjectDataProvider_before_parsing()
    {
        string marker = Path.Combine(_themeRoot, "pwned.txt");
        string file = WriteThemeFile("Views/Main.xaml", $$"""
            <ResourceDictionary {{Namespaces}} xmlns:io="clr-namespace:System.IO;assembly=System.Runtime" xmlns:sys="clr-namespace:System;assembly=mscorlib">
                <ObjectDataProvider x:Key="Run" ObjectType="{x:Type io:File}" MethodName="WriteAllText">
                    <ObjectDataProvider.MethodParameters>
                        <sys:String>{{marker}}</sys:String>
                        <sys:String>pwned</sys:String>
                    </ObjectDataProvider.MethodParameters>
                </ObjectDataProvider>
            </ResourceDictionary>
            """);

        var ex = Assert.Throws<ThemeXamlRejectedException>(() => RunOnSta(() => _loader.LoadResourceDictionary(_themeRoot, file)));

        Assert.NotEmpty(ex.Violations);
        Assert.False(File.Exists(marker));
    }

    [Fact]
    public void Rejects_event_handlers_from_wpf_parser()
    {
        string file = WriteThemeFile("Views/Main.xaml", $$"""
            <ResourceDictionary {{Namespaces}}>
                <Button x:Key="b" Click="OnClick" />
            </ResourceDictionary>
            """);

        var ex = Assert.Throws<ThemeXamlRejectedException>(() => RunOnSta(() => _loader.LoadResourceDictionary(_themeRoot, file)));

        Assert.IsType<System.Windows.Markup.XamlParseException>(ex.InnerException);
    }

    [Fact]
    public void Rejects_file_outside_theme_folder() =>
        Assert.Throws<ThemeXamlRejectedException>(() => _loader.LoadResourceDictionary(_themeRoot, "../outside.xaml"));
}
