using Launcher.Themes.Xaml;
using Xunit;

namespace Launcher.Themes.Tests.Xaml;

public class XamlValidatorTests
{
    private const string Namespaces =
        """xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" """;

    private readonly XamlValidator _validator = new(XamlSecurityPolicy.CreateDefault());

    private static string Dictionary(string body, string extraNamespaces = "") =>
        $"<ResourceDictionary {Namespaces}{extraNamespaces}>{body}</ResourceDictionary>";

    private XamlValidationResult Validate(string xaml, string path = "Views/Main.xaml") =>
        _validator.Validate(xaml, path, expectedRoot: "ResourceDictionary");

    private static void AssertRejected(XamlValidationResult result, XamlViolationCode code) =>
        Assert.Contains(result.Violations, v => v.Code == code);

    [Fact]
    public void Accepts_typical_fullscreen_theme_dictionary()
    {
        const string xaml = """
            <ResourceDictionary xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
                                xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
                                xmlns:sys="clr-namespace:System;assembly=mscorlib"
                                xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
                                xmlns:d="http://schemas.microsoft.com/expression/blend/2008"
                                xmlns:po="http://schemas.microsoft.com/winfx/2006/xaml/presentation/options"
                                mc:Ignorable="d po">
                <sys:Double x:Key="CoverWidth">240</sys:Double>
                <SolidColorBrush x:Key="AccentBrush" Color="#FF3A7BD5" po:Freeze="True" />
                <FontFamily x:Key="TitleFont">./Fonts/#Inter</FontFamily>
                <Style x:Key="GameCard" TargetType="{x:Type ListBoxItem}">
                    <Setter Property="Width" Value="{StaticResource CoverWidth}" />
                    <Setter Property="Foreground" Value="{DynamicResource AccentBrush}" />
                    <Setter Property="Template">
                        <Setter.Value>
                            <ControlTemplate TargetType="ListBoxItem">
                                <Grid d:DesignWidth="240">
                                    <Grid.RowDefinitions>
                                        <RowDefinition Height="*" />
                                        <RowDefinition Height="Auto" />
                                    </Grid.RowDefinitions>
                                    <Border x:Name="Root" CornerRadius="8" RenderTransformOrigin="0.5,0.5">
                                        <Border.RenderTransform>
                                            <ScaleTransform />
                                        </Border.RenderTransform>
                                        <Image Source="../Images/placeholder.png" Stretch="UniformToFill" />
                                    </Border>
                                    <TextBlock Grid.Row="1" Text="{Binding Name, StringFormat={}{0}}"
                                               FontFamily="{StaticResource TitleFont}"
                                               Foreground="{x:Static SystemColors.HighlightTextBrush}" />
                                </Grid>
                                <ControlTemplate.Triggers>
                                    <Trigger Property="IsSelected" Value="True">
                                        <Trigger.EnterActions>
                                            <BeginStoryboard>
                                                <Storyboard>
                                                    <DoubleAnimation Storyboard.TargetName="Root"
                                                                     Storyboard.TargetProperty="(UIElement.RenderTransform).(ScaleTransform.ScaleX)"
                                                                     To="1.1" Duration="0:0:0.15">
                                                        <DoubleAnimation.EasingFunction>
                                                            <CubicEase EasingMode="EaseOut" />
                                                        </DoubleAnimation.EasingFunction>
                                                    </DoubleAnimation>
                                                </Storyboard>
                                            </BeginStoryboard>
                                        </Trigger.EnterActions>
                                    </Trigger>
                                </ControlTemplate.Triggers>
                            </ControlTemplate>
                        </Setter.Value>
                    </Setter>
                </Style>
            </ResourceDictionary>
            """;

        var result = Validate(xaml);

        Assert.True(result.IsValid, string.Join(Environment.NewLine, result.Violations));
    }

    [Fact]
    public void Rejects_ObjectDataProvider_process_start_gadget()
    {
        const string body = """
            <ObjectDataProvider x:Key="Run" ObjectType="{x:Type diag:Process}" MethodName="Start">
                <ObjectDataProvider.MethodParameters>
                    <sys:String>calc.exe</sys:String>
                </ObjectDataProvider.MethodParameters>
            </ObjectDataProvider>
            """;
        const string ns = """xmlns:diag="clr-namespace:System.Diagnostics;assembly=System" xmlns:sys="clr-namespace:System;assembly=mscorlib" """;

        var result = Validate(Dictionary(body, ns));

        AssertRejected(result, XamlViolationCode.NamespaceNotAllowed);
        AssertRejected(result, XamlViolationCode.TypeNotAllowed);
    }

    [Theory]
    [InlineData("""<x:Code><![CDATA[ void F() { } ]]></x:Code>""")]
    [InlineData("""<Style x:Key="s" x:Class="Evil.Theme" />""")]
    [InlineData("""<Style x:Key="s" x:Subclass="Evil.Theme" />""")]
    [InlineData("""<Style x:Key="s" x:FactoryMethod="Create" />""")]
    [InlineData("""<Style x:Key="s"><x:Arguments><Style /></x:Arguments></Style>""")]
    [InlineData("""<x:XData><data /></x:XData>""")]
    public void Rejects_dangerous_xaml_directives(string body) =>
        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.DirectiveNotAllowed);

    [Theory]
    [InlineData("""<XmlDataProvider x:Key="d" Source="data.xml" />""")]
    [InlineData("""<Frame x:Key="f" />""")]
    [InlineData("""<WebBrowser x:Key="w" />""")]
    [InlineData("""<Window x:Key="w" />""")]
    [InlineData("""<Style x:Key="s" TargetType="Button"><EventSetter Event="Click" Handler="OnClick" /></Style>""")]
    [InlineData("""<Popup x:Key="p" />""")]
    public void Rejects_types_outside_allowlist(string body) =>
        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.TypeNotAllowed);

    private static string ComboBoxTemplate(string popup, string triggers = "", string targetType = "ComboBox") => Dictionary($$"""
        <Style x:Key="c" TargetType="ComboBox">
          <Setter Property="Template">
            <Setter.Value>
              <ControlTemplate TargetType="{{targetType}}">
                <Grid>
                  {{popup}}
                </Grid>
                <ControlTemplate.Triggers>{{triggers}}</ControlTemplate.Triggers>
              </ControlTemplate>
            </Setter.Value>
          </Setter>
        </Style>
        """);

    private const string DropDownPopup = """
        <Popup x:Name="Popup" IsOpen="{TemplateBinding IsDropDownOpen}" Placement="Bottom" Focusable="False" AllowsTransparency="True">
          <Border><ScrollViewer><StackPanel IsItemsHost="True" /></ScrollViewer></Border>
        </Popup>
        """;

    [Theory]
    [InlineData("ComboBox")]
    [InlineData("{x:Type ComboBox}")]
    public void Accepts_drop_down_popup_in_combo_box_template(string targetType)
    {
        var result = Validate(ComboBoxTemplate(DropDownPopup, """<Trigger Property="IsEnabled" Value="False"><Setter TargetName="Popup" Property="PopupAnimation" Value="None" /></Trigger>""", targetType));

        Assert.True(result.IsValid, string.Join(Environment.NewLine, result.Violations));
    }

    [Fact]
    public void Extended_policy_adds_popup_host_templates()
    {
        string xaml = ComboBoxTemplate(DropDownPopup, targetType: "ComboBoxEx");
        AssertRejected(Validate(xaml), XamlViolationCode.TypeNotAllowed);

        var policy = XamlSecurityPolicy.CreateDefault().Extend(presentationTypes: ["ComboBoxEx"], popupHostTemplates: ["ComboBoxEx"]);
        var result = new XamlValidator(policy).Validate(xaml, "Views/Main.xaml", "ResourceDictionary");
        Assert.True(result.IsValid, string.Join(Environment.NewLine, result.Violations));
    }

    [Theory]
    // Fora do template de lista suspensa.
    [InlineData("""<ControlTemplate x:Key="t" TargetType="Button"><Popup IsOpen="True" /></ControlTemplate>""")]
    [InlineData("""<DataTemplate x:Key="t"><Popup /></DataTemplate>""")]
    [InlineData("""<Style x:Key="s" TargetType="Popup" />""")]
    [InlineData("""<Style x:Key="s" TargetType="{x:Type Popup}" />""")]
    public void Rejects_popup_outside_combo_box_template(string body) =>
        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.TypeNotAllowed);

    [Theory]
    [InlineData("""<Popup IsOpen="True" />""")]
    [InlineData("""<Popup IsOpen="{Binding IsMouseOver, RelativeSource={RelativeSource TemplatedParent}}" />""")]
    [InlineData("""<Popup Placement="Absolute" />""")]
    [InlineData("""<Popup Placement="Mouse" />""")]
    [InlineData("""<Popup HorizontalOffset="-5000" />""")]
    [InlineData("""<Popup VerticalOffset="100" />""")]
    [InlineData("""<Popup StaysOpen="True" />""")]
    [InlineData("""<Popup Width="100000" />""")]
    [InlineData("""<Popup><Popup.PlacementTarget><Border /></Popup.PlacementTarget></Popup>""")]
    [InlineData("""<Popup><Popup IsOpen="{TemplateBinding IsDropDownOpen}" /></Popup>""")]
    [InlineData("""<ToggleButton><ToggleButton.Template><ControlTemplate TargetType="ToggleButton"><Popup /></ControlTemplate></ToggleButton.Template></ToggleButton>""")]
    public void Rejects_popup_that_could_open_or_move_on_its_own(string popup) =>
        AssertRejected(Validate(ComboBoxTemplate(popup)), XamlViolationCode.TypeNotAllowed);

    [Theory]
    [InlineData("""<Trigger Property="IsMouseOver" Value="True"><Setter TargetName="Popup" Property="IsOpen" Value="True" /></Trigger>""")]
    [InlineData("""<Trigger Property="IsMouseOver" Value="True"><Setter TargetName="Popup" Property="HorizontalOffset" Value="-5000" /></Trigger>""")]
    [InlineData("""<Trigger Property="IsMouseOver" Value="True"><Trigger.EnterActions><BeginStoryboard><Storyboard><DoubleAnimation Storyboard.TargetName="Popup" Storyboard.TargetProperty="VerticalOffset" To="500" /></Storyboard></BeginStoryboard></Trigger.EnterActions></Trigger>""")]
    [InlineData("""<Trigger Property="IsMouseOver" Value="True"><Trigger.EnterActions><BeginStoryboard><Storyboard><DoubleAnimation Storyboard.Target="{Binding ElementName=Popup}" Storyboard.TargetProperty="VerticalOffset" To="500" /></Storyboard></BeginStoryboard></Trigger.EnterActions></Trigger>""")]
    public void Rejects_triggers_that_change_popup_position_or_visibility(string triggers) =>
        AssertRejected(Validate(ComboBoxTemplate(DropDownPopup, triggers)), XamlViolationCode.TypeNotAllowed);

    [Theory]
    [InlineData("""xmlns:p="clr-namespace:System.IO;assembly=System.Runtime" """, """<p:FileInfo x:Key="f" />""")]
    [InlineData("""xmlns:p="clr-namespace:Evil" """, """<Style x:Key="s" p:Evil.Value="1" />""")]
    [InlineData("""xmlns:p="clr-namespace:Evil;assembly=Evil" """, """<TextBlock x:Key="t" Tag="{p:Payload}" />""")]
    [InlineData("""xmlns:p="http://schemas.microsoft.com/xaml/behaviors" """, """<Style x:Key="s" TargetType="Button"><Setter Property="p:Interaction.Behaviors" Value="x" /></Style>""")]
    public void Rejects_used_namespaces_outside_allowlist(string ns, string body) =>
        AssertRejected(Validate(Dictionary(body, ns)), XamlViolationCode.NamespaceNotAllowed);

    [Fact]
    public void Tolerates_unused_disallowed_declaration_and_reports_it_for_removal()
    {
        // Comum em temas Playnite: Constants.xaml declara Interactivity sem usar.
        var result = Validate(Dictionary("""<Style x:Key="s" />""", """xmlns:i="clr-namespace:System.Windows.Interactivity;assembly=System.Windows.Interactivity" """));

        Assert.True(result.IsValid, string.Join(Environment.NewLine, result.Violations));
        var unused = Assert.Single(result.UnusedDisallowedNamespaces);
        Assert.Equal("i", unused.Prefix);
    }

    [Fact]
    public void Default_namespace_outside_allowlist_is_always_rejected() =>
        AssertRejected(Validate("""<ResourceDictionary xmlns="clr-namespace:Evil;assembly=Evil" />"""), XamlViolationCode.NamespaceNotAllowed);

    [Fact]
    public void Extended_policy_allows_adapter_types_and_extensions()
    {
        var policy = XamlSecurityPolicy.CreateDefault().Extend(presentationTypes: ["ButtonEx"], presentationExtensions: ["ThemeFile"]);
        var result = new XamlValidator(policy).Validate(Dictionary("""<ButtonEx x:Key="b" Tag="{ThemeFile 'Images/x.png'}" />"""), "Views/Main.xaml", "ResourceDictionary");

        Assert.True(result.IsValid, string.Join(Environment.NewLine, result.Violations));
        AssertRejected(Validate(Dictionary("""<ButtonEx x:Key="b" />""")), XamlViolationCode.TypeNotAllowed);
    }

    [Fact]
    public void Accepts_spline_thickness_key_frame()
    {
        // Animação de margem (PS5ish, Constants.xaml): só interpola valores; liberada na homologação (MVP-018).
        var result = Validate(Dictionary("""
            <Storyboard x:Key="s"><ThicknessAnimationUsingKeyFrames Storyboard.TargetProperty="Margin">
              <SplineThicknessKeyFrame KeyTime="0:0:0.2" Value="0,10,0,0" KeySpline="0.2,0,0.2,1" />
            </ThicknessAnimationUsingKeyFrames></Storyboard>
            """));

        Assert.True(result.IsValid, string.Join(Environment.NewLine, result.Violations));
    }

    [Fact]
    public void Extended_owner_types_allow_attached_properties_but_not_elements()
    {
        var policy = XamlSecurityPolicy.CreateDefault().Extend(presentationOwnerTypes: ["MediaElementBehaviors"]);
        var validator = new XamlValidator(policy);

        var attached = validator.Validate(Dictionary("""<MediaElement x:Key="m" MediaElementBehaviors.Repeat="True" />"""), "Views/Main.xaml", "ResourceDictionary");
        Assert.True(attached.IsValid, string.Join(Environment.NewLine, attached.Violations));
        AssertRejected(validator.Validate(Dictionary("""<MediaElementBehaviors x:Key="m" />"""), "Views/Main.xaml", "ResourceDictionary"), XamlViolationCode.TypeNotAllowed);
        AssertRejected(Validate(Dictionary("""<MediaElement x:Key="m" MediaElementBehaviors.Repeat="True" />""")), XamlViolationCode.TypeNotAllowed);
    }

    [Fact]
    public void Rejects_dtd_entity_expansion()
    {
        const string xaml = """
            <?xml version="1.0"?>
            <!DOCTYPE lolz [<!ENTITY lol "lol"><!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">]>
            <ResourceDictionary xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation">&lol2;</ResourceDictionary>
            """;

        AssertRejected(Validate(xaml), XamlViolationCode.DtdProhibited);
    }

    [Theory]
    [InlineData("""<Image x:Key="i" Source="https://tracker.example/pixel.png" />""")]
    [InlineData("""<Image x:Key="i" Source="file:///C:/Windows/win.ini" />""")]
    [InlineData("""<Image x:Key="i" Source="C:\Windows\x.png" />""")]
    [InlineData("""<Image x:Key="i" Source="\\server\share\x.png" />""")]
    [InlineData("""<Image x:Key="i" Source="pack://siteoforigin:,,,/x.png" />""")]
    [InlineData("""<ImageBrush x:Key="b" ImageSource="http://example.com/bg.jpg" />""")]
    [InlineData("""<BitmapImage x:Key="b"><BitmapImage.UriSource>http://example.com/x.png</BitmapImage.UriSource></BitmapImage>""")]
    [InlineData("""<Style x:Key="s" TargetType="Image"><Setter Property="Source" Value="https://example.com/x.png" /></Style>""")]
    public void Rejects_external_uris(string body) =>
        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.ExternalUri);

    [Theory]
    [InlineData("""<Image x:Key="i" Source="../../secret.png" />""")]
    [InlineData("""<Image x:Key="i" Source="../Images/../../secret.png" />""")]
    [InlineData("""<Image x:Key="i" Source="/Windows/x.png" />""")]
    [InlineData("""<Image x:Key="i" Source="..\..\secret.png" />""")]
    [InlineData("""<Style x:Key="s" TargetType="Image"><Setter Property="Source" Value="..\..\x.png" /></Style>""")]
    public void Rejects_paths_escaping_theme(string body) =>
        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.PathEscapesTheme);

    [Theory]
    [InlineData("""<Image x:Key="i" Source="../Images/cover.png" />""")]
    [InlineData("""<Image x:Key="i" Source="Images/cover.png" />""")]
    [InlineData("""<Image x:Key="i" Source="pack://application:,,,/Images/cover.png" />""")]
    public void Accepts_theme_relative_and_application_uris(string body)
    {
        var result = Validate(Dictionary(body));

        Assert.True(result.IsValid, string.Join(Environment.NewLine, result.Violations));
    }

    [Fact]
    public void Rejects_nested_dictionary_loading()
    {
        const string body = """
            <ResourceDictionary.MergedDictionaries>
                <ResourceDictionary Source="Common.xaml" />
            </ResourceDictionary.MergedDictionaries>
            """;

        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.NestedLoadNotAllowed);
    }

    [Theory]
    [InlineData("""<TextBlock x:Key="t" Tag="{x:Static Application.Current}" />""")]
    [InlineData("""<TextBlock x:Key="t" Tag="{x:Static Member=Environment.UserName}" />""")]
    [InlineData("""<x:Static x:Key="t" Member="Application.Current" />""")]
    public void Rejects_static_members_outside_allowlist(string body) =>
        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.StaticMemberNotAllowed);

    [Theory]
    [InlineData("""<TextBlock x:Key="t" Tag="{ComponentResourceKey}" />""")]
    [InlineData("""<TextBlock x:Key="t" Text="{Binding Name, Converter={ThemeDictionary Foo}}" />""")]
    public void Rejects_markup_extensions_outside_allowlist(string body) =>
        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.MarkupExtensionNotAllowed);

    [Theory]
    [InlineData("""<mc:AlternateContent><mc:Choice Requires="x"><Style x:Key="s" /></mc:Choice></mc:AlternateContent>""", "")]
    [InlineData("""<Style x:Key="s" mc:ProcessContent="x" />""", "")]
    [InlineData("", """mc:Ignorable="x" """)]
    public void Rejects_markup_compatibility_tricks(string body, string rootAttributes)
    {
        string xaml = Dictionary(body, """xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" """ + rootAttributes);

        AssertRejected(Validate(xaml), XamlViolationCode.MarkupCompatibilityNotAllowed);
    }

    [Fact]
    public void Rejects_processing_instructions()
    {
        string xaml = """<?Mapping XmlNamespace="evil" ClrNamespace="Evil" Assembly="Evil"?>""" + Dictionary("");

        AssertRejected(Validate(xaml), XamlViolationCode.DirectiveNotAllowed);
    }

    [Fact]
    public void Rejects_attached_property_with_disallowed_owner() =>
        AssertRejected(Validate(Dictionary("""<Border x:Key="b" InkCanvas.Left="10" />""")), XamlViolationCode.TypeNotAllowed);

    [Fact]
    public void Rejects_unexpected_root() =>
        AssertRejected(Validate($"<Grid {Namespaces} />"), XamlViolationCode.UnexpectedRoot);

    [Fact]
    public void Rejects_excessive_nesting()
    {
        string body = string.Concat(Enumerable.Repeat("<Border>", 200)) + string.Concat(Enumerable.Repeat("</Border>", 200));

        AssertRejected(Validate(Dictionary(body)), XamlViolationCode.LimitExceeded);
    }

    [Fact]
    public void Reports_line_of_violation()
    {
        string xaml = $"<ResourceDictionary {Namespaces}>\n  <Frame x:Key=\"f\" />\n</ResourceDictionary>";

        var violation = Assert.Single(Validate(xaml).Violations);

        Assert.Equal(2, violation.Line);
    }
}
