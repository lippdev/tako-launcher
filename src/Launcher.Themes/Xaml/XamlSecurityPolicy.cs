using System.Collections.Frozen;

namespace Launcher.Themes.Xaml;

/// <summary>
/// Lista de permissões do carregador XAML restrito (MVP-016). Tudo o que não está listado é rejeitado
/// antes de o XAML chegar ao <c>XamlReader</c> do WPF.
/// </summary>
public sealed class XamlSecurityPolicy
{
    public const string PresentationNamespace = "http://schemas.microsoft.com/winfx/2006/xaml/presentation";
    public const string XamlNamespace = "http://schemas.microsoft.com/winfx/2006/xaml";
    public const string MarkupCompatibilityNamespace = "http://schemas.openxmlformats.org/markup-compatibility/2006";
    public const string DesignerNamespace = "http://schemas.microsoft.com/expression/blend/2008";
    public const string PresentationOptionsNamespace = "http://schemas.microsoft.com/winfx/2006/xaml/presentation/options";

    /// <summary>Tipos instanciáveis como elementos no namespace de apresentação do WPF.</summary>
    public required FrozenSet<string> PresentationTypes { get; init; }

    /// <summary>
    /// Tipos que podem aparecer como dono de propriedade anexada ou de elemento de propriedade
    /// (ex.: <c>Grid.Row</c>, <c>&lt;Control.Template&gt;</c>) sem serem instanciados.
    /// </summary>
    public required FrozenSet<string> PresentationOwnerTypes { get; init; }

    /// <summary>
    /// Tipos do namespace de apresentação cujos membros estáticos podem ser lidos com <c>{x:Static}</c>.
    /// Nos mapeamentos <c>clr-namespace</c>, vale a lista de tipos do próprio mapeamento.
    /// </summary>
    public required FrozenSet<string> StaticMemberOwners { get; init; }

    /// <summary>Mapeamentos <c>clr-namespace:...</c> permitidos e os tipos permitidos em cada um.</summary>
    public required FrozenDictionary<string, FrozenSet<string>> ClrNamespaces { get; init; }

    /// <summary>Propriedades cujo valor é carregado como arquivo ou URI pelo WPF.</summary>
    public required FrozenSet<string> UriProperties { get; init; }

    /// <summary>Extensões de marcação permitidas no namespace de apresentação (sem o sufixo "Extension").</summary>
    public FrozenSet<string> PresentationExtensions { get; init; } = Set("Binding", "StaticResource", "DynamicResource", "TemplateBinding", "RelativeSource");

    /// <summary>
    /// <c>clr-namespace</c> de origem → de destino. O carregador reescreve a declaração depois da validação
    /// (ex.: <c>assembly=Playnite.FullscreenApp</c> → adaptador P10).
    /// </summary>
    public FrozenDictionary<string, string> NamespaceRemaps { get; init; } = FrozenDictionary<string, string>.Empty;

    public int MaxCharacters { get; init; } = 2 * 1024 * 1024;

    public int MaxDepth { get; init; } = 128;

    public int MaxElements { get; init; } = 50_000;

    /// <summary>
    /// Política ampliada por um adaptador (ex.: P10): tipos e extensões que o adaptador implementa e mapeamentos de
    /// <c>clr-namespace</c> reescritos para o assembly do adaptador.
    /// </summary>
    public XamlSecurityPolicy Extend(
        IEnumerable<string>? presentationTypes = null,
        IEnumerable<string>? presentationExtensions = null,
        IEnumerable<(string Source, string Target, IEnumerable<string> Types)>? remappedNamespaces = null)
    {
        var namespaces = ClrNamespaces.ToDictionary(StringComparer.Ordinal);
        var remaps = NamespaceRemaps.ToDictionary(StringComparer.Ordinal);
        foreach (var (source, target, types) in remappedNamespaces ?? [])
        {
            namespaces[source] = Set([.. types]);
            remaps[source] = target;
        }

        return new XamlSecurityPolicy
        {
            PresentationTypes = Set([.. PresentationTypes, .. presentationTypes ?? []]),
            PresentationOwnerTypes = PresentationOwnerTypes,
            StaticMemberOwners = StaticMemberOwners,
            ClrNamespaces = namespaces.ToFrozenDictionary(StringComparer.Ordinal),
            UriProperties = UriProperties,
            PresentationExtensions = Set([.. PresentationExtensions, .. presentationExtensions ?? []]),
            NamespaceRemaps = remaps.ToFrozenDictionary(StringComparer.Ordinal),
            MaxCharacters = MaxCharacters,
            MaxDepth = MaxDepth,
            MaxElements = MaxElements,
        };
    }

    public static XamlSecurityPolicy CreateDefault() => new()
    {
        PresentationTypes = Set(
            // Layout
            "Grid", "ColumnDefinition", "RowDefinition", "StackPanel", "WrapPanel", "DockPanel", "Canvas",
            "UniformGrid", "VirtualizingStackPanel", "Viewbox", "Border", "ScrollViewer", "ScrollContentPresenter",
            "ItemsPresenter", "ContentPresenter", "AdornerDecorator", "Decorator",
            // Controles
            "ContentControl", "Control", "UserControl", "Button", "ToggleButton", "RepeatButton", "CheckBox",
            "RadioButton", "TextBlock", "Label", "AccessText", "ItemsControl", "ListBox", "ListBoxItem", "ListView",
            "ListViewItem", "ComboBox", "ComboBoxItem", "ProgressBar", "Slider", "Track", "Thumb", "ScrollBar",
            "Image", "MediaElement", "Separator", "ToolTip", "GroupBox", "Expander", "HeaderedContentControl",
            "TabControl", "TabItem", "TabPanel", "GridSplitter",
            // Texto
            "Run", "LineBreak", "Span", "Bold", "Italic", "Underline", "InlineUIContainer",
            // Formas e geometria
            "Rectangle", "Ellipse", "Line", "Path", "Polygon", "Polyline", "PathGeometry", "PathFigure",
            "PathFigureCollection", "PathSegmentCollection", "LineSegment", "BezierSegment", "ArcSegment",
            "PolyLineSegment", "PolyBezierSegment", "QuadraticBezierSegment", "RectangleGeometry",
            "EllipseGeometry", "LineGeometry", "GeometryGroup", "CombinedGeometry", "GeometryDrawing",
            "DrawingGroup", "DrawingImage", "Pen",
            // Pincéis, cores, imagens, efeitos
            "SolidColorBrush", "LinearGradientBrush", "RadialGradientBrush", "GradientStop",
            "GradientStopCollection", "ImageBrush", "VisualBrush", "DrawingBrush", "BitmapImage", "BitmapCache",
            "Color", "FontFamily", "Thickness", "CornerRadius", "GridLength", "Point", "Rect", "Size",
            "DropShadowEffect", "BlurEffect",
            // Transformações
            "ScaleTransform", "RotateTransform", "TranslateTransform", "SkewTransform", "MatrixTransform",
            "TransformGroup", "TransformCollection",
            // Recursos, estilos, templates, gatilhos
            "ResourceDictionary", "Style", "Setter", "Trigger", "MultiTrigger", "DataTrigger", "MultiDataTrigger",
            "Condition", "EventTrigger", "ControlTemplate", "DataTemplate", "HierarchicalDataTemplate",
            "ItemsPanelTemplate", "Binding", "MultiBinding", "PriorityBinding", "RelativeSource",
            "TemplateBindingExtension", "StaticResourceExtension", "DynamicResourceExtension",
            // Estados visuais e animação
            "VisualStateGroup", "VisualState", "VisualTransition", "Storyboard", "BeginStoryboard",
            "StopStoryboard", "PauseStoryboard", "ResumeStoryboard", "RemoveStoryboard", "SeekStoryboard",
            "DoubleAnimation", "ColorAnimation", "ThicknessAnimation", "PointAnimation", "Int32Animation",
            "DoubleAnimationUsingKeyFrames", "LinearDoubleKeyFrame", "EasingDoubleKeyFrame",
            "SplineDoubleKeyFrame", "DiscreteDoubleKeyFrame", "ColorAnimationUsingKeyFrames",
            "LinearColorKeyFrame", "EasingColorKeyFrame", "SplineColorKeyFrame", "DiscreteColorKeyFrame",
            "ThicknessAnimationUsingKeyFrames", "LinearThicknessKeyFrame", "EasingThicknessKeyFrame",
            "DiscreteThicknessKeyFrame", "ObjectAnimationUsingKeyFrames", "DiscreteObjectKeyFrame",
            "BooleanAnimationUsingKeyFrames", "DiscreteBooleanKeyFrame", "StringAnimationUsingKeyFrames",
            "DiscreteStringKeyFrame", "KeySpline", "CubicEase", "QuadraticEase", "QuarticEase", "QuinticEase",
            "SineEase", "ExponentialEase", "BackEase", "BounceEase", "CircleEase", "ElasticEase", "PowerEase"),
        PresentationOwnerTypes = Set(
            "UIElement", "FrameworkElement", "Control", "ContentControl", "ItemsControl", "Panel", "Selector",
            "ButtonBase", "RangeBase", "TextElement", "Inline", "Shape", "Brush", "Transform", "Geometry",
            "Timeline", "KeyboardNavigation", "FocusManager", "ToolTipService", "RenderOptions", "TextOptions",
            "Typography", "VisualStateManager", "VirtualizingPanel", "AutomationProperties", "Validation",
            "SpellCheck"),
        StaticMemberOwners = Set(
            "SystemColors", "SystemFonts", "SystemParameters", "Colors", "Brushes", "FontWeights", "FontStyles",
            "FontStretches", "Visibility", "HorizontalAlignment", "VerticalAlignment", "Orientation", "Stretch",
            "TextWrapping", "TextTrimming", "TextAlignment", "Dock", "ScrollBarVisibility", "Key"),
        ClrNamespaces = new Dictionary<string, FrozenSet<string>>
        {
            ["clr-namespace:System;assembly=mscorlib"] = SystemPrimitives,
            ["clr-namespace:System;assembly=System.Runtime"] = SystemPrimitives,
        }.ToFrozenDictionary(StringComparer.Ordinal),
        UriProperties = Set(
            "Source", "UriSource", "ImageSource", "NavigateUri", "BaseUri", "Cursor", "FontFamily"),
    };

    private static readonly FrozenSet<string> SystemPrimitives = Set(
        "String", "Double", "Single", "Decimal", "Int16", "Int32", "Int64", "Byte", "Boolean", "Char", "TimeSpan");

    private static FrozenSet<string> Set(params string[] items) => items.ToFrozenSet(StringComparer.Ordinal);
}
