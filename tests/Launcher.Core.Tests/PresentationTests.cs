using Launcher.Core.Library;
using Launcher.Tests;
using Launcher.Core.Presentation;
using Xunit;

namespace Launcher.Core.Tests;

public class PresentationTests
{
    private static readonly DateTimeOffset Now = new(2026, 10, 14, 20, 0, 0, TimeSpan.Zero);

    [Theory]
    [InlineData(null, "Nunca jogado")]
    [InlineData(0.1, "Agora mesmo")]
    [InlineData(1.2, "Jogado ontem")]
    [InlineData(3.5, "Há 3 dias")]
    [InlineData(9.0, "Há 1 semana")]
    [InlineData(21.0, "Há 3 semanas")]
    [InlineData(45.0, "Há 1 mês")]
    [InlineData(65.0, "Há 2 meses")]
    public void Last_played_uses_relative_portuguese_text(double? daysAgo, string expected)
    {
        DateTimeOffset? at = daysAgo is double d ? Now.AddDays(-d) : null;

        Assert.Equal(expected, GameText.LastPlayed(at, Now));
    }

    [Fact]
    public void Every_sample_accent_keeps_white_text_readable()
    {
        foreach (var game in SampleLibrary.Load(new FixedTimeProvider(Now)))
        {
            var palette = AccentPalette.For(game.Accent, dynamicColor: true);

            Assert.True(AccentPalette.Contrast(palette.Accent, new Rgb(255, 255, 255)) >= 4.5, game.Id);
        }
    }

    [Fact]
    public void Disabling_dynamic_color_uses_the_slate_fallback()
    {
        Assert.Equal(AccentPalette.Fallback, AccentPalette.For("#C8102E", dynamicColor: false).Accent);
    }

    [Theory]
    [InlineData("#C8102E", 200, 16, 46)]
    [InlineData("2B3A55", 43, 58, 85)]
    public void Rgb_parses_hex(string hex, byte r, byte g, byte b) => Assert.Equal(new Rgb(r, g, b), Rgb.Parse(hex));

    [Fact]
    public void Dates_use_brazilian_format() => Assert.Equal("29/10/2026", GameText.Date(new DateOnly(2026, 10, 29)));
}
