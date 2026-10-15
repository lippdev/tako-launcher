using Launcher.Core.Library.Sources;

namespace Launcher.Core.Library.Steam;

/// <summary>Fonte de biblioteca da Steam (STORE-001): os arquivos da Steam em <paramref name="steamRoot"/> (nulo = não instalada).</summary>
public sealed class SteamLibrarySource(string? steamRoot) : ILibrarySource
{
    public GameStore Store => GameStore.Steam;

    public LibraryScan Scan() => steamRoot is null
        ? LibraryScan.Missing("Steam não encontrada neste PC")
        : new LibraryScan([.. SteamLibraryScanner.Scan(steamRoot).Select(SteamGames.ToGame)], ClientInstalled: true);
}
