using System.IO;
using System.Windows;
using System.Windows.Data;
using System.Windows.Markup;
using Launcher.PlayniteCompat.P10.Controls;
using Launcher.PlayniteCompat.P10.Data;
using Launcher.PlayniteCompat.P10.Markup;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

/// <summary>Adaptadores de dados e controles PART_ do tema Playnite na interface inteira (MVP-018/019).</summary>
public class PlayniteAdapterTests
{
    private static readonly string[] InstalledIds = ["a", "c"];

    [Fact]
    public void Theme_file_paths_never_leave_the_theme_folder()
    {
        string root = Path.Combine(Path.GetTempPath(), $"tako-tema-{Guid.NewGuid():N}");
        Directory.CreateDirectory(Path.Combine(root, "Icons", "Library"));
        File.WriteAllText(Path.Combine(root, "theme.yaml"), "Id: x");
        File.WriteAllText(Path.Combine(root, "Icons", "Library", "Steam.png"), "x");
        File.WriteAllText(Path.Combine(Path.GetTempPath(), "fora-do-tema.png"), "x");
        try
        {
            Assert.Equal(Path.Combine(root, "Icons", "Library", "Steam.png"), ThemeFiles.Resolve(root, "Icons/Library/Steam.png"));
            Assert.Null(ThemeFiles.Resolve(root, "Icons/Library/Epic.png")); // não existe
            Assert.Null(ThemeFiles.Resolve(root, "../fora-do-tema.png"));
            Assert.Null(ThemeFiles.Resolve(root, "Icons/../../fora-do-tema.png"));
            Assert.Null(ThemeFiles.Resolve(root, Path.Combine(Path.GetTempPath(), "fora-do-tema.png")));
            Assert.Null(ThemeFiles.Resolve(root, @"\\servidor\compartilhamento\x.png"));
            Assert.Null(ThemeFiles.Resolve(root, "https://rastreio.example/x.png"));
            Assert.Null(ThemeFiles.Resolve(null, "Icons/Library/Steam.png"));
        }
        finally
        {
            Directory.Delete(root, recursive: true);
            File.Delete(Path.Combine(Path.GetTempPath(), "fora-do-tema.png"));
        }
    }

    [Fact]
    public void Html_text_view_shows_plain_text_only() => Sta.Run(() =>
    {
        var view = new HtmlTextView { HtmlText = "<p>Um <b>jogo</b> &amp; mais</p><script>alert(1)</script><img src=\"https://rastreio.example/p.gif\">" };

        Assert.Equal("Um jogo & mais", view.Text);
        Assert.Empty(view.Inlines.OfType<System.Windows.Documents.Hyperlink>());
    });

    [Fact]
    public void Plugin_extensions_behave_as_if_the_plugin_were_not_installed() => Sta.Run(() =>
    {
        var trigger = new DataTrigger();
        var bindingProperty = typeof(DataTrigger).GetProperty(nameof(DataTrigger.Binding))!;

        var status = Assert.IsType<Binding>(new PluginStatusExtension { Plugin = "MoData", Status = "Installed" }.ProvideValue(new Target(trigger, bindingProperty)));
        Assert.False((bool)Assert.IsType<SettingsBindingExtension.SettingValue>(status.Source).Value!);

        var settings = Assert.IsType<Binding>(new PluginSettingsExtension { Plugin = "SuccessStory", Path = "HasData" }.ProvideValue(new Target(trigger, bindingProperty)));
        Assert.Null(Assert.IsType<SettingsBindingExtension.SettingValue>(settings.Source).Value);

        // Num DependencyProperty o FallbackValue (texto do XAML) é convertido para o tipo da propriedade.
        var element = new FrameworkElement();
        Assert.Equal(Visibility.Collapsed, new PluginStatusExtension().ProvideValue(new Target(element, UIElement.VisibilityProperty)));
        Assert.False((bool)new PluginSettingsExtension { FallbackValue = "False" }.ProvideValue(new Target(element, UIElement.FocusableProperty))!);
    });

    [Fact]
    public void Api_extension_reads_the_inherited_adapter_api_one_way() => Sta.Run(() =>
    {
        var model = new P10MainModel(new FakeHost(), []);
        var parent = new System.Windows.Controls.Border();
        var child = new System.Windows.Controls.TextBlock();
        parent.Child = child;
        P10Context.SetApi(parent, new P10Api(model));

        var trigger = new DataTrigger();
        var binding = Assert.IsType<Binding>(new ApiExtension("Notifications.Count").ProvideValue(new Target(trigger, typeof(DataTrigger).GetProperty(nameof(DataTrigger.Binding))!)));
        Assert.Equal(BindingMode.OneWay, binding.Mode);

        child.SetBinding(System.Windows.Controls.TextBlock.TagProperty, binding);
        Assert.Equal(0, child.Tag);
        Assert.Same(model, P10Context.GetApi(child)!.MainModel);
    });

    [Fact]
    public void Settings_binding_writes_only_its_own_copy() => Sta.Run(() =>
    {
        var setter = new Setter();
        var binding = Assert.IsType<Binding>(new SettingsBindingExtension("Fullscreen.IsMusicMuted") { Mode = BindingMode.TwoWay }.ProvideValue(new Target(setter, typeof(Setter).GetProperty(nameof(Setter.Value))!)));
        var copy = Assert.IsType<SettingsBindingExtension.SettingValue>(binding.Source);
        copy.Value = false;

        var again = Assert.IsType<Binding>(new SettingsBindingExtension("Fullscreen.IsMusicMuted").ProvideValue(new Target(setter, typeof(Setter).GetProperty(nameof(Setter.Value))!)));
        Assert.True((bool)((SettingsBindingExtension.SettingValue)again.Source).Value!);
    });

    [Fact]
    public void Main_model_filters_selects_and_opens_details()
    {
        var host = new FakeHost();
        var model = new P10MainModel(host,
        [
            new P10FilterPreset("Todos", _ => true),
            new P10FilterPreset("Instalados", g => g.IsInstalled),
        ]);
        model.SetGames(
        [
            new P10GameEntry("a", "Alfa") { IsInstalled = true },
            new P10GameEntry("b", "Beta"),
            new P10GameEntry("c", "Gama") { IsInstalled = true },
        ]);

        Assert.Equal(3, model.Games.Count);
        Assert.Equal("a", model.SelectedGame!.Id);
        model.SelectedGame = model.Games[2];
        Assert.Equal("c", host.LastSelection?.Id);

        // Filtro rápido (PART_ButtonFilter / FilterPresetSelector): mantém o selecionado quando ele continua na lista.
        model.ToggleFiltersCommand.Execute(null);
        Assert.Equal("Instalados", model.ActiveFilterPreset!.Name);
        Assert.True(model.ActiveFilterPreset.IsActive);
        Assert.False(model.FilterPresets[0].IsActive);
        Assert.Equal(InstalledIds, model.Games.Select(g => g.Id));
        Assert.Equal("c", model.SelectedGame!.Id);

        // A na capa abre os detalhes; B fecha; Jogar chama o launcher com o jogo selecionado.
        model.ToggleGameDetailsCommand.Execute(model.Games[0]);
        Assert.True(model.GameDetailsVisible);
        Assert.Equal("a", model.SelectedGame!.Id);
        model.ActivateSelectedCommand.Execute(null);
        Assert.Equal("a", host.Activated?.Id);
        Assert.True(model.HandleBack());
        Assert.False(model.GameDetailsVisible);
        Assert.False(model.HandleBack());

        // Menu principal do tema → Configurações do Tako.
        model.ToggleMainMenuCommand.Execute(null);
        Assert.True(host.MenuOpened);
    }

    private sealed class FakeHost : IP10Host
    {
        public P10GameEntry? Activated { get; private set; }

        public P10GameEntry? LastSelection { get; private set; }

        public bool MenuOpened { get; private set; }

        public void Activate(P10GameEntry game) => Activated = game;

        public string ActionLabel(P10GameEntry game) => game.IsInstalled ? "Jogar" : "Instalar";

        public void ShowGameOptions(P10GameEntry game)
        {
        }

        public void OpenMainMenu() => MenuOpened = true;

        public void OpenSearch()
        {
        }

        public void OpenNotifications()
        {
        }

        public void SelectionChanged(P10GameEntry? game) => LastSelection = game;
    }

    private sealed class Target(object targetObject, object targetProperty) : IServiceProvider, IProvideValueTarget
    {
        public object TargetObject { get; } = targetObject;

        public object TargetProperty { get; } = targetProperty;

        public object? GetService(Type serviceType) => serviceType == typeof(IProvideValueTarget) ? this : null;
    }
}

/// <summary>Roda o teste numa thread STA (controles WPF).</summary>
internal static class Sta
{
    public static void Run(Action action)
    {
        Exception? failure = null;
        var thread = new Thread(() =>
        {
            try
            {
                action();
            }
            catch (Exception ex) when (ex is not OutOfMemoryException)
            {
                failure = ex;
            }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        thread.Join();
        if (failure is not null)
        {
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(failure).Throw();
        }
    }
}
