using System.IO.Compression;
using System.Text;
using Launcher.Core.Themes;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class ThemePackageTests : IDisposable
{
    private const string Yaml = "Id: Autor_TemaTeste\nName: Tema de teste\nAuthor: Autor\nVersion: 1.0\nMode: Fullscreen\nThemeApiVersion: 2.0.0\n";
    private const string Dictionary = "<ResourceDictionary xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" />";

    private readonly string _work = Directory.CreateTempSubdirectory("tako-pthm-").FullName;

    private string ThemesRoot => Path.Combine(_work, "themes");

    public void Dispose() => Directory.Delete(_work, recursive: true);

    private static MemoryStream Zip(params (string Name, string Content)[] entries)
    {
        var stream = new MemoryStream();
        using (var zip = new ZipArchive(stream, ZipArchiveMode.Create, leaveOpen: true))
        {
            foreach (var (name, content) in entries)
            {
                using var writer = new StreamWriter(zip.CreateEntry(name).Open(), Encoding.UTF8);
                writer.Write(content);
            }
        }

        stream.Position = 0;
        return stream;
    }

    private ThemeInstallResult Install(Stream package, ThemePackageLimits? limits = null, string[]? reserved = null) =>
        ThemePackage.Install(package, ThemesRoot, limits, reserved);

    /// <summary>Nada fora da pasta de temas e nenhuma pasta temporária sobrando.</summary>
    private void AssertNothingLeaked(params string[] themes)
    {
        Assert.All(Directory.EnumerateFileSystemEntries(_work), e => Assert.Equal("themes", Path.GetFileName(e)));
        Assert.Equal(themes.Order(StringComparer.Ordinal), Directory.EnumerateFileSystemEntries(ThemesRoot).Select(Path.GetFileName).Order(StringComparer.Ordinal));
    }

    [Fact]
    public void Installs_a_valid_package_into_a_folder_named_after_the_id()
    {
        using var package = Zip(("theme.yaml", Yaml), ("Constants.xaml", Dictionary), ("Views/Main.xaml", Dictionary), ("Images/bg.png", "png"));

        var result = Install(package);

        Assert.True(result.Succeeded, string.Join('\n', result.Diagnostics));
        Assert.Equal(Path.Combine(Path.GetFullPath(ThemesRoot), "Autor_TemaTeste"), result.Directory);
        Assert.Equal(4, result.FileCount);
        Assert.True(File.Exists(Path.Combine(result.Directory!, "Views", "Main.xaml")));
        Assert.Equal("Tema de teste", result.Manifest!.Name);
        AssertNothingLeaked("Autor_TemaTeste");
        Assert.Single(InstalledTheme.Scan(ThemesRoot));
    }

    [Fact]
    public void Accepts_a_package_wrapped_in_a_single_folder()
    {
        using var package = Zip(("MeuTema/theme.yaml", Yaml), ("MeuTema/Constants.xaml", Dictionary));

        var result = Install(package);

        Assert.True(result.Succeeded);
        Assert.True(File.Exists(Path.Combine(result.Directory!, "Constants.xaml")));
    }

    [Fact]
    public void Reinstalling_replaces_the_previous_version()
    {
        using (var first = Zip(("theme.yaml", Yaml), ("Old.xaml", Dictionary)))
        {
            Assert.True(Install(first).Succeeded);
        }

        using var second = Zip(("theme.yaml", Yaml.Replace("1.0", "2.0", StringComparison.Ordinal)), ("New.xaml", Dictionary));
        var result = Install(second);

        Assert.True(result.Succeeded);
        Assert.False(File.Exists(Path.Combine(result.Directory!, "Old.xaml")));
        Assert.True(File.Exists(Path.Combine(result.Directory!, "New.xaml")));
        AssertNothingLeaked("Autor_TemaTeste");
    }

    [Theory]
    [InlineData("../evil.xaml")]
    [InlineData("Views/../../evil.xaml")]
    [InlineData("..\\evil.xaml")]
    [InlineData("/etc/evil.xaml")]
    [InlineData("\\\\servidor\\share\\evil.xaml")]
    [InlineData("C:/Windows/evil.xaml")]
    [InlineData("C:evil.xaml")]
    [InlineData("theme.yaml:stream")]
    [InlineData("Views/CON.xaml")]
    [InlineData("Views/nome. ")]
    public void Refuses_zip_slip_and_dangerous_names_before_writing_anything(string name)
    {
        using var package = Zip(("theme.yaml", Yaml), (name, "x"));

        var result = Install(package);

        Assert.False(result.Succeeded);
        Assert.Contains(result.Errors, d => d.Code == "package.unsafe-path");
        Assert.False(Directory.Exists(ThemesRoot));
        Assert.False(File.Exists(Path.Combine(_work, "evil.xaml")));
    }

    [Fact]
    public void Refuses_desktop_themes_without_extracting()
    {
        using var package = Zip(("theme.yaml", Yaml.Replace("Fullscreen", "Desktop", StringComparison.Ordinal)), ("Constants.xaml", Dictionary));

        var result = Install(package);

        Assert.False(result.Succeeded);
        Assert.Contains(result.Errors, d => d.Code == "manifest.desktop");
        Assert.False(Directory.Exists(ThemesRoot));
    }

    [Fact]
    public void Refuses_unsupported_api_versions()
    {
        using var package = Zip(("theme.yaml", Yaml.Replace("2.0.0", "1.6.0", StringComparison.Ordinal)));

        Assert.Contains(Install(package).Errors, d => d.Code == "manifest.unsupported-api");
    }

    [Fact]
    public void Refuses_packages_without_manifest()
    {
        using var package = Zip(("Constants.xaml", Dictionary), ("Outro/theme.yaml", Yaml), ("Mais/x.xaml", Dictionary));

        Assert.Contains(Install(package).Errors, d => d.Code == "package.missing-manifest");
    }

    [Fact]
    public void Refuses_files_that_are_not_zip()
    {
        using var package = new MemoryStream(Encoding.UTF8.GetBytes("isto não é um zip"));

        Assert.Contains(Install(package).Errors, d => d.Code == "package.not-zip");
    }

    [Fact]
    public void Refuses_reserved_ids()
    {
        using var package = Zip(("theme.yaml", Yaml));

        Assert.Contains(Install(package, reserved: ["autor_temateste"]).Errors, d => d.Code == "package.reserved-id");
    }

    [Fact]
    public void Refuses_too_many_entries()
    {
        var entries = Enumerable.Range(0, 20).Select(i => ($"Views/{i}.xaml", Dictionary)).Prepend(("theme.yaml", Yaml)).ToArray();
        using var package = Zip(entries);

        var result = Install(package, new ThemePackageLimits { MaxEntries = 10 });

        Assert.Contains(result.Errors, d => d.Code == "package.too-many-entries");
    }

    [Fact]
    public void Refuses_packages_larger_than_the_total_limit()
    {
        string big = new('a', 4096);
        using var package = Zip(("theme.yaml", Yaml), ("a.xaml", big), ("b.xaml", big));

        var result = Install(package, new ThemePackageLimits { MaxTotalBytes = 6000 });

        Assert.Contains(result.Errors, d => d.Code == "package.too-large");
        Assert.False(Directory.Exists(Path.Combine(ThemesRoot, "Autor_TemaTeste")));
    }

    [Fact]
    public void Refuses_highly_compressed_files_over_the_file_limit()
    {
        // "Zip bomb" pequeno: 1 MB de zeros comprime para poucos KB.
        using var package = Zip(("theme.yaml", Yaml), ("Images/bomb.png", new string('\0', 1024 * 1024)));
        Assert.True(package.Length < 64 * 1024);

        var result = Install(package, new ThemePackageLimits { MaxFileBytes = 512 * 1024 });

        Assert.Contains(result.Errors, d => d.Code == "package.file-too-large");
    }

    [Fact]
    public void Refuses_duplicate_entries()
    {
        using var package = Zip(("theme.yaml", Yaml), ("Views/Main.xaml", Dictionary), ("views/main.xaml", Dictionary));

        Assert.Contains(Install(package).Errors, d => d.Code == "package.duplicate-entry");
    }

    [Fact]
    public void Skips_executables_with_a_warning()
    {
        using var package = Zip(("theme.yaml", Yaml), ("Constants.xaml", Dictionary), ("tools/run.exe", "MZ"), ("setup.ps1", "rm -r"));

        var result = Install(package);

        Assert.True(result.Succeeded);
        Assert.Equal(2, result.Warnings.Count(d => d.Code == "package.skipped-executable"));
        Assert.False(File.Exists(Path.Combine(result.Directory!, "tools", "run.exe")));
        Assert.False(File.Exists(Path.Combine(result.Directory!, "setup.ps1")));
    }

    [Fact]
    public void Installs_from_a_pthm_file_on_disk()
    {
        string file = Path.Combine(_work, "tema.pthm");
        using (var package = Zip(("theme.yaml", Yaml)))
        using (var output = File.Create(file))
        {
            package.CopyTo(output);
        }

        Assert.True(ThemePackage.Install(file, ThemesRoot).Succeeded);
        Assert.Contains(ThemePackage.Install(Path.Combine(_work, "nao-existe.pthm"), ThemesRoot).Errors, d => d.Code == "package.not-found");
    }

    [Fact]
    public void Remove_only_deletes_folders_directly_inside_the_themes_root()
    {
        using var package = Zip(("theme.yaml", Yaml));
        var installed = Install(package);

        Assert.Throws<InvalidOperationException>(() => ThemePackage.Remove(ThemesRoot, _work));
        Assert.Throws<InvalidOperationException>(() => ThemePackage.Remove(ThemesRoot, ThemesRoot));
        Assert.Throws<InvalidOperationException>(() => ThemePackage.Remove(ThemesRoot, Path.Combine(ThemesRoot, "..", "fora")));

        ThemePackage.Remove(ThemesRoot, installed.Directory!);
        Assert.False(Directory.Exists(installed.Directory));
        Assert.Empty(InstalledTheme.Scan(ThemesRoot));
    }

    [Theory]
    [InlineData("Views/Main.xaml", "Views/Main.xaml")]
    [InlineData("Views\\Main.xaml", "Views/Main.xaml")]
    [InlineData("./Views//Main.xaml", "Views/Main.xaml")]
    [InlineData("Views/", null)]
    public void Normalizes_safe_entry_names(string name, string? expected)
    {
        Assert.True(ZipEntryPath.TryNormalize(name, out string? relative, out _));
        Assert.Equal(expected, relative);
    }
}
