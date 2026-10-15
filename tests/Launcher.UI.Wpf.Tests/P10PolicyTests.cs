using Launcher.PlayniteCompat.P10;
using Launcher.Themes.Xaml;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

/// <summary>
/// Liberações do adaptador P10 para a interface inteira (MVP-018/019): cada uma entra por nome, e o que fica perto
/// dela continua recusado. Só validação (sem WPF): roda em qualquer sistema.
/// </summary>
public class P10PolicyTests
{
    private const string Namespaces =
        """xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" """;

    private const string PlayniteNamespaces =
        """ xmlns:pbeh="clr-namespace:Playnite.Behaviors;assembly=Playnite" xmlns:pin="clr-namespace:Playnite.Input;assembly=Playnite" """;

    private readonly XamlValidator _validator = new(P10Policy.Create(XamlSecurityPolicy.CreateDefault()));

    private XamlValidationResult Validate(string body, string extraNamespaces = PlayniteNamespaces) =>
        _validator.Validate($"<ResourceDictionary {Namespaces}{extraNamespaces}>{body}</ResourceDictionary>", "Views/Main.xaml", "ResourceDictionary");

    [Theory]
    [InlineData("""<TextBlock x:Key="t" Text="{PluginSettings Plugin=SuccessStory, Path=Total, FallbackValue='-'}" />""")]
    [InlineData("""<Style x:Key="s" TargetType="Button"><Style.Triggers><DataTrigger Binding="{PluginStatus Plugin=MoData_5a0a, Status=Installed}" Value="False"><Setter Property="Visibility" Value="Collapsed" /></DataTrigger></Style.Triggers></Style>""")]
    [InlineData("""<Style x:Key="s" TargetType="ToggleButton"><Setter Property="IsChecked" Value="{SettingsBinding Fullscreen.IsMusicMuted, Mode=TwoWay}" /></Style>""")]
    [InlineData("""<Image x:Key="i" Source="{ThemeFileBinding Library, PathFormat='Icons/Library/{0}.png'}" />""")]
    [InlineData("""<Style x:Key="s" TargetType="Button"><Style.Triggers><DataTrigger Binding="{Api Notifications.Count}" Value="0"><Setter Property="Visibility" Value="Collapsed" /></DataTrigger></Style.Triggers></Style>""")]
    [InlineData("""<TextBlock x:Key="t" Text="{Api MainModel.SelectedGame.Name}" />""")]
    [InlineData("""<CheckBox x:Key="c" IsChecked="{Settings Fullscreen.HorizontalLayout, Mode=TwoWay}" />""")]
    [InlineData("""<HtmlTextView x:Key="h" HtmlFontSize="16" HtmlForeground="{DynamicResource TextColor}" LinkForeground="White" />""")]
    [InlineData("""<Style x:Key="m" TargetType="{x:Type Main}" />""")]
    [InlineData("""<MediaElement x:Key="m" MediaElementBehaviors.Repeat="True" />""")]
    [InlineData("""<ListView x:Key="l" pbeh:FocusBahaviors.FocusBinding="True" pbeh:FocusBahaviors.OnVisibilityFocus="True" />""")]
    [InlineData("""<Grid x:Key="g"><Grid.InputBindings><pin:GameControllerInputBinding Button="B" Command="{Binding Content.ChangeProperty, ElementName=x}" /></Grid.InputBindings></Grid>""")]
    public void Accepts_playnite_extensions_and_types_implemented_by_the_adapter(string body)
    {
        var result = Validate(body);
        Assert.True(result.IsValid, string.Join(Environment.NewLine, result.Violations));
    }

    [Theory]
    // Behaviors e entrada do Playnite: só os nomes implementados, nunca o namespace inteiro.
    [InlineData("""<Grid x:Key="g" pbeh:SliderBehaviors.Anything="True" />""", XamlViolationCode.TypeNotAllowed)]
    [InlineData("""<Grid x:Key="g"><Grid.InputBindings><pin:KeyboardInputBinding /></Grid.InputBindings></Grid>""", XamlViolationCode.TypeNotAllowed)]
    // Donos de propriedades anexadas não viram elementos.
    [InlineData("""<MediaElementBehaviors x:Key="m" />""", XamlViolationCode.TypeNotAllowed)]
    // Arquivos fora do tema continuam bloqueados, inclusive como origem de binding (PS5ish e Revisited, Main.xaml).
    // {ThemeFile} e {ThemeFileBinding} são protegidos na execução (PlayniteAdapterTests).
    [InlineData("""<TextBlock x:Key="t"><TextBlock.Text><MultiBinding StringFormat="{}{0}{1}"><Binding Source="\ExtraMetadata\games\" /><Binding Path="Id" /></MultiBinding></TextBlock.Text></TextBlock>""", XamlViolationCode.PathEscapesTheme)]
    // Extensões do Playnite fora do subset.
    [InlineData("""<TextBlock x:Key="t" Text="{ExtensionSettings Foo}" />""", XamlViolationCode.MarkupExtensionNotAllowed)]
    public void Keeps_rejecting_what_the_adapter_does_not_implement(string body, XamlViolationCode code)
    {
        var result = Validate(body);
        Assert.Contains(result.Violations, v => v.Code == code);
    }

    [Fact]
    public void Base_policy_without_adapter_still_rejects_playnite_extensions()
    {
        var plain = new XamlValidator(XamlSecurityPolicy.CreateDefault());
        var result = plain.Validate($"<ResourceDictionary {Namespaces}><TextBlock x:Key=\"t\" Text=\"{{Api Notifications.Count}}\" /><HtmlTextView x:Key=\"h\" /></ResourceDictionary>", "Views/Main.xaml", "ResourceDictionary");

        Assert.Contains(result.Violations, v => v.Code == XamlViolationCode.MarkupExtensionNotAllowed);
        Assert.Contains(result.Violations, v => v.Code == XamlViolationCode.TypeNotAllowed);
    }
}
