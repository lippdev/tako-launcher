using Launcher.Core.Library;

namespace Launcher.Core.Session;

/// <summary>Como o launcher reconhece que o jogo está rodando e quando terminou (MVP-026).</summary>
public enum GameTrackingStrategy
{
    /// <summary>Sem acompanhamento: o jogo só é aberto (ex.: URI sem pasta conhecida).</summary>
    None,

    /// <summary>O processo iniciado pelo launcher e os filhos dele (launcher do jogo que abre outro exe e fecha).</summary>
    Process,

    /// <summary>Qualquer processo cujo executável esteja dentro da pasta de instalação (jogos da Steam).</summary>
    InstallFolder,

    /// <summary>Só o que a Steam informa (<c>RunningAppID</c>): jogos da Steam sem pasta conhecida.</summary>
    SteamApp,
}

/// <summary>
/// Estratégia escolhida para um jogo e a pasta observada (na estratégia por pasta). <see cref="SteamAppId"/> liga o
/// sinal da Steam (<c>RunningAppID</c>) junto com qualquer estratégia: confirma o início e marca o fim com precisão.
/// </summary>
public sealed record TrackingPlan(GameTrackingStrategy Strategy, string? Folder = null, int? SteamAppId = null)
{
    public static TrackingPlan None { get; } = new(GameTrackingStrategy.None);
}

public static class GameTracking
{
    /// <summary>
    /// Decide a estratégia: a escolhida pelo usuário quando possível; senão pasta para jogos de loja com pasta
    /// conhecida e processo para executáveis e atalhos adicionados manualmente. <paramref name="hasProcess"/> diz se
    /// abrir o jogo devolveu um processo (URIs e alguns atalhos não devolvem).
    /// </summary>
    public static TrackingPlan Plan(Game game, bool hasProcess)
    {
        ArgumentNullException.ThrowIfNull(game);
        string? folder = string.IsNullOrWhiteSpace(game.InstallPath) ? null : game.InstallPath;
        int? steamAppId = game.Store == GameStore.Steam && game.SteamAppId is > 0 ? game.SteamAppId : null;
        var byFolder = folder is null ? TrackingPlan.None : new TrackingPlan(GameTrackingStrategy.InstallFolder, folder);
        var byProcess = hasProcess ? new TrackingPlan(GameTrackingStrategy.Process) : TrackingPlan.None;
        var bySteam = steamAppId is null ? TrackingPlan.None : new TrackingPlan(GameTrackingStrategy.SteamApp);

        var plan = game.Tracking switch
        {
            GameTrackingStrategy.None => TrackingPlan.None,
            GameTrackingStrategy.InstallFolder when byFolder.Strategy != GameTrackingStrategy.None => byFolder,
            GameTrackingStrategy.Process when hasProcess => byProcess,
            GameTrackingStrategy.SteamApp when steamAppId is not null => bySteam,
            _ when game.Store != GameStore.Manual => FirstOf(byFolder, byProcess, bySteam),
            // Manual: o processo iniciado; sem ele, a pasta só serve para executáveis (a de um atalho é a do atalho).
            _ when hasProcess => byProcess,
            _ when IsExecutable(game.LaunchTarget) => byFolder,
            _ => TrackingPlan.None,
        };

        // Jogo da Steam: o RunningAppID entra como sinal extra em qualquer estratégia.
        return plan.Strategy == GameTrackingStrategy.None || steamAppId is null ? plan : plan with { SteamAppId = steamAppId };
    }

    private static TrackingPlan FirstOf(params TrackingPlan[] plans) =>
        plans.FirstOrDefault(p => p.Strategy != GameTrackingStrategy.None) ?? TrackingPlan.None;

    /// <summary>O caminho está dentro da pasta (sem diferenciar maiúsculas, aceita \ e /).</summary>
    public static bool IsInside(string? path, string folder)
    {
        ArgumentNullException.ThrowIfNull(folder);
        string root = folder.TrimEnd('\\', '/');
        return path is not null
            && root.Length > 0
            && path.Length > root.Length + 1
            && path.StartsWith(root, StringComparison.OrdinalIgnoreCase)
            && path[root.Length] is '\\' or '/';
    }

    private static bool IsExecutable(string? target) =>
        target is not null && (target.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) || target.EndsWith(".bat", StringComparison.OrdinalIgnoreCase) || target.EndsWith(".cmd", StringComparison.OrdinalIgnoreCase));
}
