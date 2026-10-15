using Launcher.Core.Library;
using Launcher.Core.Presentation;
using Launcher.Tests;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Busca, filtros e ordenação da biblioteca (UX-001/002/004).</summary>
public class LibrarySearchTests
{
    private static readonly DateTimeOffset Now = new(2026, 10, 14, 20, 0, 0, TimeSpan.Zero);

    private static IReadOnlyList<Game> Games() => SampleLibrary.Load(new FixedTimeProvider(Now));

    private static List<string> Ids(IEnumerable<Game> games) => games.Select(g => g.Id).ToList();

    [Theory]
    [InlineData("Pokémon Édition", "pokemon edition")]
    [InlineData("  God of War:   Ragnarök ", "god of war ragnarok")]
    [InlineData("Assassin's Creed® Shadows™", "assassins creed shadows")]
    [InlineData("Indiana Jones e o Grande Círculo", "indiana jones e o grande circulo")]
    [InlineData("ÆON Ølstad Łódź", "æon olstad lodz")]
    [InlineData("", "")]
    [InlineData(null, "")]
    public void Normalize_drops_case_accents_and_punctuation(string? text, string expected) =>
        Assert.Equal(expected, SearchText.Normalize(text));

    [Theory]
    [InlineData("God of War Ragnarök", "ragnarok", true)]
    [InlineData("God of War Ragnarök", "RAGNARÖK", true)]
    [InlineData("God of War Ragnarök", "war god", true)] // qualquer ordem
    [InlineData("God of War Ragnarök", "gow", false)]
    [InlineData("Resident Evil Requiem", "residentevil", true)] // sem espaços
    [InlineData("Indiana Jones e o Grande Círculo", "circulo", true)]
    [InlineData("Hollow Knight: Silksong", "knight silk", true)]
    [InlineData("Hollow Knight: Silksong", "knight silkk", false)]
    [InlineData("Qualquer jogo", "", true)]
    [InlineData("Qualquer jogo", "   ", true)]
    public void Matches_every_word_without_case_or_accents(string title, string query, bool expected) =>
        Assert.Equal(expected, SearchText.Matches(title, query));

    [Fact]
    public void Search_finds_games_live_by_partial_title()
    {
        var games = Games();

        var gow = LibraryQuery.Apply(games, LibraryFilter.All, LibraryCriteria.None with { Search = "god of", Sort = LibrarySort.Name });
        var circle = LibraryQuery.Apply(games, LibraryFilter.All, LibraryCriteria.None with { Search = "CIRCULO" });
        var none = LibraryQuery.Apply(games, LibraryFilter.All, LibraryCriteria.None with { Search = "zzz" });

        Assert.Equal(["gow3", "ragnarok"], Ids(gow));
        Assert.Equal(["indiana"], Ids(circle));
        Assert.Empty(none);
    }

    [Fact]
    public void Search_combines_with_the_sidebar_route()
    {
        var steamOnly = LibraryQuery.Apply(Games(), LibraryFilter.ForStore(GameStore.Steam), LibraryCriteria.None with { Search = "god" });

        Assert.Equal(["ragnarok"], Ids(steamOnly)); // o God of War III é adicionado manualmente
    }

    [Fact]
    public void Install_filter_cycles_and_splits_the_library()
    {
        var games = Games();
        var criteria = LibraryCriteria.None.NextInstall();
        Assert.Equal(InstallFilter.Installed, criteria.Install);
        var installed = LibraryQuery.Apply(games, LibraryFilter.All, criteria);

        criteria = criteria.NextInstall();
        Assert.Equal(InstallFilter.NotInstalled, criteria.Install);
        var notInstalled = LibraryQuery.Apply(games, LibraryFilter.All, criteria);

        Assert.All(installed, g => Assert.True(g.IsInstalled));
        Assert.Equal(["ds2", "phantom"], Ids(notInstalled).Order());
        Assert.Equal(games.Count, installed.Count + notInstalled.Count);
        Assert.Equal(InstallFilter.Any, criteria.NextInstall().Install);
    }

    [Fact]
    public void Panel_filters_are_or_within_a_group_and_and_between_groups()
    {
        var games = Games();
        var xboxOrGog = LibraryCriteria.None.ToggleSource("Xbox").ToggleSource("GOG");

        var sources = LibraryQuery.Apply(games, LibraryFilter.All, xboxOrGog with { Sort = LibrarySort.Name });
        var favoriteSources = LibraryQuery.Apply(games, LibraryFilter.All, xboxOrGog with { FavoritesOnly = true, Sort = LibrarySort.Name });
        var rpgNeverPlayed = LibraryQuery.Apply(games, LibraryFilter.All, LibraryCriteria.None.ToggleGenre("rpg") with { Install = InstallFilter.NotInstalled });

        Assert.Equal(["avowed", "clair", "forza5", "silksong", "indiana"], Ids(sources)); // "Hollow" antes de "Indiana"
        Assert.Equal(["clair", "silksong"], Ids(favoriteSources));
        Assert.Equal(["phantom"], Ids(rpgNeverPlayed)); // gênero sem diferenciar maiúsculas
        Assert.Equal(2, xboxOrGog.ActiveFilterCount);
        Assert.Equal(1, xboxOrGog.ToggleSource("xbox").ActiveFilterCount); // desmarcar
    }

    [Fact]
    public void Played_only_matches_the_home_recently_played_row()
    {
        var games = Games();
        var played = LibraryQuery.Apply(games, LibraryFilter.All, LibraryCriteria.None with { PlayedOnly = true });

        Assert.Equal(Ids(LibraryQuery.RecentlyPlayed(games)).Order(), Ids(played).Order());
        Assert.DoesNotContain(played, g => g.LastPlayedAt is null);
    }

    [Fact]
    public void Clearing_filters_keeps_search_and_sort()
    {
        var criteria = (LibraryCriteria.None with { Search = "god", Sort = LibrarySort.Rating, FavoritesOnly = true, PlayedOnly = true })
            .NextInstall().ToggleSource("Steam").ToggleGenre("Ação");
        Assert.Equal(5, criteria.ActiveFilterCount);

        var cleared = criteria.ClearFilters();

        Assert.False(cleared.HasFilters);
        Assert.Equal("god", cleared.Search);
        Assert.Equal(LibrarySort.Rating, cleared.Sort);
    }

    [Fact]
    public void Sorts_by_name_last_played_play_time_rating_and_date_added()
    {
        var games = Games();
        foreach (var (id, score) in new[] { ("ragnarok", 94), ("clair", 92), ("avowed", 80) })
        {
            games.Single(g => g.Id == id).MetacriticScore = score;
        }

        List<string> Sorted(LibrarySort sort) => Ids(LibraryQuery.Apply(games, LibraryFilter.All, LibraryCriteria.None with { Sort = sort }));

        var byName = Sorted(LibrarySort.Name);
        Assert.Equal("first-light", byName[0]); // "007" vem antes das letras
        Assert.Equal(["first-light", "avowed", "bf6", "bloodborne"], byName.Take(4));
        Assert.Equal("control", Sorted(LibrarySort.RecentlyPlayed)[0]);
        Assert.Equal(["avowed", "ds2", "phantom"], Sorted(LibrarySort.RecentlyPlayed).TakeLast(3)); // nunca jogados, por nome
        Assert.Equal("forza5", Sorted(LibrarySort.PlayTime)[0]);
        Assert.Equal(["ragnarok", "clair", "avowed"], Sorted(LibrarySort.Rating).Take(3)); // sem nota no fim
        Assert.Equal("control", Sorted(LibrarySort.RecentlyAdded)[0]);
    }

    [Fact]
    public void Name_sort_ignores_case_and_accents()
    {
        string[] titles = ["Écho", "eagle", "Zelda", "ábaco"];

        var sorted = titles.Order(LibraryQuery.TitleComparer).ToList();

        Assert.Equal(["ábaco", "eagle", "Écho", "Zelda"], sorted);
    }

    [Fact]
    public void Facets_list_present_sources_and_genres_with_counts()
    {
        var games = Games();

        var sources = LibraryQuery.Sources(games, LibraryFacts.Of);
        var genres = LibraryQuery.Genres(games, LibraryFacts.Of);

        Assert.Equal(["Adicionado", "Epic Games", "GOG", "Steam", "Xbox"], sources.Select(s => s.Name));
        Assert.Equal(10, sources.Single(s => s.Name == "Steam").Count);
        Assert.Equal(games.Count, sources.Sum(s => s.Count));
        Assert.Contains(genres, g => g.Name == "RPG" && g.Count == 5);
        Assert.Equal(genres.OrderBy(g => g.Name, LibraryQuery.TitleComparer).Select(g => g.Name), genres.Select(g => g.Name));
    }

    [Fact]
    public void Generic_apply_filters_any_item_type_with_the_same_rules()
    {
        (string Name, bool Installed)[] items = [("Pokémon", true), ("Pikmin", false), ("Zelda", true)];
        LibraryFacts Facts((string Name, bool Installed) i) => new(i.Name, i.Installed, false, "Playnite", [], null, TimeSpan.Zero, null, default);

        var result = LibraryQuery.Apply(items, Facts, LibraryCriteria.None with { Search = "POKE", Sort = LibrarySort.Name });
        var installed = LibraryQuery.Apply(items, Facts, LibraryCriteria.None with { Install = InstallFilter.Installed, Sort = LibrarySort.Name });

        Assert.Equal(["Pokémon"], result.Select(i => i.Name));
        Assert.Equal(["Pokémon", "Zelda"], installed.Select(i => i.Name));
    }

    [Fact]
    public void Sort_options_cycle_in_panel_order()
    {
        var sort = LibraryText.SortOrder[0];
        var seen = new List<LibrarySort>();
        for (int i = 0; i < LibraryText.SortOrder.Count; i++)
        {
            seen.Add(sort);
            sort = LibraryText.NextSort(sort);
        }

        Assert.Equal(LibraryText.SortOrder, seen);
        Assert.Equal(LibraryText.SortOrder[0], sort);
        Assert.Equal(Enum.GetValues<LibrarySort>().Order(), LibraryText.SortOrder.Order());
        Assert.Equal("Nota", LibraryText.SortLabel(LibrarySort.Rating));
        Assert.Equal("Filtros (2)", LibraryText.FiltersLabel(2));
        Assert.Equal("Nenhum jogo", LibraryText.GameCount(0));
    }

    [Fact]
    public void Virtual_keyboard_types_deletes_and_clears()
    {
        var keys = VirtualKeyboard.Rows.SelectMany(r => r).ToList();
        VirtualKey Key(string label) => keys.Single(k => k.Label == label);

        string text = string.Empty;
        foreach (string label in new[] { "Espaço", "G", "O", "W", "Espaço", "Espaço", "3" })
        {
            text = VirtualKeyboard.Apply(text, Key(label));
        }

        Assert.Equal("gow 3", text); // sem espaço no início nem espaço repetido
        text = VirtualKeyboard.Apply(text, Key("Apagar"));
        Assert.Equal("gow ", text);
        Assert.Equal(text, VirtualKeyboard.Apply(text, Key("Pronto")));
        Assert.Equal(string.Empty, VirtualKeyboard.Apply(text, Key("Limpar")));
        Assert.Equal(string.Empty, VirtualKeyboard.Backspace(string.Empty));
    }

    [Fact]
    public void Virtual_keyboard_rows_fill_the_ten_column_grid()
    {
        Assert.All(VirtualKeyboard.Rows, row => Assert.Equal(VirtualKeyboard.Columns, row.Sum(k => k.Width)));
        Assert.Equal(VirtualKeyboard.Rows.SelectMany(r => r).Count(), VirtualKeyboard.Rows.SelectMany(r => r).Select(k => k.Label).Distinct().Count());
    }

    [Fact]
    public void Physical_typing_ignores_control_characters_and_respects_the_limit()
    {
        Assert.Equal("pokémon", VirtualKeyboard.Type("pok", "é\r\nmon\b"));
        Assert.Equal("a b", VirtualKeyboard.Type("a", "\t b"));
        Assert.Equal(VirtualKeyboard.MaxLength, VirtualKeyboard.Type(new string('x', 59), "yyyy").Length);
        Assert.Equal("a", VirtualKeyboard.Backspace("a😀"));
    }
}
