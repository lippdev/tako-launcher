using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using System.Windows.Media;

namespace Launcher.PlayniteCompat.P10.Controls;

#pragma warning disable CA1711 // Os nomes (ButtonEx...) são os que os temas Playnite referenciam; não podem mudar.

// Equivalentes dos controles do Playnite.FullscreenApp usados pelos temas Fullscreen (MVP-018/019).
// O tema define estilos e templates; o comportamento específico do Playnite entra conforme a homologação (MVP-022).

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

/// <summary>Lista suspensa do Playnite (a lista abre num Popup, que temas não podem usar; aqui só o controle).</summary>
public class ComboBoxEx : ComboBox
{
}

/// <summary>Lista com seleção do Playnite (grade de jogos).</summary>
public class ListBoxEx : ListBox
{
}

/// <summary>Rolagem do Playnite.</summary>
public class ScrollViewerEx : ScrollViewer
{
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

// Controles compostos e views do Playnite. No Marco 0 existem para os estilos do tema carregarem; o comportamento
// (filtros, detalhes, status do jogo) entra com o adaptador completo (MVP-018/019).

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

/// <summary>Seletor de predefinições de filtro.</summary>
public class FilterPresetSelector : Control
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

/// <summary>Detalhes do jogo.</summary>
public class GameDetails : Control
{
}

/// <summary>Status do jogo em execução.</summary>
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

    public override void OnApplyTemplate()
    {
        base.OnApplyTemplate();
        if (GetTemplateChild(CoverPart) is Image cover)
        {
            cover.SetBinding(Image.SourceProperty, new Binding("CoverImage"));
        }
    }
}

/// <summary>Dados que os templates Playnite esperam de um jogo (subconjunto de <c>GamesCollectionViewEntry</c>).</summary>
public sealed class P10GameEntry(string displayName, bool isInstalled, ImageSource? coverImage)
{
    public string DisplayName { get; } = displayName;

    public bool IsInstalled { get; } = isInstalled;

    public ImageSource? CoverImage { get; set; } = coverImage;
}
