using System.Globalization;
using System.Windows;
using Launcher.PlayniteCompat.P10.Markup;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public class PlayniteConverterTests
{
    private static readonly CultureInfo Culture = CultureInfo.InvariantCulture;
    private static readonly string[] Genres = ["RPG", "Ação"];

    [Fact]
    public void Null_to_visibility_hides_only_null()
    {
        var converter = new NullToVisibilityConverter();

        Assert.Equal(Visibility.Collapsed, converter.Convert(null, typeof(Visibility), null, Culture));
        Assert.Equal(Visibility.Visible, converter.Convert("x", typeof(Visibility), null, Culture));
    }

    [Theory]
    [InlineData("https://example.com/icon.png")]
    [InlineData(@"\\server\share\icon.png")]
    [InlineData("icon.png")]
    [InlineData("")]
    public void Image_string_ignores_network_and_relative_paths(string value) =>
        Assert.Null(new ImageStringToImageConverter().Convert(value, typeof(object), null, Culture));

    [Fact]
    public void List_and_play_time_produce_launcher_text()
    {
        Assert.Equal("RPG, Ação", new ListToStringConverter().Convert(Genres, typeof(string), null, Culture));
        Assert.Equal("12 h", new PlayTimeToStringConverter().Convert(12UL * 3600, typeof(string), null, Culture));
        Assert.Equal("Nunca jogado", new DateTimeToLastPlayedConverter().Convert(null, typeof(string), null, Culture));
    }
}
