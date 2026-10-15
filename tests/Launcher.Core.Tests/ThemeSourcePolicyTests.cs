using Launcher.Core.Themes;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// Origens que um tema pode carregar em tempo de execução (frente H, questão aberta 8 do MVP-016). A guarda de mídia do
/// WPF delega para esta política; aqui ela é testada sem WPF.
/// </summary>
public sealed class ThemeSourcePolicyTests : IDisposable
{
    private readonly string _work = Directory.CreateTempSubdirectory("tako-theme-source-").FullName;
    private readonly ThemeSourcePolicy _policy = new();

    public ThemeSourcePolicyTests()
    {
        Directory.CreateDirectory(Path.Combine(Theme, "Images"));
        Directory.CreateDirectory(Path.Combine(Theme, "Views"));
        Directory.CreateDirectory(Path.Combine(BaseTheme, "Images"));
        Directory.CreateDirectory(Path.Combine(OtherTheme, "Images"));
        Directory.CreateDirectory(ArtCache);
        File.WriteAllText(Path.Combine(_work, "fora.png"), "x");
        _policy.AllowTheme(Theme);
        _policy.AllowTheme(OtherTheme);
        _policy.AllowTheme(BaseTheme, isBase: true);
        _policy.AllowDirectory(ArtCache);
    }

    private string Theme => Path.Combine(_work, "themes", "Tema");

    private string OtherTheme => Path.Combine(_work, "themes", "Outro");

    private string BaseTheme => Path.Combine(_work, "builtin", "Playnite-Default");

    private string ArtCache => Path.Combine(_work, "cache", "art");

    /// <summary>Arquivo XAML do tema: base dos caminhos relativos, como o WPF resolve.</summary>
    private Uri MainXaml => new(Path.Combine(Theme, "Views", "Main.xaml"));

    public void Dispose() => Directory.Delete(_work, recursive: true);

    [Theory]
    [InlineData(@"\\exemplo\share\x.png")]
    [InlineData("//exemplo/share/x.png")]
    [InlineData(@"\\?\UNC\exemplo\share\x.png")]
    [InlineData(@"\\.\C:\Windows\win.ini")]
    [InlineData("file://exemplo/share/x.png")]
    public void Blocks_network_paths_before_touching_the_disk(string origin)
    {
        Assert.Equal(ThemeSourceBlock.Network, _policy.Evaluate(origin, MainXaml).Block);
        Assert.Equal(ThemeSourceBlock.Network, _policy.Evaluate(origin, baseUri: null).Block);
    }

    [Fact]
    public void Blocks_network_paths_composed_from_relative_pieces() =>
        Assert.Equal(ThemeSourceBlock.Network, _policy.Evaluate("//exemplo/share/x.png", MainXaml).Block);

    [Theory]
    [InlineData("http://exemplo/x.png")]
    [InlineData("https://exemplo/x.png")]
    [InlineData("ftp://exemplo/x.png")]
    [InlineData("net.pipe://exemplo/x")]
    public void Blocks_remote_schemes(string origin) =>
        Assert.Equal(ThemeSourceBlock.External, _policy.Evaluate(origin, MainXaml).Block);

    [Theory]
    [InlineData("../../../fora.png")]
    [InlineData(@"..\..\..\fora.png")]
    [InlineData(@"..\..\..\..\..\..\..\..\..\Windows\win.ini")]
    public void Blocks_relative_paths_that_leave_the_theme(string origin) =>
        Assert.Equal(ThemeSourceBlock.OutsideTheme, _policy.Evaluate(origin, MainXaml).Block);

    [Fact]
    public void Blocks_absolute_files_outside_allowed_folders()
    {
        string outside = Path.Combine(_work, "fora.png");

        Assert.Equal(ThemeSourceBlock.OutsideTheme, _policy.Evaluate(outside, MainXaml).Block);
        Assert.Equal(ThemeSourceBlock.OutsideTheme, _policy.Evaluate(new Uri(outside), baseUri: null).Block);
    }

    [Fact]
    public void Allows_theme_files_resolved_against_the_theme_xaml()
    {
        var decision = _policy.Evaluate("../Images/x.png", MainXaml);

        Assert.True(decision.IsAllowed, decision.Block.ToString());
        Assert.Equal(Path.Combine(Theme, "Images", "x.png"), decision.Uri!.LocalPath); // permitido: URI presente
    }

    [Fact]
    public void Allows_the_base_theme_and_the_art_cache_but_not_another_theme()
    {
        Assert.True(_policy.Evaluate(Path.Combine(BaseTheme, "Images", "x.png"), MainXaml).IsAllowed);
        Assert.True(_policy.Evaluate(Path.Combine(ArtCache, "capa.jpg"), MainXaml).IsAllowed);
        Assert.Equal(ThemeSourceBlock.OutsideTheme, _policy.Evaluate(Path.Combine(OtherTheme, "Images", "x.png"), MainXaml).Block);
    }

    [Fact]
    public void Without_a_known_xaml_any_registered_theme_is_allowed()
    {
        Assert.True(_policy.Evaluate(Path.Combine(OtherTheme, "Images", "x.png"), baseUri: null).IsAllowed);
        Assert.Equal(ThemeSourceBlock.OutsideTheme, _policy.Evaluate(Path.Combine(_work, "fora.png"), baseUri: null).Block);
    }

    [Fact]
    public void Relative_paths_without_a_file_base_stay_in_the_application_package()
    {
        var decision = _policy.Evaluate("../../../Windows/win.ini", baseUri: null);

        Assert.True(decision.IsAllowed);
        Assert.False(decision.Uri!.IsAbsoluteUri); // permitido: URI presente
    }

    [Fact]
    public void Blocks_links_that_point_out_of_the_theme()
    {
        string link = Path.Combine(Theme, "Images", "atalho");
        try
        {
            Directory.CreateSymbolicLink(link, _work);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or PlatformNotSupportedException)
        {
            Assert.Skip($"Sem permissão para criar link simbólico: {ex.Message}");
        }

        Assert.Equal(ThemeSourceBlock.Link, _policy.Evaluate(Path.Combine(link, "fora.png"), MainXaml).Block);
        Assert.True(ThemeSourcePolicy.CrossesLink(Theme, Path.Combine(link, "fora.png")));
        Assert.False(ThemeSourcePolicy.CrossesLink(Theme, Path.Combine(Theme, "Images", "x.png")));
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    public void Blocks_empty_text(string origin) =>
        Assert.Equal(ThemeSourceBlock.Invalid, _policy.Evaluate(origin, MainXaml).Block);

    [Fact]
    public void Network_folders_cannot_be_registered() =>
        Assert.Throws<ArgumentException>(() => _policy.AllowDirectory(@"\\exemplo\share"));
}
