using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Media;
using Launcher.Core.Navigation;
using Launcher.PlayniteCompat.P10.Controls;
using Launcher.UI.Wpf.Controls;
using Launcher.UI.Wpf.Navigation;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    private const int LargeLibrary = 2000;

    /// <summary>
    /// Frente D (PERF-004): grades virtualizadas com 2000 itens. Na grade do Tako (rolagem por <c>TranslateTransform</c>,
    /// como as páginas) só existem as linhas perto da tela; descer 30 linhas pelo controle, com a rolagem animada, sempre
    /// acha a linha de baixo; o foco restaurado num item lá embaixo cria o elemento dele. Na grade dos temas Playnite
    /// (<see cref="P10TilePanel"/> num ListBox), idem com o <c>ScrollViewer</c>.
    /// </summary>
    private static void LargeLibraryVirtualization()
    {
        var tiles = Enumerable.Range(0, LargeLibrary).Select(i => new Tile($"t{i}")).ToList();

        // Mesma estrutura da página da biblioteca: palco 1920x1080 → viewport (Canvas) → host rolável → grade.
        var button = new FrameworkElementFactory(typeof(Button));
        button.SetValue(FrameworkElement.WidthProperty, 204.0);
        button.SetValue(FrameworkElement.HeightProperty, 400.0);
        button.SetValue(FrameworkElement.HorizontalAlignmentProperty, HorizontalAlignment.Left);
        button.SetValue(Nav.GroupProperty, "grid");
        button.SetBinding(Nav.KeyProperty, new Binding(nameof(Tile.NavKey)));
        var panel = new FrameworkElementFactory(typeof(VirtualTilePanel));
        panel.SetValue(VirtualTilePanel.ItemWidthProperty, 240.0);
        panel.SetValue(VirtualTilePanel.ItemHeightProperty, 440.0);
        var grid = new ItemsControl
        {
            ItemsSource = tiles,
            ItemTemplate = new DataTemplate { VisualTree = button },
            ItemsPanel = new ItemsPanelTemplate(panel),
            Margin = new Thickness(64, 48, 72, 200),
            Focusable = false,
        };
        VirtualizingPanel.SetVirtualizationMode(grid, VirtualizationMode.Recycling);
        var frozen = new TranslateTransform();
        frozen.Freeze(); // como nos templates: a primeira rolagem troca por uma animável
        var host = new StackPanel { Width = 1920, RenderTransform = frozen };
        Nav.SetScrollHost(host, "Vertical");
        host.Children.Add(new Border { Height = 300 }); // cabeçalho da página
        host.Children.Add(grid);
        var viewport = new Canvas { Width = 1920, Height = 1080, ClipToBounds = true };
        viewport.Children.Add(host);
        var stage = new Grid { Width = 1920, Height = 1080 };
        stage.Children.Add(viewport);

        var window = new Window { Width = 640, Height = 360, Content = new Viewbox { Child = stage }, Title = "Tako · virtualização" };
        window.Show();
        window.Activate();
        Pump(0.5);
        var duration = TimeSpan.FromMilliseconds(120);
        var nav = new FocusNavigator(stage, () => [stage], () => duration);

        int columns = (1920 - 64 - 72) / 240; // 7
        int realized = nav.Navigables().Count;
        Assert.InRange(realized, columns, columns * 10);

        Assert.True(nav.FocusDefault(null));
        Assert.Equal("t0", Nav.GetKey(nav.Focused!));
        for (int i = 0; i < 30; i++)
        {
            Assert.True(nav.Move(NavDirection.Down), $"↓ {i + 1}: a linha de baixo não existia");
            Pump(0.14); // direcional segurado: o próximo comando chega antes do fim da animação
        }

        Pump(0.3);
        Assert.Equal($"t{30 * columns}", Nav.GetKey(nav.Focused!));
        Assert.InRange(nav.Navigables().Count, columns, columns * 10);

        // Volta dos detalhes para um item fora da tela (o elemento dele não existe até ser pedido).
        Assert.DoesNotContain(nav.Navigables(), e => Nav.GetKey(e) == "t1990");
        Assert.True(nav.FocusDefault("grid:t1990"));
        Assert.Equal("t1990", Nav.GetKey(nav.Focused!));
        Pump(0.5);
        Assert.True(nav.Move(NavDirection.Up));
        Assert.Equal($"t{1990 - columns}", Nav.GetKey(nav.Focused!));
        Assert.InRange(nav.Navigables().Count, columns, columns * 10);
        window.Close();

        // Grade dos temas Playnite: ListBox padrão (ScrollViewer com rolagem lógica) + P10TilePanel.
        var list = new ListBox
        {
            ItemsSource = tiles,
            ItemsPanel = new ItemsPanelTemplate(new FrameworkElementFactory(typeof(P10TilePanel))),
            Width = 1200,
            Height = 700,
        };
        ScrollViewer.SetHorizontalScrollBarVisibility(list, ScrollBarVisibility.Disabled);
        var themeWindow = new Window { Width = 640, Height = 360, Content = new Viewbox { Child = list }, Title = "Tako · virtualização P10" };
        themeWindow.Show();
        Pump(0.5);
        int themeRealized = Enumerable.Range(0, LargeLibrary).Count(i => list.ItemContainerGenerator.ContainerFromIndex(i) is not null);
        Assert.InRange(themeRealized, 7, 7 * 10);
        Assert.Null(list.ItemContainerGenerator.ContainerFromIndex(1500));
        list.ScrollIntoView(tiles[1500]);
        Pump(0.5);
        Assert.NotNull(list.ItemContainerGenerator.ContainerFromIndex(1500));
        Assert.InRange(Enumerable.Range(0, LargeLibrary).Count(i => list.ItemContainerGenerator.ContainerFromIndex(i) is not null), 7, 7 * 10);
        themeWindow.Close();
    }

    private sealed record Tile(string NavKey) : INavKeyed;
}
