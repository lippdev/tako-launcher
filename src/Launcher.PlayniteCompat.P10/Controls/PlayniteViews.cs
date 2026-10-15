using System.Globalization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using System.Windows.Input;
using Launcher.PlayniteCompat.P10.Data;

namespace Launcher.PlayniteCompat.P10.Controls;

/// <summary>
/// Tela principal de um tema Fullscreen do Playnite (<c>Views/Main.xaml</c>, MVP-019). O tema só desenha; este controle
/// liga os elementos nomeados (<c>PART_</c>) ao <see cref="P10MainModel"/> (DataContext), como o
/// <c>Playnite.FullscreenApp.Controls.Views.Main</c>: grade de jogos e seleção, botões do menu principal e das ações,
/// filtros rápidos, relógio, detalhes do jogo. Elementos sem equivalente no Tako (bateria, progresso, atualização do
/// Playnite, painel completo de filtros, status do jogo) ficam ocultos.
/// </summary>
[TemplatePart(Name = "PART_ListGameItems", Type = typeof(ListBox))]
[TemplatePart(Name = "PART_ButtonPlay", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonInstall", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonDetails", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonGameOptions", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonSearch", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonFilter", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonMainMenu", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonNotifications", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonProgramUpdate", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_TextClock", Type = typeof(TextBlock))]
[TemplatePart(Name = "PART_ImageBackground", Type = typeof(Image))]
[TemplatePart(Name = "PART_ElemGameDetails", Type = typeof(FrameworkElement))]
[TemplatePart(Name = "PART_ElemGameStatus", Type = typeof(FrameworkElement))]
public class Main : Control
{
    public const string GamesGroup = "p10-games";
    public const string ActionsGroup = "p10-actions";
    public const string TopGroup = "p10-top";

    private static readonly BooleanToVisibilityConverter Visible = new();
    private static readonly InverseBooleanToVisibilityConverter Collapsed = new();

    private ListBox? _list;

    public Main()
    {
        Focusable = false;
    }

    public override void OnApplyTemplate()
    {
        base.OnApplyTemplate();
        WireGameList(Part<ListBox>("PART_ListGameItems"));

        // Ações do jogo selecionado (barra de baixo do tema).
        Button("PART_ButtonPlay", nameof(P10MainModel.ActivateSelectedCommand), ActionsGroup, "play", visibleWhenInstalled: true);
        Button("PART_ButtonInstall", nameof(P10MainModel.ActivateSelectedCommand), ActionsGroup, "install", visibleWhenInstalled: false);
        Button("PART_ButtonDetails", nameof(P10MainModel.ToggleGameDetailsCommand), ActionsGroup, "details");
        Button("PART_ButtonGameOptions", nameof(P10MainModel.ToggleGameOptionsCommand), ActionsGroup, "options");
        Button("PART_ButtonSearch", nameof(P10MainModel.OpenSearchCommand), ActionsGroup, "search");
        Button("PART_ButtonFilter", nameof(P10MainModel.ToggleFiltersCommand), ActionsGroup, "filter");

        // Barra de cima: o menu principal do tema abre as Configurações do Tako (saída garantida).
        Button("PART_ButtonMainMenu", nameof(P10MainModel.ToggleMainMenuCommand), TopGroup, "menu");
        Button("PART_ButtonNotifications", nameof(P10MainModel.ToggleNotificationsCommand), TopGroup, "notifications");
        if (Button("PART_ButtonProgramUpdate", null, TopGroup, "update") is { } update)
        {
            update.SetBinding(VisibilityProperty, new Binding(nameof(P10MainModel.UpdatesAvailable)) { Converter = Visible });
        }

        Bind(Part<TextBlock>("PART_TextClock"), TextBlock.TextProperty, nameof(P10MainModel.CurrentTime));
        Bind(Part<TextBlock>("PART_TextProgressTooltip"), TextBlock.TextProperty, nameof(P10MainModel.ProgressStatus));
        Bind(Part<Image>("PART_ImageBackground"), Image.SourceProperty, nameof(P10MainModel.BackgroundImage));
        BindVisibility("PART_ElemProgressIndicator", nameof(P10MainModel.ProgressActive));
        BindVisibility("PART_ElemSearchActive", nameof(P10MainModel.IsSearchActive));
        BindVisibility("PART_ElemExtraFilterActive", nameof(P10MainModel.IsExtraFilterActive));
        BindVisibility("PART_ElemFilters", nameof(P10MainModel.FilterPanelVisible));
        BindVisibility("PART_ElemFiltersAdditional", nameof(P10MainModel.FilterPanelVisible));
        BindVisibility("PART_ContentFilterItems", nameof(P10MainModel.FilterPanelVisible));
        BindVisibility("PART_ElemGameStatus", nameof(P10MainModel.GameStatusVisible));
        BindVisibility("PART_ElemBatteryStatus", nameof(P10MainModel.ShowBattery));
        BindVisibility("PART_TextBatteryPercentage", nameof(P10MainModel.ShowBattery));

        if (Part<FrameworkElement>("PART_ElemGameDetails") is { } details)
        {
            // O DataContext dos detalhes é o jogo; a visibilidade vem do modelo do Main (fonte explícita).
            details.SetBinding(VisibilityProperty, new Binding($"{nameof(DataContext)}.{nameof(P10MainModel.GameDetailsVisible)}") { Source = this, Converter = Visible, FallbackValue = Visibility.Collapsed });
            details.SetBinding(DataContextProperty, new Binding($"{nameof(DataContext)}.{nameof(P10MainModel.SelectedGame)}") { Source = this });
            if (details is GameDetails view)
            {
                view.SetBinding(GameDetails.ModelProperty, new Binding(nameof(DataContext)) { Source = this });
            }

            P10Navigation.Announce(details, e => new P10NavigableEventArgs(P10Navigation.NavigableEvent, e) { IsModalScope = true });
        }
    }

    /// <summary>Solta a seleção da grade do modelo (ensaio descartado: a lista não pode escrever no jogo selecionado).</summary>
    internal void DetachSelection()
    {
        if (_list is not null)
        {
            BindingOperations.ClearBinding(_list, Selector.SelectedItemProperty);
        }
    }

    private void WireGameList(ListBox? list)
    {
        if (_list is not null)
        {
            _list.ItemContainerGenerator.StatusChanged -= OnContainersChanged;
        }

        _list = list;
        if (list is null)
        {
            return;
        }

        list.SetBinding(ItemsControl.ItemsSourceProperty, new Binding(nameof(P10MainModel.Games)));
        list.SetBinding(Selector.SelectedItemProperty, new Binding(nameof(P10MainModel.SelectedGame)) { Mode = BindingMode.TwoWay });
        if (IsDefault(list, ItemsControl.ItemContainerStyleProperty))
        {
            list.SetResourceReference(ItemsControl.ItemContainerStyleProperty, "ListGameItemStyle");
        }

        if (IsDefault(list, ItemsControl.ItemTemplateProperty))
        {
            var item = new FrameworkElementFactory(typeof(GameListItem));
            item.SetResourceReference(StyleProperty, "ListGameItemTemplate");
            list.ItemTemplate = new DataTemplate { VisualTree = item };
        }

        if (IsDefault(list, ItemsControl.ItemsPanelProperty))
        {
            list.ItemsPanel = new ItemsPanelTemplate(new FrameworkElementFactory(typeof(P10TilePanel)));
        }

        if (IsDefault(list, ScrollViewer.HorizontalScrollBarVisibilityProperty))
        {
            ScrollViewer.SetHorizontalScrollBarVisibility(list, ScrollBarVisibility.Disabled);
        }

        list.ItemContainerGenerator.StatusChanged += OnContainersChanged;
        OnContainersChanged(list.ItemContainerGenerator, EventArgs.Empty);
    }

    /// <summary>Cada capa gerada entra na navegação; focar seleciona o jogo e rola a grade até ele.</summary>
    private void OnContainersChanged(object? sender, EventArgs e)
    {
        if (_list is not { } list || list.ItemContainerGenerator.Status != GeneratorStatus.ContainersGenerated)
        {
            return;
        }

        var toggleDetails = (DataContext as P10MainModel)?.ToggleGameDetailsCommand;
        for (int i = 0; i < list.Items.Count; i++)
        {
            if (list.ItemContainerGenerator.ContainerFromIndex(i) is not ListBoxItem container || list.Items[i] is not P10GameEntry game)
            {
                continue;
            }

            container.Focusable = true;
            container.GotKeyboardFocus -= OnItemFocused;
            container.GotKeyboardFocus += OnItemFocused;
            P10Navigation.Announce(container, c => new P10NavigableEventArgs(P10Navigation.NavigableEvent, c)
            {
                Group = GamesGroup,
                Key = game.Id,
                IsDefault = list.Items.IndexOf(game) == 0,
                Activate = toggleDetails,
                ActivateParameter = game,
            });
        }
    }

    private void OnItemFocused(object sender, KeyboardFocusChangedEventArgs e)
    {
        if (sender is ListBoxItem container && _list is { } list)
        {
            list.SelectedItem = list.ItemContainerGenerator.ItemFromContainer(container);
            container.BringIntoView();
        }
    }

    private ButtonBase? Button(string name, string? command, string group, string key, bool? visibleWhenInstalled = null)
    {
        if (Part<ButtonBase>(name) is not { } button)
        {
            return null;
        }

        if (command is not null)
        {
            button.SetBinding(ButtonBase.CommandProperty, new Binding(command));
        }

        if (visibleWhenInstalled is { } installed)
        {
            button.SetBinding(VisibilityProperty, new Binding($"{nameof(P10MainModel.SelectedGame)}.{nameof(P10GameEntry.IsInstalled)}")
            {
                Converter = installed ? Visible : Collapsed,
                FallbackValue = Visibility.Collapsed,
                TargetNullValue = Visibility.Collapsed,
            });
        }

        P10Navigation.Announce(button, b => new P10NavigableEventArgs(P10Navigation.NavigableEvent, b) { Group = group, Key = key });
        return button;
    }

    private void BindVisibility(string name, string path)
    {
        if (Part<FrameworkElement>(name) is { } element)
        {
            element.SetBinding(VisibilityProperty, new Binding(path) { Converter = Visible, FallbackValue = Visibility.Collapsed });
        }
    }

    private static void Bind(FrameworkElement? element, DependencyProperty property, string path) =>
        element?.SetBinding(property, new Binding(path));

    private T? Part<T>(string name)
        where T : class => GetTemplateChild(name) as T;

    private static bool IsDefault(DependencyObject element, DependencyProperty property) =>
        DependencyPropertyHelper.GetValueSource(element, property).BaseValueSource == BaseValueSource.Default;
}

/// <summary>
/// Detalhes do jogo (<c>Views/GameDetails.xaml</c>, MVP-019). DataContext = jogo selecionado; <see cref="Model"/> dá os
/// comandos (Jogar/Instalar no <c>PART_ButtonContext</c>, opções no <c>PART_ButtonOptions</c>). A descrição vai para o
/// <see cref="HtmlTextView"/> seguro como texto puro.
/// </summary>
[TemplatePart(Name = "PART_ButtonContext", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ButtonOptions", Type = typeof(ButtonBase))]
[TemplatePart(Name = "PART_ImageCover", Type = typeof(Image))]
[TemplatePart(Name = "PART_ImageBackground", Type = typeof(Image))]
[TemplatePart(Name = "PART_HtmlDescription", Type = typeof(TextBlock))]
public class GameDetails : Control
{
    public const string DetailsGroup = "p10-details";

    public static readonly DependencyProperty ModelProperty =
        DependencyProperty.Register(nameof(Model), typeof(P10MainModel), typeof(GameDetails), new PropertyMetadata(null));

    public GameDetails()
    {
        Focusable = false;
    }

    public P10MainModel? Model
    {
        get => (P10MainModel?)GetValue(ModelProperty);
        set => SetValue(ModelProperty, value);
    }

    public override void OnApplyTemplate()
    {
        base.OnApplyTemplate();
        if (GetTemplateChild("PART_ButtonContext") is ButtonBase context)
        {
            context.SetBinding(ButtonBase.CommandProperty, new Binding($"{nameof(Model)}.{nameof(P10MainModel.ActivateSelectedCommand)}") { Source = this });
            if (context is ContentControl { Content: null } content)
            {
                content.SetBinding(ContentControl.ContentProperty, new Binding($"{nameof(Model)}.{nameof(P10MainModel.SelectedGameActionLabel)}") { Source = this });
            }

            P10Navigation.Announce(context, b => new P10NavigableEventArgs(P10Navigation.NavigableEvent, b) { Group = DetailsGroup, Key = "play", IsDefault = true });
        }

        if (GetTemplateChild("PART_ButtonOptions") is ButtonBase options)
        {
            options.SetBinding(ButtonBase.CommandProperty, new Binding($"{nameof(Model)}.{nameof(P10MainModel.ToggleGameOptionsCommand)}") { Source = this });
            P10Navigation.Announce(options, b => new P10NavigableEventArgs(P10Navigation.NavigableEvent, b) { Group = DetailsGroup, Key = "options" });
        }

        if (GetTemplateChild("PART_ImageCover") is Image cover)
        {
            cover.SetBinding(Image.SourceProperty, new Binding(nameof(P10GameEntry.CoverImage)));
        }

        if (GetTemplateChild("PART_ImageBackground") is Image background)
        {
            background.SetBinding(Image.SourceProperty, new Binding(nameof(P10GameEntry.BackgroundImage)));
        }

        switch (GetTemplateChild("PART_HtmlDescription"))
        {
            case HtmlTextView html when html.ReadLocalValue(HtmlTextView.HtmlTextProperty) == DependencyProperty.UnsetValue:
                html.SetBinding(HtmlTextView.HtmlTextProperty, new Binding(nameof(P10GameEntry.Description)));
                break;
            case TextBlock text when text is not HtmlTextView:
                text.SetBinding(TextBlock.TextProperty, new Binding(nameof(P10GameEntry.Description)));
                break;
        }
    }
}

/// <summary>
/// Filtros rápidos (<c>PART_ItemsFilterPresets</c>, MVP-019): um <c>CheckBoxEx</c> por filtro com o estilo
/// <c>ItemFilterQuickPreset</c> do tema; A aplica o filtro na grade.
/// </summary>
[TemplatePart(Name = "PART_ItemsFilterPresets", Type = typeof(ItemsControl))]
public class FilterPresetSelector : Control
{
    public const string PresetsGroup = "p10-presets";

    public FilterPresetSelector()
    {
        Focusable = false;
    }

    public override void OnApplyTemplate()
    {
        base.OnApplyTemplate();
        if (GetTemplateChild("PART_ItemsFilterPresets") is not ItemsControl items)
        {
            return;
        }

        items.SetBinding(ItemsControl.ItemsSourceProperty, new Binding(nameof(P10MainModel.FilterPresets)));
        if (items.ItemTemplate is null)
        {
            var check = new FrameworkElementFactory(typeof(CheckBoxEx));
            check.SetResourceReference(StyleProperty, "ItemFilterQuickPreset");
            check.SetBinding(ToggleButton.IsCheckedProperty, new Binding(nameof(P10FilterPreset.IsActive)) { Mode = BindingMode.OneWay });
            check.SetBinding(ButtonBase.CommandProperty, new Binding($"{nameof(DataContext)}.{nameof(P10MainModel.ApplyFilterPresetCommand)}") { Source = this });
            check.SetBinding(ButtonBase.CommandParameterProperty, new Binding());
            check.AddHandler(LoadedEvent, new RoutedEventHandler(OnPresetLoaded));
            items.ItemTemplate = new DataTemplate { VisualTree = check };
        }
    }

    private static void OnPresetLoaded(object sender, RoutedEventArgs e)
    {
        if (sender is FrameworkElement { DataContext: P10FilterPreset preset } check)
        {
            P10Navigation.Announce(check, c => new P10NavigableEventArgs(P10Navigation.NavigableEvent, c) { Group = PresetsGroup, Key = preset.Name });
        }
    }
}

/// <summary>Falso → Visible; verdadeiro → Collapsed.</summary>
public sealed class InverseBooleanToVisibilityConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value is true ? Visibility.Collapsed : Visibility.Visible;

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => Binding.DoNothing;
}
