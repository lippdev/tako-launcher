using System.IO;
using System.IO.Compression;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text;

namespace Launcher.UI.Wpf.Tests;

/// <summary>Tema Fullscreen real usado na homologação (MVP-022): release fixa, verificada por SHA-256.</summary>
internal sealed record HomologatedTheme(string Name, string Slug, string License, Uri Package, string Sha256);

/// <summary>
/// Pacotes de tema para o teste de fumaça (frente C). Os temas de terceiros não são versionados: o teste baixa o
/// <c>.pthm</c> da release fixada e confere o hash antes de importar. Sem rede, a homologação é registrada como não
/// executada e o resto do teste segue.
/// </summary>
internal static class ThemeFixtures
{
    public static readonly HomologatedTheme[] Homologated =
    [
        new("PS5ish 5.0.3", "ps5ish", "MIT, davidkgriggs",
            new Uri("https://github.com/davidkgriggs/PS5ish/releases/download/v5.0.3/PS5ish_676e10ec-adfe-48d8-a1bd-4d5771b5a2ca_5_0_3.pthm"),
            "b9f07cfa2b23540755bae0897ecb4ea4afd7a9f4e6769b989fb638c72db4b632"),
        new("Toggle 1.10.1", "toggle", "MIT, Jonathan Sellier",
            new Uri("https://github.com/jonosellier/toggle-theme-playnite/releases/download/v1.10.1/Toggle_28b7d2c0-105b-4632-8dca-d11348bd61d2_1_10_1.pthm"),
            "06147d8e5ef7bdfcc254f558123fcb1b49d9e33e9fe0248f6e7445e6df320888"),
        new("Revisited 2.1.0 (parcial)", "revisited", "MIT, ld892012",
            new Uri("https://github.com/ld892012/PlayniteRevisitedSkin/releases/download/2.1.0/Revisited_2_1_0.pthm"),
            "623abde6b121f535b697abdc39f9f14ca682248ff1f7c64b1a477d1b8ef4c838"),
    ];

    private static readonly string Work = Path.Combine(Path.GetTempPath(), "tako-theme-fixtures");

    /// <summary>Baixa (com cache) e confere o hash; nulo com o motivo quando não dá.</summary>
    public static string? TryDownload(HomologatedTheme theme, out string? problem)
    {
        string file = Path.Combine(Work, Path.GetFileName(theme.Package.LocalPath));
        try
        {
            Directory.CreateDirectory(Work);
            if (!File.Exists(file) || Hash(file) != theme.Sha256)
            {
                using var http = new HttpClient { Timeout = TimeSpan.FromSeconds(90) };
                http.DefaultRequestHeaders.UserAgent.ParseAdd("Tako-CI/0.1 (+https://github.com/lippdev/ultralauncher)");
                byte[] bytes = http.GetByteArrayAsync(theme.Package).GetAwaiter().GetResult();
                File.WriteAllBytes(file, bytes);
            }

            if (Hash(file) != theme.Sha256)
            {
                problem = "SHA-256 diferente do fixado";
                return null;
            }

            problem = null;
            return file;
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException or IOException)
        {
            problem = ex.Message;
            return null;
        }
    }

    /// <summary>Cópia do Fullscreen padrão do Playnite empacotada como <c>.pthm</c>, com outro Id.</summary>
    public static string CopyOfDefault(string defaultThemeDirectory)
    {
        string file = Path.Combine(Work, "copia-do-padrao.pthm");
        Directory.CreateDirectory(Work);
        File.Delete(file);
        using var zip = ZipFile.Open(file, ZipArchiveMode.Create);
        foreach (string path in Directory.EnumerateFiles(defaultThemeDirectory, "*", SearchOption.AllDirectories))
        {
            string relative = Path.GetRelativePath(defaultThemeDirectory, path).Replace('\\', '/');
            if (relative == "theme.yaml")
            {
                Write(zip, relative, "Id: Tako_Teste_CopiaDoPadrao\nName: Cópia do padrão\nAuthor: Teste\nVersion: 1.0\nMode: Fullscreen\nThemeApiVersion: 2.0.0\n");
            }
            else
            {
                zip.CreateEntryFromFile(path, relative);
            }
        }

        return file;
    }

    /// <summary>Tema do modo Desktop: o importador recusa antes de extrair.</summary>
    public static string DesktopPackage()
    {
        string file = Path.Combine(Work, "tema-desktop.pthm");
        Directory.CreateDirectory(Work);
        File.Delete(file);
        using var zip = ZipFile.Open(file, ZipArchiveMode.Create);
        Write(zip, "theme.yaml", "Id: Tako_Teste_Desktop\nName: Tema Desktop\nAuthor: Teste\nVersion: 1.0\nMode: Desktop\nThemeApiVersion: 2.0.0\n");
        Write(zip, "Constants.xaml", "<ResourceDictionary xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" />");
        return file;
    }

    private static void Write(ZipArchive zip, string name, string content)
    {
        using var writer = new StreamWriter(zip.CreateEntry(name).Open(), new UTF8Encoding(false));
        writer.Write(content);
    }

    private static string Hash(string file)
    {
        using var stream = File.OpenRead(file);
        return Convert.ToHexStringLower(SHA256.HashData(stream));
    }
}
