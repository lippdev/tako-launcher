using System.Globalization;
using Launcher.Core.Library;

namespace Launcher.Core.Persistence;

/// <summary>Coleções do usuário (UX-003) no <c>tako.db</c>: cada mudança grava na hora, numa transação.</summary>
public sealed class CollectionStore(TakoDatabase database)
{
    public IReadOnlyList<CollectionInfo> LoadAll()
    {
        using var connection = database.Open();
        var members = new Dictionary<string, HashSet<string>>(StringComparer.Ordinal);
        using (var command = connection.CreateCommand())
        {
            command.CommandText = "SELECT collection_id, game_id FROM collection_games ORDER BY added_at;";
            using var reader = command.ExecuteReader();
            while (reader.Read())
            {
                string id = reader.GetString(0);
                if (!members.TryGetValue(id, out var games))
                {
                    members[id] = games = new HashSet<string>(StringComparer.Ordinal);
                }

                games.Add(reader.GetString(1));
            }
        }

        var result = new List<CollectionInfo>();
        using (var command = connection.CreateCommand())
        {
            command.CommandText = "SELECT id, name, created_at FROM collections ORDER BY created_at, rowid;";
            using var reader = command.ExecuteReader();
            while (reader.Read())
            {
                string id = reader.GetString(0);
                result.Add(new CollectionInfo(id, reader.GetString(1), ParseDate(reader.GetString(2)), members.GetValueOrDefault(id) ?? []));
            }
        }

        return result;
    }

    public void Insert(CollectionInfo collection)
    {
        ArgumentNullException.ThrowIfNull(collection);
        Execute(
            "INSERT INTO collections (id, name, created_at) VALUES ($id, $name, $at);",
            ("$id", collection.Id),
            ("$name", collection.Name),
            ("$at", FormatDate(collection.CreatedAt)));
    }

    public void Rename(string id, string name) =>
        Execute("UPDATE collections SET name = $name WHERE id = $id;", ("$id", id), ("$name", name));

    /// <summary>Apaga a coleção e a lista de jogos dela (os jogos continuam na biblioteca).</summary>
    public void Delete(string id) =>
        Execute("DELETE FROM collection_games WHERE collection_id = $id; DELETE FROM collections WHERE id = $id;", ("$id", id));

    public void AddGame(string collectionId, string gameId, DateTimeOffset at) =>
        Execute(
            "INSERT OR IGNORE INTO collection_games (collection_id, game_id, added_at) VALUES ($c, $g, $at);",
            ("$c", collectionId),
            ("$g", gameId),
            ("$at", FormatDate(at)));

    public void RemoveGame(string collectionId, string gameId) =>
        Execute("DELETE FROM collection_games WHERE collection_id = $c AND game_id = $g;", ("$c", collectionId), ("$g", gameId));

    private void Execute(string sql, params (string Name, string Value)[] parameters)
    {
        using var connection = database.Open();
        using var transaction = connection.BeginTransaction();
        using (var command = connection.CreateCommand())
        {
            command.Transaction = transaction;
            command.CommandText = sql;
            foreach (var (name, value) in parameters)
            {
                command.Parameters.AddWithValue(name, value);
            }

            command.ExecuteNonQuery();
        }

        transaction.Commit();
    }

    private static string FormatDate(DateTimeOffset value) => value.ToString("O", CultureInfo.InvariantCulture);

    private static DateTimeOffset ParseDate(string value) => DateTimeOffset.Parse(value, CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind);
}
