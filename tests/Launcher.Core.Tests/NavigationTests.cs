using Launcher.Core.Navigation;
using Xunit;

namespace Launcher.Core.Tests;

public class NavigationTests
{
    // Duas fileiras de capas (como a Início) e um botão acima.
    private static readonly NavCandidate<string>[] Layout =
    [
        new("play", new NavRect(184, 380, 210, 76), "hero"),
        new("r1c1", new NavRect(184, 560, 204, 306), "row1"),
        new("r1c2", new NavRect(414, 560, 204, 306), "row1"),
        new("r1c3", new NavRect(644, 560, 204, 306), "row1"),
        new("r2c1", new NavRect(184, 940, 204, 306), "row2"),
        new("r2c2", new NavRect(414, 940, 204, 306), "row2"),
    ];

    private static string? Next(string from, NavDirection direction) =>
        SpatialNavigator.FindNext(Layout.Single(c => c.Item == from).Rect, Layout, direction)?.Item;

    [Theory]
    [InlineData("r1c1", NavDirection.Right, "r1c2")]
    [InlineData("r1c2", NavDirection.Left, "r1c1")]
    [InlineData("r1c2", NavDirection.Down, "r2c2")]
    [InlineData("r1c3", NavDirection.Up, "play")]
    [InlineData("play", NavDirection.Down, "r1c1")]
    public void Picks_the_nearest_neighbor_in_the_direction(string from, NavDirection direction, string expected) =>
        Assert.Equal(expected, Next(from, direction));

    [Theory]
    [InlineData("r1c3", NavDirection.Right)]
    [InlineData("play", NavDirection.Up)]
    public void Returns_nothing_at_the_edge(string from, NavDirection direction) => Assert.Null(Next(from, direction));
}
