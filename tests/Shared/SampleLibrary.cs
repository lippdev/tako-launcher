using System.Globalization;
using System.Text.Json;
using System.Text.Json.Serialization;

using Launcher.Core.Library;

namespace Launcher.Tests;

/// <summary>
/// Biblioteca fictícia usada só nos testes (o app usa a biblioteca real). Os dados ficam em <c>SampleLibrary.json</c>
/// (recurso embutido) e trazem só URLs das artes; nenhuma imagem de jogo é versionada.
/// </summary>
public static class SampleLibrary
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNameCaseInsensitive = true,
        Converters = { new JsonStringEnumConverter() },
    };

    public static IReadOnlyList<Game> Load(TimeProvider time)
    {
        ArgumentNullException.ThrowIfNull(time);
        using var stream = typeof(SampleLibrary).Assembly.GetManifestResourceStream("Launcher.Tests.SampleLibrary.json")
            ?? throw new InvalidOperationException("SampleLibrary.json não foi embutido no assembly.");
        var document = JsonSerializer.Deserialize<SampleDocument>(stream, Options)
            ?? throw new InvalidOperationException("SampleLibrary.json vazio.");
        var now = time.GetUtcNow();
        return document.Games.Select(e => e.ToGame(now)).ToList();
    }

    private sealed record SampleDocument(int Version, IReadOnlyList<SampleEntry> Games);

    private sealed record SampleEntry(
        string Id,
        string Title,
        GameStore Store,
        string? SizeLabel,
        string Accent,
        Uri? CoverUrl,
        Uri? HeroUrl,
        string? HeroAlignment,
        IReadOnlyList<string>? Genres,
        string? Description,
        string? Note,
        string? ReleaseDate,
        string? UpcomingRelease,
        int? LastPlayedDaysAgo,
        double PlayTimeHours,
        bool IsInstalled,
        bool IsFavorite,
        int AddedDaysAgo)
    {
        public Game ToGame(DateTimeOffset now) => new()
        {
            Id = Id,
            Title = Title,
            Store = Store,
            SizeLabel = SizeLabel,
            Accent = Accent,
            CoverUri = CoverUrl,
            HeroUri = HeroUrl,
            HeroAlignment = HeroAlignment,
            Genres = Genres ?? [],
            Description = Description ?? string.Empty,
            Note = Note,
            ReleaseDate = ParseDate(ReleaseDate),
            UpcomingRelease = ParseDate(UpcomingRelease),
            IsInstalled = IsInstalled,
            IsFavorite = IsFavorite,
            AddedAt = now.AddDays(-AddedDaysAgo),
            LastPlayedAt = LastPlayedDaysAgo is int days ? now.AddDays(-days).AddHours(-1) : null,
            PlayTime = TimeSpan.FromHours(PlayTimeHours),
        };

        private static DateOnly? ParseDate(string? value) =>
            value is null ? null : DateOnly.ParseExact(value, "yyyy-MM-dd", CultureInfo.InvariantCulture);
    }
}
