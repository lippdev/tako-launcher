using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Launcher.Core.Themes;
using Launcher.Themes.Security;
using Launcher.UI.Wpf.Controls;
using Launcher.UI.Wpf.Services;
using Microsoft.Extensions.DependencyInjection;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// Frente H (MVP-016, questão aberta 8): cópia do Fullscreen padrão com um <c>Constants.xaml</c> malicioso que passa na
    /// validação estática e monta, em tempo de execução, <c>\\exemplo\share\x.png</c>, <c>http://exemplo/x.png</c>,
    /// <c>..\..\..\Windows\win.ini</c> e um arquivo fora do tema (MultiBinding, sys:String e StringFormat), ligados a
    /// Image, ImageBrush e MediaElement dentro de um <see cref="ThemeScope"/>. Nada pode carregar, o mesmo caminho dentro
    /// do tema carrega, e o tema continua com os 33 arquivos. O relatório vai para <c>theme-security.txt</c>.
    /// </summary>
    private static void ThemeSecurity(ServiceProvider services, BindingErrorListener bindingErrors)
    {
        var playnite = services.GetRequiredService<PlayniteThemeService>();
        int bindingErrorsBefore = bindingErrors.Errors.Count;
        string work = Path.Combine(Path.GetTempPath(), "tako-frente-h-" + Guid.NewGuid().ToString("N")[..8]);
        string theme = Path.Combine(work, "Malicioso");
        string outside = Path.Combine(work, "fora.png");
        string inside = Path.Combine(theme, "Images", "dentro.png");
        CopyDirectory(playnite.BuiltIn().Directory, theme);
        File.WriteAllText(Path.Combine(theme, "theme.yaml"), "Id: Tako_Teste_Malicioso\nName: Malicioso\nAuthor: Teste\nVersion: 1.0\nMode: Fullscreen\nThemeApiVersion: 2.0.0\n");
        WritePng(outside);
        WritePng(inside);

        string constants = Path.Combine(theme, "Constants.xaml");
        string original = File.ReadAllText(constants);
        int end = original.LastIndexOf("</ResourceDictionary>", StringComparison.Ordinal);
        File.WriteAllText(constants, original[..end] + MaliciousResources(outside, inside) + original[end..]);

        var blocked = new List<ThemeMediaBlockedEventArgs>();
        void OnBlocked(object? sender, ThemeMediaBlockedEventArgs e) => blocked.Add(e);
        ThemeMediaGuard.Blocked += OnBlocked;
        var lines = new List<string>();
        try
        {
            var report = playnite.LoadDirectory(theme);
            lines.Add($"Tema malicioso: {report.Loaded.Count} de {report.Total} carregados; recusas: {string.Join(" | ", report.Rejected.Select(x => $"{x.File}: {x.Reason}"))}");
            Assert.Contains("Constants.xaml", report.Loaded); // passa na validação estática: o ataque é em tempo de execução
            Assert.Equal(33, report.Loaded.Count);

            var probe = new ContentControl { Template = (ControlTemplate)report.Resources["TakoTesteMalicioso"] };
            var host = new Window
            {
                Content = new ThemeScope { ThemeResources = report.Resources, Content = probe },
                Width = 320,
                Height = 480,
                Left = -20000,
                Top = -20000,
                ShowInTaskbar = false,
                ShowActivated = false,
                WindowStyle = WindowStyle.None,
            };
            host.Show();
            Pump(2);

            T Part<T>(string name) => (T)probe.Template.FindName(name, probe);
            var sources = new Dictionary<string, object?>
            {
                ["Image UNC (MultiBinding + sys:String)"] = Part<Image>("ImageUnc").Source,
                ["Image http (MultiBinding + StringFormat)"] = Part<Image>("ImageHttp").Source,
                [@"Image ..\..\..\Windows\win.ini (StringFormat)"] = Part<Image>("ImageParent").Source,
                ["Image fora do tema"] = Part<Image>("ImageOutside").Source,
                ["ImageBrush UNC"] = ((ImageBrush)Part<Border>("BrushUnc").Background).ImageSource,
                ["MediaElement http"] = Part<MediaElement>("MediaHttp").Source,
                ["MediaElement .."] = Part<MediaElement>("MediaParent").Source,
            };
            string[] composed = [Part<TextBlock>("Unc").Text, Part<TextBlock>("Http").Text, Part<TextBlock>("Parent").Text, Part<TextBlock>("Outside").Text];
            object? insideSource = Part<Image>("ImageInside").Source;
            host.Close();

            lines.AddRange(composed.Select(c => "  montado: " + c.Replace(work, "<tmp>", StringComparison.OrdinalIgnoreCase)));
            lines.AddRange(sources.Select(s => $"  {s.Key}: {(s.Value is null ? "nada carregado" : "CARREGOU " + s.Value)}"));
            lines.Add($"  Image dentro do tema (controle positivo): {(insideSource is null ? "NÃO carregou" : "carregou")}");
            lines.AddRange(blocked.Select(b => $"  bloqueado: {b.Reason} → {b.TargetType.Name} ({b.Origin})"));

            Assert.Equal([@"\\exemplo\share\x.png", "http://exemplo/x.png", @"..\..\..\Windows\win.ini", outside], composed);
            Assert.All(sources, s => Assert.True(s.Value is null, $"{s.Key} carregou {s.Value}"));
            Assert.IsAssignableFrom<BitmapSource>(insideSource);
            Assert.Contains(blocked, b => b.Reason == ThemeSourceBlock.Network);
            Assert.Contains(blocked, b => b.Reason == ThemeSourceBlock.External && b.TargetType == typeof(Uri));
            Assert.Contains(blocked, b => b.Reason == ThemeSourceBlock.OutsideTheme);
            Assert.DoesNotContain(blocked, b => b.Origin.Contains(work, StringComparison.OrdinalIgnoreCase));
        }
        finally
        {
            ThemeMediaGuard.Blocked -= OnBlocked;
            File.WriteAllLines(Path.Combine(Screenshots, "theme-security.txt"), lines);
            bindingErrors.Errors.RemoveRange(bindingErrorsBefore, bindingErrors.Errors.Count - bindingErrorsBefore);
            try
            {
                Directory.Delete(work, recursive: true);
            }
            catch (IOException)
            {
                // Imagem do controle positivo ainda aberta pelo WPF: a pasta temporária fica para o sistema.
            }
        }
    }

    /// <summary>Cada caminho é montado de pedaços que, sozinhos, não parecem caminho para a validação estática.</summary>
    private static string MaliciousResources(string outside, string inside) => $$$"""
            <sys:String x:Key="TakoSep">\</sys:String>
            <sys:String x:Key="TakoHost">exemplo\share\x.png</sys:String>
            <sys:String x:Key="TakoProto">http</sys:String>
            <sys:String x:Key="TakoSlash">/</sys:String>
            <sys:String x:Key="TakoWeb">exemplo/x.png</sys:String>
            <sys:String x:Key="TakoUp">..\..\..\Windows\win.ini</sys:String>
            <sys:String x:Key="TakoDrive">{{{outside[..1]}}}</sys:String>
            <sys:String x:Key="TakoOutside">{{{outside[1..]}}}</sys:String>
            <sys:String x:Key="TakoInside">{{{inside[1..]}}}</sys:String>
            <ControlTemplate x:Key="TakoTesteMalicioso" TargetType="ContentControl">
                <StackPanel>
                    <TextBlock x:Name="Unc" Tag="{StaticResource TakoSep}">
                        <TextBlock.Text>
                            <MultiBinding StringFormat="{}{0}{0}{1}">
                                <Binding Path="Tag" RelativeSource="{RelativeSource Self}" />
                                <Binding Source="{StaticResource TakoHost}" />
                            </MultiBinding>
                        </TextBlock.Text>
                    </TextBlock>
                    <TextBlock x:Name="Http">
                        <TextBlock.Text>
                            <MultiBinding StringFormat="{}{0}:{1}{1}{2}">
                                <Binding Source="{StaticResource TakoProto}" />
                                <Binding Source="{StaticResource TakoSlash}" />
                                <Binding Source="{StaticResource TakoWeb}" />
                            </MultiBinding>
                        </TextBlock.Text>
                    </TextBlock>
                    <TextBlock x:Name="Parent" Text="{Binding Source={StaticResource TakoUp}, StringFormat={}{0}}" />
                    <TextBlock x:Name="Outside">
                        <TextBlock.Text>
                            <MultiBinding StringFormat="{}{0}{1}">
                                <Binding Source="{StaticResource TakoDrive}" />
                                <Binding Source="{StaticResource TakoOutside}" />
                            </MultiBinding>
                        </TextBlock.Text>
                    </TextBlock>
                    <TextBlock x:Name="Inside">
                        <TextBlock.Text>
                            <MultiBinding StringFormat="{}{0}{1}">
                                <Binding Source="{StaticResource TakoDrive}" />
                                <Binding Source="{StaticResource TakoInside}" />
                            </MultiBinding>
                        </TextBlock.Text>
                    </TextBlock>
                    <Image x:Name="ImageUnc" Source="{Binding Text, ElementName=Unc}" />
                    <Image x:Name="ImageHttp" Source="{Binding Text, ElementName=Http}" />
                    <Image x:Name="ImageParent" Source="{Binding Text, ElementName=Parent}" />
                    <Image x:Name="ImageOutside" Source="{Binding Text, ElementName=Outside}" />
                    <Image x:Name="ImageInside" Source="{Binding Text, ElementName=Inside}" />
                    <Border x:Name="BrushUnc" Width="8" Height="8">
                        <Border.Background>
                            <ImageBrush ImageSource="{Binding Text, ElementName=Unc}" />
                        </Border.Background>
                    </Border>
                    <MediaElement x:Name="MediaHttp" LoadedBehavior="Manual" Source="{Binding Text, ElementName=Http}" />
                    <MediaElement x:Name="MediaParent" LoadedBehavior="Manual" Source="{Binding Text, ElementName=Parent}" />
                </StackPanel>
            </ControlTemplate>
        """;

    private static void CopyDirectory(string source, string target)
    {
        foreach (string file in Directory.EnumerateFiles(source, "*", SearchOption.AllDirectories))
        {
            string destination = Path.Combine(target, Path.GetRelativePath(source, file));
            Directory.CreateDirectory(Path.GetDirectoryName(destination)!); // Combine com arquivo sempre tem diretório.
            File.Copy(file, destination);
        }
    }

    private static void WritePng(string path)
    {
        byte[] pixels = new byte[4 * 4 * 4];
        Array.Fill(pixels, (byte)0xFF);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(BitmapSource.Create(4, 4, 96, 96, PixelFormats.Bgra32, null, pixels, 16)));
        using var file = File.Create(path);
        encoder.Save(file);
    }
}
