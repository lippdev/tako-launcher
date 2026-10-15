using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using System.Windows.Media;
using Launcher.Core.Presentation;

namespace Launcher.PlayniteCompat.P10.Controls;

#pragma warning disable CA1711 // Os nomes (ButtonEx...) são os que os temas Playnite referenciam; não podem mudar.

// Equivalentes dos controles do Playnite.FullscreenApp usados pelos temas Fullscreen (MVP-018/019).
// O tema define estilos e templates; o comportamento vem do adaptador (ver Main, GameDetails e FilterPresetSelector).

/// <summary>Botão do Playnite (Fullscreen). <see cref="InputHint"/> é o glifo do botão do controle ao lado do texto.</summary>
public class ButtonEx : Button
{
    public static readonly DependencyProperty InputHintProperty =
        DependencyProperty.Register(nameof(InputHint), typeof(DataTemplate), typeof(ButtonEx), new PropertyMetadata(null));

    public DataTemplate? InputHint
    {
        get => (DataTemplate?)GetValue(InputHintProperty);
        set => SetValue(InputHintProperty, value);
    }
}

/// <summary>Lista de itens do Playnite (menus e filtros). <see cref="HorizontalLayout"/> troca o painel para horizontal.</summary>
public class ItemsControlEx : ItemsControl
{
    public static readonly DependencyProperty HorizontalLayoutProperty =
        DependencyProperty.Register(nameof(HorizontalLayout), typeof(bool), typeof(ItemsControlEx), new PropertyMetadata(false, OnHorizontalLayoutChanged));

    public bool HorizontalLayout
    {
        get => (bool)GetValue(HorizontalLayoutProperty);
        set => SetValue(HorizontalLayoutProperty, value);
    }

    private static void OnHorizontalLayoutChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var panel = new FrameworkElementFactory(typeof(StackPanel));
        panel.SetValue(StackPanel.OrientationProperty, e.NewValue is true ? Orientation.Horizontal : Orientation.Vertical);
        ((ItemsControlEx)d).ItemsPanel = new ItemsPanelTemplate(panel);
    }
}

/// <summary>Lista suspensa do Playnite. O Popup da lista só é aceito no template dela (ver <c>XamlSecurityPolicy.PopupHostTemplates</c>).</summary>
public class ComboBoxEx : ComboBox
{
}

/// <summary>Lista com seleção do Playnite (grade de jogos).</summary>
public class ListBoxEx : ListBox
{
}

/// <summary>Rolagem do Playnite. <see cref="CustomScrollAmount"/> é o passo da roda do mouse no Playnite; aqui, informativo.</summary>
public class ScrollViewerEx : ScrollViewer
{
    public static readonly DependencyProperty CustomScrollAmountProperty =
        DependencyProperty.Register(nameof(CustomScrollAmount), typeof(double), typeof(ScrollViewerEx), new PropertyMetadata(0.0));

    public double CustomScrollAmount
    {
        get => (double)GetValue(CustomScrollAmountProperty);
        set => SetValue(CustomScrollAmountProperty, value);
    }
}

/// <summary>Caixa de seleção do Playnite.</summary>
public class CheckBoxEx : CheckBox
{
}

/// <summary>Controle deslizante do Playnite.</summary>
public class SliderEx : Slider
{
}

/// <summary>Botão de alternância do Playnite.</summary>
public class ToggleButtonEx : ToggleButton
{
}

// Controles compostos de filtro do Playnite: existem para os estilos do tema carregarem. O painel de filtros completo
// fica fora do subset (MVP-019); os filtros rápidos (FilterPresetSelector) funcionam.

/// <summary>Seleção de itens de filtro (gêneros, plataformas...).</summary>
public class FilterDbItemtSelection : Control
{
}

/// <summary>Seleção de valores enumerados de filtro.</summary>
public class FilterEnumListSelection : Control
{
}

/// <summary>Seleção de textos de filtro.</summary>
public class FilterStringListSelection : Control
{
}

/// <summary>Painel de filtros.</summary>
public class Filters : Control
{
}

/// <summary>Painel de filtros adicionais.</summary>
public class FiltersAdditional : Control
{
}

/// <summary>Status do jogo em execução. No Tako a sessão de jogo tem tela própria; a do tema fica oculta.</summary>
public class GameStatus : Control
{
}

/// <summary>Imagem de fundo com transição do Playnite; aqui, imagem simples.</summary>
public class FadeImage : Image
{
}

/// <summary>
/// Item da grade de jogos (<c>PART_ImageCover</c>). O template vem do tema (estilo <c>ListGameItemTemplate</c>);
/// a capa é ligada à propriedade <c>CoverImage</c> do item, como no Playnite.
/// </summary>
[TemplatePart(Name = CoverPart, Type = typeof(Image))]
public class GameListItem : Control
{
    private const string CoverPart = "PART_ImageCover";

    public GameListItem()
    {
        Loaded += (_, _) => PinStyle();
    }

    /// <summary>
    /// O estilo vem do tema por <c>DynamicResource</c>. Ao sair da árvore (lista filtrada ou recarregada) a referência
    /// deixaria de achar o recurso, o template sumiria no meio da remoção e o WPF falharia ao percorrer os filhos dele
    /// (NullReferenceException em TreeWalkHelper). Fixa o estilo já resolvido como valor local.
    /// </summary>
    private void PinStyle()
    {
        if (ReadLocalValue(StyleProperty) is not System.Windows.Style && Style is { } style)
        {
            Style = style;
        }
    }

    public override void OnApplyTemplate()
    {
        base.OnApplyTemplate();
        if (GetTemplateChild(CoverPart) is Image cover)
        {
            cover.SetBinding(Image.SourceProperty, new Binding("CoverImage"));
        }
    }
}

/// <summary>
/// Substituto seguro do <c>HtmlTextView</c> do Playnite (MVP-018). O original renderiza HTML (imagens, links, estilos);
/// este nunca interpreta HTML: <see cref="HtmlText"/> vira texto puro (<see cref="PlainText.FromHtml"/>), sem carregar
/// nada da rede ou do disco e sem links clicáveis. Mantém o nome e as propriedades para o XAML do tema carregar.
/// </summary>
public class HtmlTextView : TextBlock
{
    public static readonly DependencyProperty HtmlTextProperty =
        DependencyProperty.Register(nameof(HtmlText), typeof(string), typeof(HtmlTextView), new PropertyMetadata(null, (d, e) => ((TextBlock)d).Text = PlainText.FromHtml(e.NewValue as string)));

    public static readonly DependencyProperty HtmlFontSizeProperty =
        DependencyProperty.Register(nameof(HtmlFontSize), typeof(double), typeof(HtmlTextView), new PropertyMetadata(16.0, (d, e) => ((TextBlock)d).FontSize = Math.Clamp((double)e.NewValue, 6, 96)));

    public static readonly DependencyProperty HtmlForegroundProperty =
        DependencyProperty.Register(nameof(HtmlForeground), typeof(Color), typeof(HtmlTextView), new PropertyMetadata(Colors.White, (d, e) => ((TextBlock)d).Foreground = Frozen((Color)e.NewValue)));

    /// <summary>Cor de links no Playnite. Links não existem no texto puro; a propriedade fica só por compatibilidade.</summary>
    public static readonly DependencyProperty LinkForegroundProperty =
        DependencyProperty.Register(nameof(LinkForeground), typeof(Color), typeof(HtmlTextView), new PropertyMetadata(Colors.White));

    public static readonly DependencyProperty HtmlFontFamilyProperty =
        DependencyProperty.Register(nameof(HtmlFontFamily), typeof(FontFamily), typeof(HtmlTextView), new PropertyMetadata(null, (d, e) =>
        {
            if (e.NewValue is FontFamily family)
            {
                ((TextBlock)d).FontFamily = family;
            }
        }));

    public HtmlTextView()
    {
        TextWrapping = TextWrapping.Wrap;
        FontSize = 16;
        Foreground = Brushes.White;
    }

    public string? HtmlText
    {
        get => (string?)GetValue(HtmlTextProperty);
        set => SetValue(HtmlTextProperty, value);
    }

    public double HtmlFontSize
    {
        get => (double)GetValue(HtmlFontSizeProperty);
        set => SetValue(HtmlFontSizeProperty, value);
    }

    public Color HtmlForeground
    {
        get => (Color)GetValue(HtmlForegroundProperty);
        set => SetValue(HtmlForegroundProperty, value);
    }

    public Color LinkForeground
    {
        get => (Color)GetValue(LinkForegroundProperty);
        set => SetValue(LinkForegroundProperty, value);
    }

    public FontFamily? HtmlFontFamily
    {
        get => (FontFamily?)GetValue(HtmlFontFamilyProperty);
        set => SetValue(HtmlFontFamilyProperty, value);
    }

    private static SolidColorBrush Frozen(Color color)
    {
        var brush = new SolidColorBrush(color);
        brush.Freeze();
        return brush;
    }
}

/// <summary>
/// <c>MediaElementBehaviors.Repeat</c> do Playnite: reinicia o vídeo/áudio ao terminar. Só mexe na posição do próprio
/// <see cref="MediaElement"/>; a origem da mídia continua sujeita às regras de caminho do validador.
/// </summary>
public static class MediaElementBehaviors
{
    public static readonly DependencyProperty RepeatProperty =
        DependencyProperty.RegisterAttached("Repeat", typeof(bool), typeof(MediaElementBehaviors), new PropertyMetadata(false, OnRepeatChanged));

    public static bool GetRepeat(DependencyObject element)
    {
        ArgumentNullException.ThrowIfNull(element);
        return (bool)element.GetValue(RepeatProperty);
    }

    public static void SetRepeat(DependencyObject element, bool value)
    {
        ArgumentNullException.ThrowIfNull(element);
        element.SetValue(RepeatProperty, value);
    }

    private static void OnRepeatChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        if (d is not MediaElement media)
        {
            return;
        }

        media.MediaEnded -= Restart;
        if (e.NewValue is true)
        {
            media.MediaEnded += Restart;
        }
    }

    private static void Restart(object sender, RoutedEventArgs e)
    {
        var media = (MediaElement)sender;
        media.Position = TimeSpan.Zero;
        if (media.LoadedBehavior == MediaState.Manual)
        {
            media.Play();
        }
    }
}

/// <summary>
/// Grade de capas do <c>PART_ListGameItems</c> (o <c>FullscreenTilePanel</c> do Playnite): colunas de largura igual e
/// altura proporcional à capa. Sem virtualização (a biblioteca do MVP cabe na memória).
/// </summary>
public class P10TilePanel : Panel
{
    public static readonly DependencyProperty ColumnsProperty =
        DependencyProperty.Register(nameof(Columns), typeof(int), typeof(P10TilePanel), new FrameworkPropertyMetadata(7, FrameworkPropertyMetadataOptions.AffectsMeasure));

    public static readonly DependencyProperty ItemAspectRatioProperty =
        DependencyProperty.Register(nameof(ItemAspectRatio), typeof(double), typeof(P10TilePanel), new FrameworkPropertyMetadata(1.5, FrameworkPropertyMetadataOptions.AffectsMeasure));

    /// <summary>Largura usada quando o painel é medido sem limite (ex.: rolagem horizontal).</summary>
    private const double FallbackWidth = 1600;

    public int Columns
    {
        get => (int)GetValue(ColumnsProperty);
        set => SetValue(ColumnsProperty, value);
    }

    /// <summary>Altura ÷ largura de cada item (capa 2:3 = 1,5).</summary>
    public double ItemAspectRatio
    {
        get => (double)GetValue(ItemAspectRatioProperty);
        set => SetValue(ItemAspectRatioProperty, value);
    }

    protected override Size MeasureOverride(Size availableSize)
    {
        var item = ItemSize(availableSize.Width);
        foreach (UIElement child in InternalChildren)
        {
            child.Measure(item);
        }

        int rows = (InternalChildren.Count + ColumnCount - 1) / ColumnCount;
        return new Size(item.Width * ColumnCount, item.Height * rows);
    }

    protected override Size ArrangeOverride(Size finalSize)
    {
        var item = ItemSize(finalSize.Width);
        for (int i = 0; i < InternalChildren.Count; i++)
        {
            InternalChildren[i].Arrange(new Rect(new Point(i % ColumnCount * item.Width, i / ColumnCount * item.Height), item));
        }

        return finalSize;
    }

    private int ColumnCount => Math.Clamp(Columns, 1, 30);

    private Size ItemSize(double available)
    {
        double width = (double.IsInfinity(available) || available <= 0 ? FallbackWidth : available) / ColumnCount;
        return new Size(width, width * Math.Clamp(ItemAspectRatio, 0.2, 5));
    }
}
