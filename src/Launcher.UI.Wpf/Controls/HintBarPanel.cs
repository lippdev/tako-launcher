using System.Windows;
using System.Windows.Controls;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.ViewModels;

namespace Launcher.UI.Wpf.Controls;

/// <summary>
/// Painel da barra de dicas (ADR-0020): alinha as dicas à direita e, quando elas não cabem na largura (telas 4:3 e
/// 16:10, escala alta, teclado com "Shift+Tab"), abrevia todas (<see cref="IsCompactProperty"/>) e depois esconde as
/// menos importantes (<see cref="HintViewModel.Priority"/>). Nenhuma dica é cortada no meio.
/// </summary>
public sealed class HintBarPanel : Panel
{
    /// <summary>Herdado pela dica: o template troca o texto pelo curto e aproxima as dicas.</summary>
    public static readonly DependencyProperty IsCompactProperty = DependencyProperty.RegisterAttached(
        "IsCompact", typeof(bool), typeof(HintBarPanel), new FrameworkPropertyMetadata(false, FrameworkPropertyMetadataOptions.Inherits | FrameworkPropertyMetadataOptions.AffectsMeasure));

    private HashSet<int> _hidden = [];

    public HintBarPanel()
    {
        ClipToBounds = true; // dicas escondidas ficam fora do painel, sem desenho nem clique
    }

    /// <summary>Quantas dicas estão visíveis agora (teste de fumaça).</summary>
    public int VisibleCount => InternalChildren.Count - _hidden.Count;

    public static bool GetIsCompact(DependencyObject d) => (bool)d.GetValue(IsCompactProperty);

    public static void SetIsCompact(DependencyObject d, bool value) => d.SetValue(IsCompactProperty, value);

    protected override Size MeasureOverride(Size availableSize)
    {
        var children = InternalChildren.Cast<UIElement>().ToList();
        var infinite = new Size(double.PositiveInfinity, double.PositiveInfinity);
        var full = Widths(children, compact: false, infinite);
        var compact = Widths(children, compact: true, infinite);
        var priority = children.Select(c => (c as FrameworkElement)?.DataContext is HintViewModel hint ? hint.Priority : 0).ToList();
        var layout = double.IsInfinity(availableSize.Width) ? HintLayout.Full : HintFit.Plan(full, compact, priority, availableSize.Width);

        _hidden = [.. layout.Hidden];
        double width = 0;
        double height = 0;
        for (int i = 0; i < children.Count; i++)
        {
            SetIsCompact(children[i], layout.Compact);
            children[i].Measure(infinite);
            if (!_hidden.Contains(i))
            {
                width += children[i].DesiredSize.Width;
                height = Math.Max(height, children[i].DesiredSize.Height);
            }
        }

        return new Size(double.IsInfinity(availableSize.Width) ? width : Math.Min(width, availableSize.Width), height);
    }

    protected override Size ArrangeOverride(Size finalSize)
    {
        var children = InternalChildren.Cast<UIElement>().ToList();
        double total = children.Where((_, i) => !_hidden.Contains(i)).Sum(c => c.DesiredSize.Width);
        double x = Math.Max(0, finalSize.Width - total);
        for (int i = 0; i < children.Count; i++)
        {
            var child = children[i];
            if (_hidden.Contains(i))
            {
                child.Arrange(new Rect(-100000, 0, child.DesiredSize.Width, child.DesiredSize.Height));
                continue;
            }

            child.Arrange(new Rect(x, (finalSize.Height - child.DesiredSize.Height) / 2, child.DesiredSize.Width, child.DesiredSize.Height));
            x += child.DesiredSize.Width;
        }

        return finalSize;
    }

    private static List<double> Widths(List<UIElement> children, bool compact, Size infinite)
    {
        var widths = new List<double>(children.Count);
        foreach (var child in children)
        {
            SetIsCompact(child, compact);
            child.Measure(infinite);
            widths.Add(child.DesiredSize.Width);
        }

        return widths;
    }
}
