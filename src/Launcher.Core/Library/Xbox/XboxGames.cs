namespace Launcher.Core.Library.Xbox;

/// <summary>Converte jogos do app Xbox / Microsoft Store em <see cref="Game"/>.</summary>
public static class XboxGames
{
    public const string IdPrefix = "xbox:";

    public static Game ToGame(XboxInstalledGame game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return new Game
        {
            Id = IdPrefix + game.FamilyName,
            Title = game.Title,
            Store = GameStore.Xbox,
            Accent = game.Accent,
            // Logos do pacote são só o plano B: a arte da Steam (metadados casados pelo nome) tem prioridade.
            CoverUri = game.CoverPath is null ? null : new Uri(game.CoverPath),
            HeroUri = (game.HeroPath ?? game.CoverPath) is { } hero ? new Uri(hero) : null,
            HasFallbackArt = game.CoverPath is not null,
            IsInstalled = true,
            AddedAt = game.InstalledAt,
            // Sempre pelo AUMID: pacotes protegidos não abrem pelo executável.
            LaunchTarget = game.LaunchTarget,
            // O AUMID não devolve processo: o acompanhamento automático usa os processos dentro da pasta do pacote.
            InstallPath = game.Folder,
        };
    }
}
