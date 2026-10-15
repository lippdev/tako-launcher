using Launcher.Core.Library.Sources;

namespace Launcher.Core.Library.Xbox;

/// <summary>
/// Fonte de biblioteca do app Xbox / Microsoft Store (STORE-004, ADR-0019): <see cref="XboxLibraryScanner"/> como
/// <see cref="ILibrarySource"/>. "Cliente instalado" = app Xbox e Serviços de Jogos presentes; sem eles, o aviso de
/// <see cref="XboxGameRules.Notice"/> aparece nas Configurações.
/// </summary>
public sealed class XboxLibrarySource(IXboxSystem system) : ILibrarySource
{
    public GameStore Store => GameStore.Xbox;

    public LibraryScan Scan()
    {
        var result = XboxLibraryScanner.Scan(system);
        return new LibraryScan(
            [.. result.Games.Select(XboxGames.ToGame)],
            result.Availability == XboxAvailability.Ready,
            XboxGameRules.Notice(result.Availability));
    }
}
