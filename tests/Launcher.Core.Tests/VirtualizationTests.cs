using Launcher.Core.Presentation;
using Xunit;

namespace Launcher.Core.Tests;

public class VirtualizationTests
{
    // Grade da biblioteca: capas de 240 × 440, sete colunas num palco 16:9.
    private static readonly VirtualGrid Library = new(1000, 7, 240, 440);

    [Theory]
    [InlineData(1784, 240, 1000, 7)]
    [InlineData(1680, 240, 1000, 7)] // múltiplo exato: cabe, como no WrapPanel
    [InlineData(1679.9, 240, 1000, 6)]
    [InlineData(100, 240, 1000, 1)] // pelo menos uma coluna
    [InlineData(double.PositiveInfinity, 230, 12, 12)] // sem limite: uma linha só
    public void Columns_wrap_like_the_wrap_panel(double available, double itemWidth, int count, int expected)
    {
        Assert.Equal(expected, VirtualGrid.ColumnsFor(available, itemWidth, count));
    }

    [Fact]
    public void Extent_covers_every_item_even_when_only_a_few_exist()
    {
        Assert.Equal(143, Library.Rows);
        Assert.Equal(7 * 240, Library.Width);
        Assert.Equal(143 * 440, Library.Height);
        Assert.Equal((240 * 3, 440 * 2), Library.PositionOf(17));

        var few = new VirtualGrid(3, 7, 240, 440);
        Assert.Equal(3 * 240, few.Width); // só as colunas usadas
        Assert.Equal(440, few.Height);
    }

    [Fact]
    public void Only_rows_touching_the_viewport_plus_the_buffer_are_realized()
    {
        // Rolada até a linha 10 (4400), tela de 1080 de altura: linhas 10 a 12 visíveis, 2 de folga de cada lado.
        var range = Library.Visible(0, 4400, 1680, 4400 + 1080, bufferLines: 2);

        Assert.Equal(8 * 7, range.First);
        Assert.Equal((15 * 7) - 1, range.Last);
        Assert.Equal(7 * 7, range.Count);
    }

    [Fact]
    public void Viewport_at_the_top_or_bottom_is_clamped_to_the_grid()
    {
        var top = Library.Visible(0, -300, 1680, 780, bufferLines: 2);
        Assert.Equal(0, top.First);
        Assert.Equal((4 * 7) - 1, top.Last);

        var bottom = Library.Visible(0, Library.Height - 500, 1680, Library.Height + 580, bufferLines: 2);
        Assert.Equal(999, bottom.Last);
        Assert.Equal((141 - 2) * 7, bottom.First);
    }

    [Fact]
    public void Viewport_outside_the_grid_keeps_only_the_buffer_near_the_edge()
    {
        Assert.Equal(new ItemRange(0, 13), Library.Visible(0, -5000, 1680, -3000, bufferLines: 2));
        Assert.Equal(new ItemRange(141 * 7, 999), Library.Visible(0, Library.Height + 10, 1680, Library.Height + 1000, bufferLines: 2));
        Assert.True(Library.Visible(0, -5000, 1680, -3000, bufferLines: 0).IsEmpty);
    }

    [Fact]
    public void Single_row_realizes_by_column()
    {
        // Fileira da Início: 230 por capa, 40 jogos, trilho rolado até a 10ª capa, tela de 1800 de largura.
        var row = new VirtualGrid(40, 40, 230, 450);
        var range = row.Visible(2300, 0, 2300 + 1800, 450, bufferLines: 3);

        Assert.Equal(7, range.First);
        Assert.Equal(17 + 3, range.Last);
        Assert.True(row.Visible(0, 0, 0, 450, 3).IsEmpty);
    }

    [Fact]
    public void Empty_grid_has_nothing_to_realize()
    {
        var empty = new VirtualGrid(0, 7, 240, 440);
        Assert.True(empty.Visible(0, 0, 1680, 1080, 2).IsEmpty);
        Assert.Equal(0, empty.Height);
    }

    [Fact]
    public void Range_reports_its_size_and_membership()
    {
        var range = new ItemRange(10, 40);
        Assert.Equal(31, range.Count);
        Assert.True(range.Contains(10));
        Assert.False(range.Contains(41));
        Assert.Equal(0, ItemRange.Empty.Count);
        Assert.False(ItemRange.Empty.Contains(0));
    }

    [Theory]
    [InlineData(0, 1000, 200, 300, 0)] // visível: não mexe
    [InlineData(500, 1000, 200, 300, 200)] // acima: alinha em cima
    [InlineData(0, 1000, 900, 300, 200)] // abaixo: alinha embaixo
    [InlineData(0, 500, 900, 800, 900)] // maior que a área, abaixo: alinha em cima
    public void Minimal_scroll_matches_the_wpf_scroll_presenter(double viewStart, double viewSize, double itemStart, double itemSize, double expected)
    {
        Assert.Equal(expected, VirtualGrid.MinimalScroll(viewStart, viewSize, itemStart, itemSize));
    }

    [Fact]
    public void Lru_evicts_least_recently_used_until_under_capacity()
    {
        var cache = new SizedLruCache<string, string>(capacity: 10);
        cache.Add("a", "A", 4);
        cache.Add("b", "B", 4);
        Assert.True(cache.TryGet("a", out _)); // "a" vira o mais recente

        var evicted = cache.Add("c", "C", 4);

        Assert.Equal(["B"], evicted);
        Assert.False(cache.TryGet("b", out _));
        Assert.True(cache.TryGet("a", out var a));
        Assert.Equal("A", a);
        Assert.Equal(8, cache.TotalCost);
        Assert.Equal(2, cache.Count);
    }

    [Fact]
    public void Lru_keeps_the_newest_item_even_above_capacity()
    {
        var cache = new SizedLruCache<int, int>(capacity: 10);
        cache.Add(1, 1, 3);

        var evicted = cache.Add(2, 2, 50);

        Assert.Equal([1], evicted);
        Assert.True(cache.TryGet(2, out _));
        Assert.Equal(50, cache.TotalCost);
    }

    [Fact]
    public void Lru_replacing_a_key_updates_its_cost()
    {
        var cache = new SizedLruCache<int, string>(capacity: 100);
        cache.Add(1, "x", 10);
        cache.Add(1, "y", 30);

        Assert.Equal(30, cache.TotalCost);
        Assert.Equal(1, cache.Count);
        Assert.True(cache.Remove(1));
        Assert.Equal(0, cache.TotalCost);
        Assert.False(cache.Remove(1));
    }
}
