using System.Collections.Specialized;
using System.ComponentModel;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Media;
using System.Windows.Threading;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Navigation;

namespace Launcher.UI.Wpf.Controls;

/// <summary>
/// Painel virtualizado das capas (PERF-004): a grade da biblioteca (<see cref="Orientation.Vertical"/>, quebra de linha
/// como o <c>WrapPanel</c>) e as fileiras da Início (<see cref="Orientation.Horizontal"/>, como o <c>StackPanel</c>).
/// O tamanho é o da lista inteira, então o layout e a rolagem da tela não mudam; só existem os elementos das capas que
/// tocam a área visível, com algumas linhas de folga para a navegação achar o vizinho. A rolagem do Tako não usa
/// <c>ScrollViewer</c>: o conteúdo anda por um <c>TranslateTransform</c> no elemento marcado com
/// <c>Nav.ScrollHost</c> (ver <see cref="FocusNavigator"/>), e o painel acompanha essa transformação. Com
/// <c>VirtualizingPanel.VirtualizationMode="Recycling"</c>, os elementos que saem da tela são reaproveitados.
/// </summary>
public sealed class VirtualTilePanel : VirtualizingPanel, IVirtualNavHost
{
    public static readonly DependencyProperty ItemWidthProperty =
        DependencyProperty.Register(nameof(ItemWidth), typeof(double), typeof(VirtualTilePanel), new FrameworkPropertyMetadata(240.0, FrameworkPropertyMetadataOptions.AffectsMeasure));

    /// <summary>Altura de cada item; sem valor (NaN), a do item mais alto já medido.</summary>
    public static readonly DependencyProperty ItemHeightProperty =
        DependencyProperty.Register(nameof(ItemHeight), typeof(double), typeof(VirtualTilePanel), new FrameworkPropertyMetadata(double.NaN, FrameworkPropertyMetadataOptions.AffectsMeasure));

    public static readonly DependencyProperty OrientationProperty =
        DependencyProperty.Register(nameof(Orientation), typeof(Orientation), typeof(VirtualTilePanel), new FrameworkPropertyMetadata(Orientation.Vertical, FrameworkPropertyMetadataOptions.AffectsMeasure));

    /// <summary>Folga: linhas da grade (ou capas da fileira) criadas além da área visível, de cada lado.</summary>
    private const int GridBufferRows = 2;
    private const int RowBufferItems = 3;

    private static readonly DependencyPropertyDescriptor RenderTransformDescriptor =
        DependencyPropertyDescriptor.FromProperty(RenderTransformProperty, typeof(UIElement));

    private FrameworkElement? _host;
    private FrameworkElement? _viewport;
    private Transform? _watched;
    private VirtualGrid _grid;
    private ItemRange _realized = ItemRange.Empty;
    private double _measuredItemHeight;
    private int _pinned = -1;
    private bool _checkScheduled;

    public VirtualTilePanel()
    {
        Loaded += (_, _) => FindHost();
        Unloaded += (_, _) => Detach();
    }

    public double ItemWidth
    {
        get => (double)GetValue(ItemWidthProperty);
        set => SetValue(ItemWidthProperty, value);
    }

    public double ItemHeight
    {
        get => (double)GetValue(ItemHeightProperty);
        set => SetValue(ItemHeightProperty, value);
    }

    public Orientation Orientation
    {
        get => (Orientation)GetValue(OrientationProperty);
        set => SetValue(OrientationProperty, value);
    }

    /// <summary>Foco restaurado num jogo fora da tela (volta dos detalhes): cria o elemento dele.</summary>
    public bool RealizeKey(string key)
    {
        ArgumentNullException.ThrowIfNull(key);
        if (ItemsControl.GetItemsOwner(this) is not { } owner)
        {
            return false;
        }

        for (int i = 0; i < owner.Items.Count; i++)
        {
            if (owner.Items[i] is INavKeyed item && (key == item.NavKey || key.EndsWith(":" + item.NavKey, StringComparison.Ordinal)))
            {
                if (_realized.Contains(i) || _pinned == i)
                {
                    return false; // já existe; o problema não é a virtualização
                }

                _pinned = i;
                InvalidateMeasure();
                return true;
            }
        }

        return false;
    }

    protected override Size MeasureOverride(Size availableSize)
    {
        if (_host is null)
        {
            FindHost();
        }

        var owner = ItemsControl.GetItemsOwner(this);
        int count = owner?.Items.Count ?? 0;
        double itemWidth = Math.Max(1, ItemWidth);
        bool row = Orientation == Orientation.Horizontal;
        double itemHeight = double.IsNaN(ItemHeight) ? _measuredItemHeight : ItemHeight;
        int columns = row ? count : VirtualGrid.ColumnsFor(availableSize.Width, itemWidth, count);
        _grid = new VirtualGrid(count, columns, itemWidth, itemHeight);

        _realized = VisibleRange(owner);
        Realize(_realized, _pinned < count ? _pinned : -1);

        var childSize = new Size(itemWidth, double.IsNaN(ItemHeight) ? (row ? availableSize.Height : double.PositiveInfinity) : ItemHeight);
        foreach (UIElement child in InternalChildren)
        {
            child.Measure(childSize);
            if (double.IsNaN(ItemHeight))
            {
                _measuredItemHeight = Math.Max(_measuredItemHeight, child.DesiredSize.Height);
            }
        }

        if (double.IsNaN(ItemHeight) && _measuredItemHeight != itemHeight)
        {
            _grid = new VirtualGrid(count, columns, itemWidth, _measuredItemHeight);
        }

        return new Size(_grid.Width, _grid.Height);
    }

    protected override Size ArrangeOverride(Size finalSize)
    {
        var generator = ItemContainerGenerator;
        bool row = Orientation == Orientation.Horizontal;
        for (int i = 0; i < InternalChildren.Count; i++)
        {
            int index = generator.IndexFromGeneratorPosition(new GeneratorPosition(i, 0));
            var (x, y) = _grid.PositionOf(index);
            InternalChildren[i].Arrange(new Rect(x, y, _grid.ItemWidth, row ? finalSize.Height : _grid.ItemHeight));
        }

        // A posição do painel na tela só vale depois deste arranjo: confere a faixa visível logo depois do layout.
        if (!_checkScheduled)
        {
            _checkScheduled = true;
            Dispatcher.BeginInvoke(() =>
            {
                _checkScheduled = false;
                CheckRange();
            }, DispatcherPriority.Loaded);
        }

        return finalSize;
    }

    protected override void OnItemsChanged(object sender, ItemsChangedEventArgs args)
    {
        ArgumentNullException.ThrowIfNull(args);
        switch (args.Action)
        {
            case NotifyCollectionChangedAction.Remove:
            case NotifyCollectionChangedAction.Replace:
            case NotifyCollectionChangedAction.Move:
                RemoveInternalChildRange(args.Position.Index, args.ItemUICount);
                break;
        }

        _pinned = -1;
        base.OnItemsChanged(sender, args);
        InvalidateMeasure();
    }

    protected override void OnClearChildren()
    {
        base.OnClearChildren();
        _realized = ItemRange.Empty;
    }

    /// <summary>Itens que tocam a área visível do palco (coordenadas deste painel) mais a folga.</summary>
    private ItemRange VisibleRange(ItemsControl? owner)
    {
        var all = _grid.Count == 0 ? ItemRange.Empty : new ItemRange(0, _grid.Count - 1);
        if (owner is null || !GetIsVirtualizing(owner) || _viewport is null || !_viewport.IsAncestorOf(this))
        {
            return all; // fora do layout de rolagem do Tako: cria tudo, como o painel antigo
        }

        Rect visible;
        try
        {
            if (_viewport.TransformToDescendant(this) is not { } transform)
            {
                return all;
            }

            visible = transform.TransformBounds(new Rect(_viewport.RenderSize));
        }
        catch (InvalidOperationException)
        {
            return all;
        }

        int buffer = _grid.Rows == 1 ? RowBufferItems : GridBufferRows;
        return _grid.Visible(visible.Left, visible.Top, visible.Right, visible.Bottom, buffer);
    }

    /// <summary>
    /// Cria os elementos da faixa (e do item <paramref name="pinned"/>, pedido pela navegação) e devolve, para
    /// reaproveitar, os de fora; o elemento com foco sempre fica.
    /// </summary>
    private void Realize(ItemRange range, int pinned)
    {
        var children = InternalChildren; // garante o gerador ligado ao painel
        var generator = ItemContainerGenerator;
        var recycling = ItemsControl.GetItemsOwner(this) is { } owner && GetVirtualizationMode(owner) == VirtualizationMode.Recycling
            ? generator as IRecyclingItemContainerGenerator
            : null;

        for (int i = children.Count - 1; i >= 0; i--)
        {
            var position = new GeneratorPosition(i, 0);
            int index = generator.IndexFromGeneratorPosition(position);
            if (range.Contains(index) || index == pinned || children[i].IsKeyboardFocusWithin)
            {
                continue;
            }

            if (recycling is not null)
            {
                recycling.Recycle(position, 1);
            }
            else
            {
                generator.Remove(position, 1);
            }

            RemoveInternalChildRange(i, 1);
        }

        Generate(range, generator);
        if (pinned >= 0 && !range.Contains(pinned))
        {
            Generate(new ItemRange(pinned, pinned), generator);
        }
    }

    private void Generate(ItemRange range, IItemContainerGenerator generator)
    {
        if (range.IsEmpty)
        {
            return;
        }

        var children = InternalChildren;
        var start = generator.GeneratorPositionFromIndex(range.First);
        int childIndex = start.Offset == 0 ? start.Index : start.Index + 1;
        using (generator.StartAt(start, GeneratorDirection.Forward, allowStartAtRealizedItem: true))
        {
            for (int index = range.First; index <= range.Last; index++, childIndex++)
            {
                if (generator.GenerateNext(out _) is not UIElement child)
                {
                    break;
                }

                if (childIndex < children.Count && ReferenceEquals(children[childIndex], child))
                {
                    continue; // já estava na tela
                }

                // Novo ou reaproveitado: entra na posição da ordem dos itens.
                if (childIndex >= children.Count)
                {
                    AddInternalChild(child);
                }
                else
                {
                    InsertInternalChild(childIndex, child);
                }

                generator.PrepareItemContainer(child);
            }
        }
    }

    private void CheckRange()
    {
        if (VisibleRange(ItemsControl.GetItemsOwner(this)) != _realized)
        {
            InvalidateMeasure();
        }
    }

    private void OnScrolled(object? sender, EventArgs e)
    {
        _pinned = -1; // o item pedido já recebeu o foco (e o foco segura o elemento)
        CheckRange();
    }

    private void FindHost()
    {
        for (var d = VisualTreeHelper.GetParent(this); d is not null; d = VisualTreeHelper.GetParent(d))
        {
            if (d is FrameworkElement host && Nav.GetScrollHost(host) is not null)
            {
                if (ReferenceEquals(host, _host))
                {
                    return;
                }

                Detach();
                _host = host;
                _viewport = VisualTreeHelper.GetParent(host) as FrameworkElement;
                RenderTransformDescriptor.AddValueChanged(host, OnHostTransformReplaced);
                if (_viewport is not null)
                {
                    _viewport.SizeChanged += OnScrolled;
                }

                Watch(host.RenderTransform);
                return;
            }
        }
    }

    private void Detach()
    {
        if (_host is not null)
        {
            RenderTransformDescriptor.RemoveValueChanged(_host, OnHostTransformReplaced);
        }

        if (_viewport is not null)
        {
            _viewport.SizeChanged -= OnScrolled;
        }

        Watch(null);
        _host = null;
        _viewport = null;
    }

    /// <summary>A transformação do template vem congelada; a primeira rolagem troca por uma animável.</summary>
    private void OnHostTransformReplaced(object? sender, EventArgs e)
    {
        Watch(_host?.RenderTransform);
        OnScrolled(sender, e);
    }

    /// <summary><c>Changed</c> também dispara a cada passo da animação de rolagem.</summary>
    private void Watch(Transform? transform)
    {
        if (_watched is not null)
        {
            _watched.Changed -= OnScrolled;
        }

        _watched = transform is { IsFrozen: false } ? transform : null;
        if (_watched is not null)
        {
            _watched.Changed += OnScrolled;
        }
    }
}
