using System.Globalization;

namespace Launcher.Core.Library.Steam;

/// <summary>Converte jogos encontrados nas bibliotecas da Steam em <see cref="Game"/>.</summary>
public static class SteamGames
{
    /// <summary>Esquema das artes resolvidas pelo launcher (API da Steam, CDN e cache local da Steam).</summary>
    public const string ArtScheme = "steam-art";

    /// <summary>Cores de destaque enquanto não há cor extraída da arte; escolhidas pelo appid para variar entre jogos.</summary>
    private static readonly string[] Accents = ["#2B3A55", "#4A2B55", "#552B3A", "#2B5548", "#55432B", "#2B4655", "#3F2B55", "#55302B"];

    public static Game ToGame(SteamApp app)
    {
        ArgumentNullException.ThrowIfNull(app);
        string id = app.AppId.ToString(CultureInfo.InvariantCulture);
        return new Game
        {
            Id = $"steam:{id}",
            Title = app.Name,
            Store = GameStore.Steam,
            SizeLabel = SizeLabel(app.SizeOnDisk),
            Accent = Accents[app.AppId % Accents.Length],
            CoverUri = new Uri($"{ArtScheme}://{id}/cover"),
            HeroUri = new Uri($"{ArtScheme}://{id}/hero"),
            IsInstalled = true,
            AddedAt = app.InstalledAt,
            LastPlayedAt = app.LastPlayed,
            PlayTime = app.PlayTime,
            LaunchTarget = $"steam://rungameid/{id}",
            InstallPath = app.InstallPath,
            SteamAppId = app.AppId,
        };
    }

    public static string? SizeLabel(long bytes) => bytes switch
    {
        <= 0 => null,
        < 1L << 30 => $"{Math.Max(1, bytes >> 20)} MB",
        _ => $"{Math.Round(bytes / (double)(1L << 30), bytes < 10L << 30 ? 1 : 0).ToString(CultureInfo.GetCultureInfo("pt-BR"))} GB",
    };
}
