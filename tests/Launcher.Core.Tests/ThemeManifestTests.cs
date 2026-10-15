using Launcher.Core.Themes;
using Xunit;

namespace Launcher.Core.Tests;

public class ThemeManifestTests
{
    private const string PlayniteDefault = "﻿Id: Playnite_builtin_DefaultFullscreen\r\nName: Default\r\nAuthor: Playnite\r\nVersion: 1.0\r\nMode: Fullscreen\r\nThemeApiVersion: 2.0.0";

    private static ThemeManifestResult Check(string yaml) => ThemeManifestValidator.Validate(ThemeManifest.Parse(yaml));

    private static string Manifest(string? id = "Autor_Tema", string? mode = "Fullscreen", string? api = "2.0.0", string? name = "Tema") =>
        string.Join('\n', new[]
        {
            id is null ? null : $"Id: {id}",
            name is null ? null : $"Name: {name}",
            "Author: Alguém",
            "Version: 1.2",
            mode is null ? null : $"Mode: {mode}",
            api is null ? null : $"ThemeApiVersion: {api}",
        }.OfType<string>());

    [Fact]
    public void Reads_the_playnite_default_manifest()
    {
        var result = Check(PlayniteDefault);

        Assert.True(result.IsCompatible);
        Assert.Empty(result.Diagnostics);
        var m = result.Manifest;
        Assert.Equal("Playnite_builtin_DefaultFullscreen", m.Id);
        Assert.Equal("Default", m.Name);
        Assert.Equal("Playnite", m.Author);
        Assert.Equal("1.0", m.Version);
        Assert.Equal(ThemeMode.Fullscreen, m.Mode);
        Assert.Equal(new Version(2, 0, 0), m.ThemeApiVersion);
    }

    [Fact]
    public void Handles_quotes_comments_nested_values_and_case()
    {
        var m = ThemeManifest.Parse("# cabeçalho\nid: 'Meu.Tema-1'\nName: \"Nome: com dois pontos\" \nmode: fullscreen # comentário\nThemeApiVersion: 2\nLinks:\n  - Name: Site\n    Url: https://exemplo\n");

        Assert.Equal("Meu.Tema-1", m.Id);
        Assert.Equal("Nome: com dois pontos", m.Name);
        Assert.Equal(ThemeMode.Fullscreen, m.Mode);
        Assert.Equal(new Version(2, 0, 0), m.ThemeApiVersion);
        Assert.False(m.Values.ContainsKey("Url"));
    }

    [Fact]
    public void Refuses_desktop_themes()
    {
        var result = Check(Manifest(mode: "Desktop"));

        Assert.False(result.IsCompatible);
        Assert.Contains(result.Errors, d => d.Code == "manifest.desktop");
    }

    [Theory]
    [InlineData("1.9.0", "manifest.unsupported-api")]
    [InlineData("3.0.0", "manifest.unsupported-api")]
    [InlineData("dois", "manifest.invalid-api")]
    public void Refuses_api_versions_outside_2x(string api, string code)
    {
        var result = Check(Manifest(api: api));

        Assert.False(result.IsCompatible);
        Assert.Contains(result.Errors, d => d.Code == code);
    }

    [Theory]
    [InlineData("2.0.0", false)]
    [InlineData("2.0", false)]
    [InlineData("2.9.1", true)]
    public void Accepts_2x_and_warns_about_newer_minor_versions(string api, bool warns)
    {
        var result = Check(Manifest(api: api));

        Assert.True(result.IsCompatible);
        Assert.Equal(warns, result.Diagnostics.Any(d => d.Code == "manifest.newer-api"));
    }

    [Fact]
    public void Reports_every_missing_field()
    {
        var result = Check("Author: X");

        Assert.False(result.IsCompatible);
        Assert.Equal(
            ["manifest.missing-api", "manifest.missing-id", "manifest.missing-mode", "manifest.missing-name", "manifest.missing-version"],
            result.Diagnostics.Select(d => d.Code).Order(StringComparer.Ordinal));
        Assert.DoesNotContain(result.Diagnostics, d => d.Code == "manifest.missing-version" && d.IsError);
    }

    [Theory]
    [InlineData("../fora")]
    [InlineData("a/b")]
    [InlineData("C:")]
    [InlineData("CON")]
    [InlineData("nul.tema")]
    [InlineData(".oculto")]
    [InlineData("termina.")]
    [InlineData("com espaço")]
    public void Refuses_ids_that_are_unsafe_folder_names(string id)
    {
        var result = Check(Manifest(id: id));

        Assert.Contains(result.Errors, d => d.Code == "manifest.invalid-id");
    }

    [Fact]
    public void Reading_a_folder_without_manifest_is_a_diagnostic_not_an_exception()
    {
        string dir = Directory.CreateTempSubdirectory("tako-theme-").FullName;
        try
        {
            var result = ThemeManifest.Read(dir);
            Assert.Contains(result.Errors, d => d.Code == "manifest.missing");
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }
}
