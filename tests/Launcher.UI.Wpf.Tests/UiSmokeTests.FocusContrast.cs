using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Shapes;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Controls;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

/// <summary>
/// Linha focada (MVP-029, UX-009): a linha focada vira branca (surface-solid) e tudo dentro dela — chave, marcador,
/// contagem, valor, seta, ícone — continua visível, com contraste WCAG AA. Na alpha.10 a chave branca sumia na linha
/// branca e a última linha do painel de filtros ficava por baixo da barra de dicas.
/// <para>
/// A verificação lê as cores reais da árvore visual (pincéis resolvidos, opacidades e fundos compostos) e roda em toda
/// captura (<see cref="Snap"/>): qualquer tela cujo item focado tenha superfície sólida é conferida.
/// </para>
/// </summary>
public sealed partial class UiSmokeTests
{
    /// <summary>Filtros com uma linha focada de cada tipo: chave ligada e desligada, marcador, contagem e valor.</summary>
    private static void FocusedRowsInFilters(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        shell.OpenLibrary(LibraryFilter.All);
        Pump(1);
        shell.OpenLibraryFilters();
        Pump(0.8);
        var filters = Assert.IsType<LibraryFiltersViewModel>(shell.TopOverlay);
        var rows = filters.Sections.SelectMany(s => s.Rows).ToList();
        var genres = filters.Sections.Single(s => s.Title == "Gênero").Rows;
        genres[0].Command.Execute(null); // um gênero ligado: chave ligada com contagem
        Assert.True(genres[0].IsOn);

        (string Name, PanelRowViewModel Row)[] cases =
        [
            ("chave-ligada", genres[0]),
            ("chave-desligada", genres.First(r => !r.IsOn)),
            ("opcao-escolhida", rows.First(r => r.IsRadio && r.IsOn)),
            ("opcao", rows.First(r => r.IsRadio && !r.IsOn)),
            ("valor", rows.First(r => r.Label == "Instalação")),
            ("ultima-linha", rows[^1]),
        ];
        foreach (var (name, row) in cases)
        {
            shell.RequestFocus($"filter:{row.Label}");
            Pump(0.6);
            Assert.Equal($"filter:{row.Label}", Nav.GetKey(nav.Focused!));
            Snap(window, $"76-filtros-foco-{name}");
            AssertFocusedRowReadable(window, $"Filtros › {row.Label}", requireSolid: true);
            AssertAboveHintBar(window, $"Filtros › {row.Label}");
        }

        genres[0].Command.Execute(null);
        Press(window, AppCommand.Back);
        Pump(0.6);
        Assert.Null(shell.TopOverlay);
        shell.LibraryCriteria = LibraryCriteria.None;
    }

    /// <summary>Painel de Áudio da Central: chave (Mudo) e marcador (saída) focados na linha branca.</summary>
    private static void FocusedRowsInControlCenterAudio(MainWindow window)
    {
        var nav = window.Navigator;
        Assert.Equal("audio:volume", Nav.GetKey(nav.Focused!));
        AssertFocusedRowReadable(window, "Central › Volume", requireSolid: true);
        Press(window, AppCommand.Down);
        Assert.Equal("audio:mute", Nav.GetKey(nav.Focused!));
        Pump(0.3);
        Snap(window, "11a-painel-audio-chave");
        AssertFocusedRowReadable(window, "Central › Mudo", requireSolid: true);
        Press(window, AppCommand.Down);
        Assert.StartsWith("audio:output:", Nav.GetKey(nav.Focused!), StringComparison.Ordinal);
        Pump(0.3);
        Snap(window, "11b-painel-audio-saida");
        AssertFocusedRowReadable(window, "Central › Saída de áudio", requireSolid: true);
        Press(window, AppCommand.Up);
        Press(window, AppCommand.Up);
        Assert.Equal("audio:volume", Nav.GetKey(nav.Focused!));
    }

    /// <summary>
    /// Confere o item focado quando ele é pintado com uma superfície opaca (linha de painel, categoria, item da barra
    /// lateral, pílula primária): todo texto, ícone e controle visível dentro dele contrasta com o fundo em que está.
    /// </summary>
    private static void AssertFocusedRowReadable(MainWindow window, string screen, bool requireSolid = false)
    {
        if (window.Navigator.Focused is not Control focused || IsThirdPartyTheme(focused))
        {
            Assert.False(requireSolid, $"{screen}: nada focado");
            return;
        }

        focused.UpdateLayout();
        if (SolidSurface(focused) is not Rgb surface)
        {
            Assert.False(requireSolid, $"{screen}: a linha focada ({Nav.GetKey(focused)}) não está com a superfície sólida");
            return;
        }

        if (requireSolid)
        {
            Assert.True(FocusSurface.GetIsSolid(focused), $"{screen}: a linha focada não avisa os controles da superfície sólida (FocusSurface.IsSolid)");
        }

        var failures = new List<string>();
        int checkedMarks = Walk(focused, surface, 1, failures);
        Assert.True(failures.Count == 0, $"{screen} ({Nav.GetKey(focused)}): contraste insuficiente na linha focada:\n{string.Join('\n', failures)}");
        Assert.True(!requireSolid || checkedMarks > 0, $"{screen}: nada conferido na linha focada");
    }

    /// <summary>O item focado e a moldura com recorte da lista terminam acima da barra de dicas.</summary>
    private static void AssertAboveHintBar(MainWindow window, string screen)
    {
        var stage = window.StageElement;
        var focused = window.Navigator.Focused!;
        var bounds = focused.TransformToAncestor(stage).TransformBounds(new Rect(focused.RenderSize));
        Assert.True(bounds.Bottom <= HintBarTop, $"{screen}: o item focado termina em {bounds.Bottom:0}, por baixo da barra de dicas ({HintBarTop})");
        for (var d = VisualTreeHelper.GetParent(focused); d is not null && d != stage; d = VisualTreeHelper.GetParent(d))
        {
            if (d is FrameworkElement { ClipToBounds: true } frame)
            {
                var clip = frame.TransformToAncestor(stage).TransformBounds(new Rect(frame.RenderSize));
                Assert.True(clip.Bottom <= HintBarTop + 0.5, $"{screen}: a lista rola por baixo da barra de dicas (moldura até {clip.Bottom:0})");
                return;
            }
        }
    }

    private static bool IsThirdPartyTheme(DependencyObject element)
    {
        for (var d = element; d is not null; d = VisualTreeHelper.GetParent(d))
        {
            if (d is ThemeScope or PlayniteMainHost)
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>Cor do corpo do item quando ele é opaco ("Body" ou o disco dos ícones da Central); senão, nada a conferir.</summary>
    private static Rgb? SolidSurface(Control control)
    {
        if (control.Template?.FindName("Body", control) is Border { IsVisible: true, Background: SolidColorBrush body } && Opaque(body))
        {
            return ToRgb(body.Color);
        }

        if (control.Template?.FindName("Disc", control) is Shape { IsVisible: true, Fill: SolidColorBrush disc } && Opaque(disc))
        {
            return ToRgb(disc.Color);
        }

        return null;
    }

    /// <summary>Percorre os filhos visíveis compondo opacidade e fundos; devolve quantas marcas (texto/forma) conferiu.</summary>
    private static int Walk(DependencyObject node, Rgb background, double opacity, List<string> failures)
    {
        int count = 0;
        for (int i = 0; i < VisualTreeHelper.GetChildrenCount(node); i++)
        {
            var child = VisualTreeHelper.GetChild(node, i);
            if (child is UIElement { IsVisible: false } || child is FrameworkElement { Name: "Ring" })
            {
                continue;
            }

            double alpha = opacity * (child is UIElement ui ? ui.Opacity : 1);
            if (alpha < 0.02)
            {
                continue;
            }

            var inner = background;
            string name = Describe(child);
            switch (child)
            {
                case FrameworkElement { Name: "Body" or "Disc" }:
                    break; // a própria superfície (já é o fundo)
                case TextBlock text when !string.IsNullOrWhiteSpace(text.Text) && text.Foreground is SolidColorBrush fg:
                    count++;
                    Require(failures, $"texto \"{text.Text}\" ({text.FontSize:0}px)", Blend(fg, alpha, background), background,
                        ContrastRules.ForText(text.FontSize, text.FontWeight.ToOpenTypeWeight()));
                    break;
                case Shape shape:
                    double fill = Ratio(shape.Fill, alpha, background);
                    double stroke = shape.StrokeThickness > 0 ? Ratio(shape.Stroke, alpha, background) : 0;
                    if (fill > 0 || stroke > 0)
                    {
                        count++;
                        Check(failures, name, Math.Max(fill, stroke), ContrastRules.Component);
                    }

                    break;
                case Border border:
                    double fillRatio = Ratio(border.Background, alpha, background);
                    double edge = border.BorderThickness.Left > 0 ? Ratio(border.BorderBrush, alpha, background) : 0;
                    if (border.Name == "Track" || edge > 0)
                    {
                        // Componente (trilho da chave, contorno): o preenchimento ou o contorno se destacam do fundo.
                        count++;
                        Check(failures, name, Math.Max(fillRatio, edge), ContrastRules.Component);
                    }

                    if (border.Background is SolidColorBrush own)
                    {
                        inner = Blend(own, alpha, background);
                    }

                    break;
                case Panel { Background: SolidColorBrush panel }:
                    inner = Blend(panel, alpha, background);
                    break;
            }

            count += Walk(child, inner, alpha, failures);
        }

        return count;
    }

    private static void Require(List<string> failures, string what, Rgb color, Rgb background, double minimum) =>
        Check(failures, $"{what} {color} sobre {background}", ContrastRules.Ratio(color, background), minimum);

    private static void Check(List<string> failures, string what, double ratio, double minimum)
    {
        if (ratio < minimum)
        {
            failures.Add($"  {what}: {ratio:0.00}:1 (mínimo {minimum}:1)");
        }
    }

    /// <summary>Contraste de um pincel sólido com o fundo; 0 quando ele não desenha nada (nulo, transparente).</summary>
    private static double Ratio(Brush? brush, double opacity, Rgb background) =>
        brush is SolidColorBrush solid && solid.Color.A * solid.Opacity * opacity >= 5
            ? ContrastRules.Ratio(Blend(solid, opacity, background), background)
            : 0;

    private static Rgb Blend(SolidColorBrush brush, double opacity, Rgb background) =>
        ContrastRules.Over(ToRgb(brush.Color), brush.Color.A / 255d * brush.Opacity * opacity, background);

    private static bool Opaque(SolidColorBrush brush) => brush.Color.A == 255 && brush.Opacity >= 1;

    private static Rgb ToRgb(Color color) => new(color.R, color.G, color.B);

    private static string Describe(DependencyObject element) =>
        element is FrameworkElement { Name.Length: > 0 } named ? $"{element.GetType().Name} \"{named.Name}\"" : element.GetType().Name;
}
