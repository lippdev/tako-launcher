using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using System.Windows.Media;
using Launcher.Core.Presentation;
using Launcher.PlayniteCompat.P10.Data;

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

// Controles de filtro do Playnite (UX-002, MVP-019). Como no Playnite, o tema desenha o painel e o adaptador preenche o
// PART_ItemsHost com caixas e botões que usam os estilos do próprio tema (FilterPanelCheckBoxEx,
// FilterPanelNagivationButton, FilterPanelButtonEx, FilterPanelText e FilterItemtSelectionStyle).

/// <summary>
/// Painel de filtros (<c>PART_ElemFilters</c>): Instalados, Não instalados, Favoritos, Jogados recentemente, listas de
/// Fonte e Gênero, Ordenar e Limpar filtros. DataContext = <see cref="P10MainModel"/>. Enquanto visível, só ele navega.
/// </summary>
[TemplatePart(Name = "PART_ItemsHost", Type = typeof(ItemsControl))]
public class Filters : Control
{
    public const string FiltersGroup = "p10-filters";

    public Filters()
    {
        Focusable = false;
        DataContextChanged += (_, _) => Build();
    }

    public override void OnApplyTemplate()
    {
        base.OnApplyTemplate();
        Build();
        P10Navigation.Announce(this, f => new P10NavigableEventArgs(P10Navigation.NavigableEvent, f) { IsModalScope = true });
    }

    private void Build()
    {
        if (GetTemplateChild("PART_ItemsHost") is not ItemsControl host || DataContext is not P10MainModel model)
        {
            return;
        }

        var panel = model.FilterPanel;
        host.ItemsSource = null;
        host.Items.Clear();
        host.Items.Add(FilterElements.Header("Filtros"));
        foreach (var option in panel.Checks)
        {
            host.Items.Add(FilterElements.Check(option, "FilterPanelCheckBoxEx", FiltersGroup, isDefault: option == panel.Checks[0]));
        }

        foreach (var option in panel.Lists)
        {
            host.Items.Add(FilterElements.Button(option, "FilterPanelNagivationButton", FiltersGroup));
        }

        host.Items.Add(FilterElements.Header("Ordenação"));
        host.Items.Add(FilterElements.Button(panel.Sort, "FilterPanelButtonEx", FiltersGroup));
        host.Items.Add(FilterElements.Button(panel.Clear, "FilterPanelButtonEx", FiltersGroup));
    }
}

/// <summary>
/// Lista de um filtro (fonte ou gênero) em <c>PART_ContentFilterItems</c>: uma caixa por valor, Voltar
/// (<c>PART_ButtonBack</c>) e Limpar (<c>PART_ButtonClear</c>). DataContext = <see cref="P10SubFilter"/>.
/// </summary>
[TemplatePart(Name = "PART_ItemsHost", Type = typeof(ItemsControl))]
[TemplatePart(Name = "PART_ButtonBack", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonClear", Type = typeof(ButtonBase))]
public class FilterDbItemtSelection : Control
{
    public const string SubFilterGroup = "p10-subfilter";

    public FilterDbItemtSelection()
    {
        Focusable = false;
        DataContextChanged += (_, _) => Build();
    }

    public override void OnApplyTemplate()
    {
        base.OnApplyTemplate();
        Build();
        P10Navigation.Announce(this, f => new P10NavigableEventArgs(P10Navigation.NavigableEvent, f) { IsModalScope = true });
    }

    private void Build()
    {
        if (GetTemplateChild("PART_ItemsHost") is not ItemsControl host)
        {
            return;
        }

        host.ItemsSource = null;
        host.Items.Clear();
        if (DataContext is not P10SubFilter filter)
        {
            return;
        }

        if (GetTemplateChild("PART_ButtonBack") is ButtonBase back)
        {
            back.Command = filter.BackCommand;
            P10Navigation.Announce(back, b => new P10NavigableEventArgs(P10Navigation.NavigableEvent, b) { Group = SubFilterGroup, Key = "back", IsDefault = filter.Options.Count == 0 });
        }

        if (GetTemplateChild("PART_ButtonClear") is ButtonBase clear)
        {
            clear.Command = filter.ClearCommand;
            P10Navigation.Announce(clear, b => new P10NavigableEventArgs(P10Navigation.NavigableEvent, b) { Group = SubFilterGroup, Key = "clear" });
        }

        foreach (var option in filter.Options)
        {
            host.Items.Add(FilterElements.Check(option, "FilterItemtSelectionStyle", SubFilterGroup, isDefault: option == filter.Options[0]));
        }
    }
}

/// <summary>Seleção de valores enumerados de filtro (existe para o estilo do tema carregar; as listas usam <see cref="FilterDbItemtSelection"/>).</summary>
public class FilterEnumListSelection : Control
{
}

/// <summary>Seleção de textos de filtro (existe para o estilo do tema carregar).</summary>
public class FilterStringListSelection : Control
{
}

/// <summary>Painel de filtros adicionais (oculto: fonte e gênero ficam no painel principal).</summary>
public class FiltersAdditional : Control
{
}

/// <summary>Elementos dos painéis de filtro, com os estilos do tema por nome (sem o estilo, o controle fica com o padrão).</summary>
internal static class FilterElements
{
    private static readonly BooleanToVisibilityConverter Visible = new();

    public static TextBlock Header(string text)
    {
        var header = new TextBlock { Text = text };
        header.SetResourceReference(FrameworkElement.StyleProperty, "FilterPanelText");
        return header;
    }

    public static CheckBoxEx Check(P10FilterOption option, string styleKey, string group, bool isDefault)
    {
        var check = new CheckBoxEx { DataContext = option, Command = option.Command };
        check.SetResourceReference(FrameworkElement.StyleProperty, styleKey);
        check.SetBinding(ContentControl.ContentProperty, new Binding(nameof(P10FilterOption.Name)));
        check.SetBinding(ToggleButton.IsCheckedProperty, new Binding(nameof(P10FilterOption.IsChecked)) { Mode = BindingMode.OneWay });
        check.SetBinding(UIElement.VisibilityProperty, new Binding(nameof(P10FilterOption.IsVisible)) { Converter = Visible });
        P10Navigation.Announce(check, c => new P10NavigableEventArgs(P10Navigation.NavigableEvent, c) { Group = group, Key = option.Key, IsDefault = isDefault });
        return check;
    }

    /// <summary>Botão do painel; <c>Tag</c> = verdadeiro quando há algo escolhido (o tema destaca o texto).</summary>
    public static ButtonEx Button(P10FilterOption option, string styleKey, string group)
    {
        var button = new ButtonEx { DataContext = option, Command = option.Command };
        button.SetResourceReference(FrameworkElement.StyleProperty, styleKey);
        button.SetBinding(ContentControl.ContentProperty, new Binding(nameof(P10FilterOption.Name)));
        button.SetBinding(FrameworkElement.TagProperty, new Binding(nameof(P10FilterOption.IsChecked)) { Mode = BindingMode.OneWay });
        button.SetBinding(UIElement.VisibilityProperty, new Binding(nameof(P10FilterOption.IsVisible)) { Converter = Visible });
        P10Navigation.Announce(button, b => new P10NavigableEventArgs(P10Navigation.NavigableEvent, b) { Group = group, Key = option.Key });
        return button;
    }
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
        PinStyle(); // antes do Loaded: a lista pode recriar os itens já na primeira medida
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
