using System.IO;
using System.Runtime.ExceptionServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Launcher.Core.Themes;
using Launcher.Themes.Security;
using Launcher.Themes.Xaml;
using Xunit;

namespace Launcher.Themes.Tests.Security;

/// <summary>
/// Frente H (questão aberta 8 do MVP-016): um tema que passa na validação estática monta caminhos em tempo de execução
/// (<c>MultiBinding</c>, <c>sys:String</c>, <c>StringFormat</c>) e os liga a <c>Image.Source</c>,
/// <c>ImageBrush.ImageSource</c> e <c>MediaElement.Source</c>. A guarda de mídia não deixa nada disso carregar.
/// </summary>
public sealed class ThemeMediaGuardTests : IDisposable
{
    private readonly string _work = Directory.CreateTempSubdirectory("tako-theme-guard-").FullName;

    public void Dispose()
    {
        try
        {
            Directory.Delete(_work, recursive: true);
        }
        catch (IOException)
        {
            // Arquivo de imagem ainda aberto pelo WPF; a pasta temporária fica para o sistema limpar.
        }
    }

    private string ThemeRoot => Path.Combine(_work, "themes", "Malicioso");

    [Fact]
    public void Runtime_composed_paths_never_load()
    {
        Directory.CreateDirectory(Path.Combine(ThemeRoot, "Images"));
        Directory.CreateDirectory(Path.Combine(ThemeRoot, "Views"));
        File.WriteAllText(Path.Combine(ThemeRoot, "theme.yaml"), "Id: Teste_Malicioso\nName: Malicioso\nMode: Fullscreen\nThemeApiVersion: 2.0.0\n");
        string inside = Path.Combine(ThemeRoot, "Images", "ok.png");
        string outside = Path.Combine(_work, "fora.png");
        RunOnSta(() =>
        {
            WritePng(inside);
            WritePng(outside);
            return 0;
        });

        // Caminhos absolutos divididos em dois recursos: nenhum pedaço parece caminho para a validação estática.
        File.WriteAllText(Path.Combine(ThemeRoot, "Views", "Evil.xaml"), $$$"""
            <ResourceDictionary xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
                                xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
                                xmlns:sys="clr-namespace:System;assembly=mscorlib">
                <sys:String x:Key="Sep">\</sys:String>
                <sys:String x:Key="Host">exemplo\share\x.png</sys:String>
                <sys:String x:Key="Proto">http</sys:String>
                <sys:String x:Key="Slash">/</sys:String>
                <sys:String x:Key="Web">exemplo/x.png</sys:String>
                <sys:String x:Key="Up">..\..\..\Windows\win.ini</sys:String>
                <sys:String x:Key="Drive">{{{outside[..1]}}}</sys:String>
                <sys:String x:Key="OutsideRest">{{{outside[1..]}}}</sys:String>
                <sys:String x:Key="InsideRest">{{{inside[1..]}}}</sys:String>
                <ControlTemplate x:Key="Evil" TargetType="ContentControl">
                    <StackPanel>
                        <TextBlock x:Name="Unc" Tag="{StaticResource Sep}">
                            <TextBlock.Text>
                                <MultiBinding StringFormat="{}{0}{0}{1}">
                                    <Binding Path="Tag" RelativeSource="{RelativeSource Self}" />
                                    <Binding Source="{StaticResource Host}" />
                                </MultiBinding>
                            </TextBlock.Text>
                        </TextBlock>
                        <TextBlock x:Name="Http">
                            <TextBlock.Text>
                                <MultiBinding StringFormat="{}{0}:{1}{1}{2}">
                                    <Binding Source="{StaticResource Proto}" />
                                    <Binding Source="{StaticResource Slash}" />
                                    <Binding Source="{StaticResource Web}" />
                                </MultiBinding>
                            </TextBlock.Text>
                        </TextBlock>
                        <TextBlock x:Name="Parent" Text="{Binding Source={StaticResource Up}, StringFormat={}{0}}" />
                        <TextBlock x:Name="Outside">
                            <TextBlock.Text>
                                <MultiBinding StringFormat="{}{0}{1}">
                                    <Binding Source="{StaticResource Drive}" />
                                    <Binding Source="{StaticResource OutsideRest}" />
                                </MultiBinding>
                            </TextBlock.Text>
                        </TextBlock>
                        <TextBlock x:Name="Inside">
                            <TextBlock.Text>
                                <MultiBinding StringFormat="{}{0}{1}">
                                    <Binding Source="{StaticResource Drive}" />
                                    <Binding Source="{StaticResource InsideRest}" />
                                </MultiBinding>
                            </TextBlock.Text>
                        </TextBlock>
                        <Image x:Name="ImageUnc" Source="{Binding Text, ElementName=Unc}" />
                        <Image x:Name="ImageHttp" Source="{Binding Text, ElementName=Http}" />
                        <Image x:Name="ImageParent" Source="{Binding Text, ElementName=Parent}" />
                        <Image x:Name="ImageOutside" Source="{Binding Text, ElementName=Outside}" />
                        <Image x:Name="ImageTag" Tag="{Binding Text, ElementName=Unc}" Source="{Binding Tag, RelativeSource={RelativeSource Self}}" />
                        <Image x:Name="ImageInside" Source="{Binding Text, ElementName=Inside}" />
                        <Border x:Name="BrushHttp" Width="8" Height="8">
                            <Border.Background>
                                <ImageBrush ImageSource="{Binding Text, ElementName=Http}" />
                            </Border.Background>
                        </Border>
                        <MediaElement x:Name="MediaUnc" LoadedBehavior="Manual" Source="{Binding Text, ElementName=Unc}" />
                        <MediaElement x:Name="MediaParent" LoadedBehavior="Manual" Source="{Binding Text, ElementName=Parent}" />
                    </StackPanel>
                </ControlTemplate>
            </ResourceDictionary>
            """);

        var blocked = new List<ThemeMediaBlockedEventArgs>();
        void OnBlocked(object? sender, ThemeMediaBlockedEventArgs e)
        {
            lock (blocked)
            {
                blocked.Add(e);
            }
        }

        ThemeMediaGuard.Blocked += OnBlocked;
        try
        {
            var result = RunOnSta(() =>
            {
                Assert.True(ThemeMediaGuard.EnsureInstalled(out string? problem), problem);
                ThemeSourcePolicy.Shared.AllowTheme(ThemeRoot);
                var loader = new RestrictedXamlLoader(new XamlValidator(XamlSecurityPolicy.CreateDefault()));
                var dictionary = loader.LoadResourceDictionary(ThemeRoot, "Views/Evil.xaml");
                var host = new ContentControl { Template = (ControlTemplate)dictionary["Evil"] };
                var window = new Window
                {
                    Content = host,
                    Width = 320,
                    Height = 480,
                    Left = -20000,
                    Top = -20000,
                    ShowInTaskbar = false,
                    ShowActivated = false,
                    WindowStyle = WindowStyle.None,
                };
                window.Show();
                Pump(TimeSpan.FromSeconds(1.5));

                T Part<T>(string name) => (T)host.Template.FindName(name, host);
                var sources = new Dictionary<string, object?>
                {
                    ["UNC (MultiBinding + sys:String)"] = Part<Image>("ImageUnc").Source,
                    ["http (MultiBinding)"] = Part<Image>("ImageHttp").Source,
                    [@"..\..\..\Windows\win.ini (StringFormat)"] = Part<Image>("ImageParent").Source,
                    ["arquivo fora do tema"] = Part<Image>("ImageOutside").Source,
                    ["UNC via Tag (object)"] = Part<Image>("ImageTag").Source,
                    ["ImageBrush http"] = ((ImageBrush)Part<Border>("BrushHttp").Background).ImageSource,
                    ["MediaElement UNC"] = Part<MediaElement>("MediaUnc").Source,
                    ["MediaElement .."] = Part<MediaElement>("MediaParent").Source,
                };
                var composed = new[] { Part<TextBlock>("Unc").Text, Part<TextBlock>("Http").Text, Part<TextBlock>("Outside").Text };
                object? insideSource = Part<Image>("ImageInside").Source;
                window.Close();
                return (sources, composed, insideSource);
            });

            // O tema conseguiu montar os caminhos (o ataque é real)...
            Assert.Equal(@"\\exemplo\share\x.png", result.composed[0]);
            Assert.Equal("http://exemplo/x.png", result.composed[1]);
            Assert.Equal(outside, result.composed[2]);

            // ...e nenhum deles virou imagem ou mídia.
            Assert.All(result.sources, s => Assert.True(s.Value is null, $"{s.Key} carregou: {s.Value}"));

            // Controle positivo: o mesmo caminho montado, dentro da pasta do tema, carrega.
            Assert.IsAssignableFrom<BitmapSource>(result.insideSource);

            lock (blocked)
            {
                Assert.Contains(blocked, b => b.Reason == ThemeSourceBlock.Network && b.TargetType == typeof(ImageSource));
                Assert.Contains(blocked, b => b.Reason == ThemeSourceBlock.Network && b.TargetType == typeof(Uri));
                Assert.Contains(blocked, b => b.Reason == ThemeSourceBlock.External && b.TargetType == typeof(ImageSource));
                Assert.Contains(blocked, b => b.Reason == ThemeSourceBlock.OutsideTheme);

                // O resumo do log não leva pastas do usuário.
                Assert.DoesNotContain(blocked, b => b.Origin.Contains(_work, StringComparison.OrdinalIgnoreCase));
            }
        }
        finally
        {
            ThemeMediaGuard.Blocked -= OnBlocked;
        }
    }

    [Fact]
    public void Guard_is_installed_once_per_thread() =>
        Assert.True(RunOnSta(() => ThemeMediaGuard.EnsureInstalled(out _) && ThemeMediaGuard.EnsureInstalled(out _)));

    private static void WritePng(string path)
    {
        var pixels = new byte[4 * 4 * 4];
        Array.Fill(pixels, (byte)0xFF);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(BitmapSource.Create(4, 4, 96, 96, PixelFormats.Bgra32, null, pixels, 16)));
        using var file = File.Create(path);
        encoder.Save(file);
    }

    private static void Pump(TimeSpan duration)
    {
        var frame = new DispatcherFrame();
        var timer = new DispatcherTimer(DispatcherPriority.Background) { Interval = duration };
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            frame.Continue = false;
        };
        timer.Start();
        Dispatcher.PushFrame(frame);
    }

    /// <summary>Objetos WPF exigem thread STA.</summary>
    private static T RunOnSta<T>(Func<T> action)
    {
        T result = default!; // Atribuído pela thread ou exceção relançada abaixo.
        ExceptionDispatchInfo? failure = null;
        var thread = new Thread(() =>
        {
            try
            {
                result = action();
            }
            catch (Exception ex) when (ex is not OutOfMemoryException)
            {
                failure = ExceptionDispatchInfo.Capture(ex);
            }
            finally
            {
                Dispatcher.CurrentDispatcher.InvokeShutdown();
            }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        thread.Join();
        failure?.Throw();
        return result;
    }
}
