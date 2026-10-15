using System.IO;
using Launcher.Core.Library;
using Launcher.Core.Persistence;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf;

public partial class App
{
    /// <summary>Coleções do usuário (UX-003). Banco ilegível: o Tako abre sem coleções em vez de cair.</summary>
    private static CollectionLibrary LoadCollections(IServiceProvider services)
    {
        var collections = new CollectionLibrary(services.GetRequiredService<CollectionStore>(), services.GetRequiredService<TimeProvider>());
        try
        {
            collections.Load();
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogCollectionsFailed(services.GetRequiredService<ILogger<App>>(), ex);
        }

        return collections;
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível ler as coleções; seguindo sem elas")]
    private static partial void LogCollectionsFailed(ILogger logger, Exception exception);
}
