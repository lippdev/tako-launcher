using System.Globalization;
using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Launcher.Core.Library;

namespace Launcher.Bench;

/// <summary>
/// Biblioteca fictícia de tamanho configurável, sempre igual para o mesmo tamanho (semente fixa). As artes são JPEG
/// gerados localmente (sem rede): cada jogo tem a própria capa (decodificação real por jogo) e os fundos se repetem
/// a partir de um conjunto pequeno, como acontece com o cache de artes grandes do Tako.
/// </summary>
internal static class SyntheticLibrary
{
    private const int CoverPool = 40;
    private const int HeroPool = 12;
    private const int Seed = 35;

    private static readonly string[] Words =
    [
        "Horizonte", "Abismo", "Ecos", "Lâmina", "Fronteira", "Neon", "Reino", "Tempestade", "Órbita", "Ruínas",
        "Crônicas", "Vale", "Sombras", "Aurora", "Circuito", "Deriva", "Colosso", "Maré", "Vértice", "Relíquia",
    ];

    private static readonly string[] Genres = ["Ação", "Aventura", "RPG", "Corrida", "Estratégia", "Plataforma", "Tiro", "Esportes", "Indie", "Simulação"];

    private static readonly GameStore[] Stores = [GameStore.Steam, GameStore.Steam, GameStore.Epic, GameStore.Gog, GameStore.Xbox];

    /// <summary>Gera (uma vez) as artes de <paramref name="count"/> jogos em <paramref name="artDirectory"/>.</summary>
    public static void EnsureArt(string artDirectory, int count)
    {
        Directory.CreateDirectory(artDirectory);
        for (int i = 0; i < Math.Min(CoverPool, count); i++)
        {
            string pooled = Path.Combine(artDirectory, $"pool-cover-{i:00}.jpg");
            if (!File.Exists(pooled))
            {
                Render(pooled, 600, 900, i, $"#{i + 1}");
            }
        }

        for (int i = 0; i < HeroPool; i++)
        {
            string hero = HeroPath(artDirectory, i);
            if (!File.Exists(hero))
            {
                Render(hero, 1920, 1080, i + 100, string.Empty);
            }
        }

        // Um arquivo por jogo: o Tako guarda imagens por endereço, então capas repetidas não seriam decodificadas de novo.
        for (int i = 0; i < count; i++)
        {
            string cover = CoverPath(artDirectory, i);
            if (!File.Exists(cover))
            {
                File.Copy(Path.Combine(artDirectory, $"pool-cover-{i % CoverPool:00}.jpg"), cover);
            }
        }
    }

    public static IReadOnlyList<Game> Create(int count, string artDirectory, DateTimeOffset now)
    {
#pragma warning disable CA5394 // Dados fictícios com semente fixa; não é uso criptográfico.
        var random = new Random(Seed);
        var games = new List<Game>(count);
        for (int i = 0; i < count; i++)
        {
            string title = $"{Words[random.Next(Words.Length)]} {Words[random.Next(Words.Length)]} {i + 1}";
            games.Add(new Game
            {
                Id = $"bench-{i:0000}",
                Title = title,
                Store = Stores[i % Stores.Length],
                Accent = string.Create(CultureInfo.InvariantCulture, $"#{random.Next(0x20, 0x90):X2}{random.Next(0x20, 0x90):X2}{random.Next(0x20, 0x90):X2}"),
                CoverUri = new Uri(CoverPath(artDirectory, i)),
                HeroUri = new Uri(HeroPath(artDirectory, i % HeroPool)),
                Genres = [Genres[random.Next(Genres.Length)], Genres[random.Next(Genres.Length)]],
                Description = $"Jogo fictício número {i + 1} do benchmark do Tako. Texto de tamanho realista para a página de detalhes, com duas ou três linhas como as descrições da Steam Store.",
                Developer = "Estúdio de Teste",
                ReleaseDate = new DateOnly(2015 + (i % 11), 1 + (i % 12), 1 + (i % 28)),
                SizeLabel = $"{5 + random.Next(150)} GB",
                IsInstalled = i % 5 != 4,
                IsFavorite = i % 17 == 0,
                AddedAt = now.AddDays(-random.Next(1, 900)),
                LastPlayedAt = i < 15 ? now.AddDays(-i).AddHours(-1) : null,
                PlayTime = TimeSpan.FromHours(random.Next(0, 300)),
            });
        }
#pragma warning restore CA5394

        return games;
    }

    private static string CoverPath(string directory, int index) => Path.Combine(directory, $"cover-{index:0000}.jpg");

    private static string HeroPath(string directory, int index) => Path.Combine(directory, $"hero-{index:00}.jpg");

    /// <summary>Gradiente com formas e texto: comprime como uma arte real (não como uma cor sólida).</summary>
    private static void Render(string path, int width, int height, int variant, string label)
    {
        var hue = (variant * 47) % 360;
        var visual = new DrawingVisual();
        using (var context = visual.RenderOpen())
        {
            var gradient = new LinearGradientBrush(FromHue(hue, 0.55, 0.45), FromHue((hue + 60) % 360, 0.6, 0.15), 45);
            context.DrawRectangle(gradient, null, new Rect(0, 0, width, height));
            for (int i = 0; i < 14; i++)
            {
                var brush = new SolidColorBrush(Color.FromArgb(40, 255, 255, 255));
                double size = (width / 10.0) + (i * width / 40.0);
                context.DrawEllipse(brush, null, new Point((i * 137 + variant * 31) % width, (i * 89 + variant * 53) % height), size, size * 0.6);
            }

            if (label.Length > 0)
            {
                var text = new FormattedText(label, CultureInfo.InvariantCulture, FlowDirection.LeftToRight, new Typeface("Segoe UI"), width / 6.0, Brushes.White, 1);
                context.DrawText(text, new Point(width / 12.0, height - (width / 3.0)));
            }
        }

        var bitmap = new RenderTargetBitmap(width, height, 96, 96, PixelFormats.Pbgra32);
        bitmap.Render(visual);
        var encoder = new JpegBitmapEncoder { QualityLevel = 85 };
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using var file = File.Create(path);
        encoder.Save(file);
    }

    private static Color FromHue(double hue, double saturation, double lightness)
    {
        double c = (1 - Math.Abs((2 * lightness) - 1)) * saturation;
        double x = c * (1 - Math.Abs((hue / 60 % 2) - 1));
        double m = lightness - (c / 2);
        (double r, double g, double b) = hue switch
        {
            < 60 => (c, x, 0d),
            < 120 => (x, c, 0d),
            < 180 => (0d, c, x),
            < 240 => (0d, x, c),
            < 300 => (x, 0d, c),
            _ => (c, 0d, x),
        };
        return Color.FromRgb((byte)((r + m) * 255), (byte)((g + m) * 255), (byte)((b + m) * 255));
    }
}
