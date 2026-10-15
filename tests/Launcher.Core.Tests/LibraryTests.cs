using Launcher.Core.Library;
using Launcher.Tests;
using Xunit;

namespace Launcher.Core.Tests;

public class LibraryTests
{
    private static readonly DateTimeOffset Now = new(2026, 10, 14, 20, 0, 0, TimeSpan.Zero);
    private readonly FixedTimeProvider _time = new(Now);

    private InMemoryGameLibrary CreateLibrary() => new(SampleLibrary.Load(_time), _time);

    [Fact]
    public void Sample_library_loads_every_game_with_art_urls_only()
    {
        var games = SampleLibrary.Load(_time);

        Assert.Equal(19, games.Count);
        Assert.All(games, g => Assert.True(g.CoverUri is null || g.CoverUri.Scheme == Uri.UriSchemeHttps));
        Assert.Contains(games, g => g.Id == "phantom" && g.IsUpcoming && !g.CanPlay);
        Assert.Contains(games, g => g.Store == GameStore.Manual && g.Note is not null);
    }

    [Fact]
    public void Recently_played_excludes_never_played_and_is_newest_first()
    {
        var recent = LibraryQuery.RecentlyPlayed(CreateLibrary().Games);

        Assert.DoesNotContain(recent, g => g.LastPlayedAt is null);
        Assert.Equal(recent.OrderByDescending(g => g.LastPlayedAt).Select(g => g.Id), recent.Select(g => g.Id));
        Assert.Equal("control", recent[0].Id);
    }

    [Fact]
    public void Recording_a_session_moves_the_game_to_the_front()
    {
        var library = CreateLibrary();
        var before = library.Find("gow3")!.PlayTime;
        _time.Now = Now.AddMinutes(30);

        library.RecordSession("gow3", TimeSpan.FromMinutes(25));

        Assert.Equal("gow3", LibraryQuery.RecentlyPlayed(library.Games)[0].Id);
        Assert.Equal(before + TimeSpan.FromMinutes(25), library.Find("gow3")!.PlayTime);
    }

    [Fact]
    public void Toggling_favorite_raises_changed()
    {
        var library = CreateLibrary();
        bool raised = false;
        library.Changed += (_, _) => raised = true;
        bool wasFavorite = library.Find("ds2")!.IsFavorite;

        library.ToggleFavorite("ds2");

        Assert.True(raised);
        Assert.NotEqual(wasFavorite, library.Find("ds2")!.IsFavorite);
    }

    [Theory]
    [InlineData(GameStore.Xbox, 4)]
    [InlineData(GameStore.Manual, 2)]
    [InlineData(GameStore.Gog, 1)]
    public void Store_filter_matches_only_that_store(GameStore store, int expected)
    {
        var result = LibraryQuery.Apply(CreateLibrary().Games, LibraryFilter.ForStore(store), LibrarySort.Name);

        Assert.Equal(expected, result.Count);
        Assert.All(result, g => Assert.Equal(store, g.Store));
    }

    [Fact]
    public void Name_sort_is_alphabetical_and_recent_sort_puts_unplayed_last()
    {
        var games = CreateLibrary().Games;

        var byName = LibraryQuery.Apply(games, LibraryFilter.All, LibrarySort.Name);
        var byRecent = LibraryQuery.Apply(games, LibraryFilter.All, LibrarySort.RecentlyPlayed);

        Assert.Equal(byName.Select(g => g.Title).Order(StringComparer.CurrentCultureIgnoreCase), byName.Select(g => g.Title));
        Assert.True(byRecent.SkipWhile(g => g.LastPlayedAt is not null).All(g => g.LastPlayedAt is null));
    }
}
