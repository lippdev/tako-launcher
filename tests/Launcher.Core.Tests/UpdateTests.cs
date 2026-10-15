using System.Security.Cryptography;
using System.Text.Json;
using Launcher.Core.Updates;
using Xunit;

namespace Launcher.Core.Tests;

public class UpdateTests
{
    private const string Digest = "sha256:1861bbb28004cc7811dc37015e74acee00f5cf59242dcb8e64d995754ba2dea7";

    private static JsonElement Releases(params (string Tag, bool Pre, bool Draft, bool WithDigest)[] releases)
    {
        var list = releases.Select(r => new
        {
            tag_name = r.Tag,
            name = "Tako " + r.Tag.TrimStart('v'),
            draft = r.Draft,
            prerelease = r.Pre,
            html_url = $"https://github.com/lippdev/ultralauncher/releases/tag/{r.Tag}",
            body = "### What's new\n- **Your real library**: Steam games show up by themselves. (#13)\n- Other.",
            assets = new object[]
            {
                new { name = "Tako-Setup-x64.exe", browser_download_url = $"https://example/{r.Tag}/setup", digest = r.WithDigest ? Digest : null },
                new { name = "Tako-Portable-x64.exe", browser_download_url = $"https://example/{r.Tag}/portable", digest = r.WithDigest ? Digest : null },
            },
        });
        return JsonSerializer.SerializeToElement(list);
    }

    [Theory]
    [InlineData("0.1.0-alpha.5", "0.1.0-alpha.10", -1)]
    [InlineData("0.1.0-alpha.5", "0.1.0", -1)]
    [InlineData("0.1.0", "0.1.1-beta.1", -1)]
    [InlineData("1.2.0", "1.10.0", -1)]
    [InlineData("v0.1.0-alpha.5", "Tako 0.1.0-alpha.5", 0)]
    public void Compares_versions(string a, string b, int expected) =>
        Assert.Equal(expected, Math.Sign(SemVer.Parse(a).CompareTo(SemVer.Parse(b))));

    [Fact]
    public void Picks_the_newest_newer_release_with_the_right_file()
    {
        var feed = Releases(("v0.1.0-alpha.7", true, false, true), ("v0.1.0-alpha.6", true, false, true), ("v0.1.0-alpha.5", true, false, true));

        var portable = ReleaseFeed.Pick(feed, SemVer.Parse("0.1.0-alpha.5"), optedIntoPrereleases: false, installed: false)!;
        var installed = ReleaseFeed.Pick(feed, SemVer.Parse("0.1.0-alpha.5"), optedIntoPrereleases: false, installed: true)!;

        Assert.Equal("0.1.0-alpha.7", portable.Version);
        Assert.Equal("Tako-Portable-x64.exe", portable.AssetName);
        Assert.Equal("Tako-Setup-x64.exe", installed.AssetName);
        Assert.True(portable.CanInstall);
        Assert.Equal("Your real library: Steam games show up by themselves.", ReleaseFeed.FirstNote(portable.Notes));
    }

    [Fact]
    public void Stable_builds_skip_test_releases_unless_opted_in()
    {
        var feed = Releases(("v0.2.0-beta.1", true, false, true), ("v0.1.1", false, false, true));

        Assert.Equal("0.1.1", ReleaseFeed.Pick(feed, SemVer.Parse("0.1.0"), false, true)?.Version);
        Assert.Equal("0.2.0-beta.1", ReleaseFeed.Pick(feed, SemVer.Parse("0.1.0"), true, true)?.Version);
    }

    [Fact]
    public void Up_to_date_drafts_and_older_releases_give_nothing()
    {
        var feed = Releases(("v0.2.0", false, true, true), ("v0.1.0", false, false, true));

        Assert.Null(ReleaseFeed.Pick(feed, SemVer.Parse("0.1.0"), true, true));
    }

    [Fact]
    public void Without_github_digest_the_update_is_only_offered_as_a_page()
    {
        var update = ReleaseFeed.Pick(Releases(("v0.2.0", false, false, false)), SemVer.Parse("0.1.0"), false, true)!;

        Assert.False(update.CanInstall);
        Assert.Equal("https://github.com/lippdev/ultralauncher/releases/tag/v0.2.0", update.PageUrl);
    }

    [Fact]
    public async Task Verifies_downloaded_file_against_digest()
    {
        string file = Path.GetTempFileName();
        try
        {
            await File.WriteAllBytesAsync(file, [1, 2, 3], TestContext.Current.CancellationToken);
            string digest = "sha256:" + Convert.ToHexStringLower(SHA256.HashData([1, 2, 3]));

            Assert.True(await ReleaseFeed.VerifyFileAsync(file, digest, TestContext.Current.CancellationToken));
            Assert.False(await ReleaseFeed.VerifyFileAsync(file, Digest, TestContext.Current.CancellationToken));
            Assert.False(await ReleaseFeed.VerifyFileAsync(file, "md5:abc", TestContext.Current.CancellationToken));
        }
        finally
        {
            File.Delete(file);
        }
    }

    [Fact]
    public void Reads_the_version_section_of_a_changelog()
    {
        const string changelog = """
            # Histórico de alterações

            ## [Unreleased]

            ## [0.1.0-alpha.6]
            ### Novidades
            - **Atualização automática**: o Tako procura e baixa versões novas sozinho. (#15)

            ## [0.1.0-alpha.5]
            ### Novidades
            - Outra coisa.
            """;

        string section = ReleaseFeed.ChangelogSection(changelog, "0.1.0-alpha.6")!;

        Assert.Equal("Atualização automática: o Tako procura e baixa versões novas sozinho.", ReleaseFeed.FirstNote(section));
        Assert.DoesNotContain("Outra coisa", section, StringComparison.Ordinal);
        Assert.Null(ReleaseFeed.ChangelogSection(changelog, "9.9.9"));
    }
}
