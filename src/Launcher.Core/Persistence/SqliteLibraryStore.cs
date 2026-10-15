using System.Globalization;
using Launcher.Core.Library;
using Launcher.Core.Session;
using Microsoft.Data.Sqlite;

namespace Launcher.Core.Persistence;

/// <summary>
/// Biblioteca do usuário no <c>tako.db</c> (MVP-005). Na primeira abertura importa o <c>library.json</c> antigo
/// (<paramref name="legacyJsonPath"/>) e o renomeia para <c>library.json.importado</c>.
/// </summary>
public sealed class SqliteLibraryStore(TakoDatabase database, string? legacyJsonPath = null) : ILibraryStore
{
    public const string ImportedSuffix = ".importado";

    public List<ManualGameRecord> Manual { get; private set; } = [];

    public Dictionary<string, GameStateRecord> State { get; private set; } = [];

    public void Load()
    {
        using var connection = database.Open();
        ImportLegacyJson(connection);
        Manual = ReadManual(connection);
        State = ReadState(connection);
    }

    /// <summary>Regrava as duas tabelas numa transação: ou tudo é salvo, ou nada muda.</summary>
    public void Save()
    {
        using var connection = database.Open();
        using var transaction = connection.BeginTransaction();
        Execute(connection, transaction, "DELETE FROM manual_games; DELETE FROM game_state;");
        Write(connection, transaction, Manual, State, replace: true);
        transaction.Commit();
    }

    private void ImportLegacyJson(SqliteConnection connection)
    {
        if (legacyJsonPath is null || !File.Exists(legacyJsonPath))
        {
            return;
        }

        // O leitor JSON já trata arquivo corrompido (guarda .corrompido e devolve vazio).
        var legacy = new LibraryStore(legacyJsonPath);
        legacy.Load();
        using (var transaction = connection.BeginTransaction())
        {
            // INSERT OR IGNORE: reimportar (se a renomeação falhar) nunca sobrescreve dados mais novos do banco.
            Write(connection, transaction, legacy.Manual, legacy.State, replace: false);
            transaction.Commit();
        }

        File.Move(legacyJsonPath, legacyJsonPath + ImportedSuffix, overwrite: true);
    }

    private static void Write(SqliteConnection connection, SqliteTransaction transaction, List<ManualGameRecord> manual, Dictionary<string, GameStateRecord> state, bool replace)
    {
        string verb = replace ? "INSERT OR REPLACE" : "INSERT OR IGNORE";
        using (var command = connection.CreateCommand())
        {
            command.Transaction = transaction;
            command.CommandText = $"""
                {verb} INTO manual_games (id, title, path, arguments, working_directory, cover_path, hero_path, added_at, track_by)
                VALUES ($id, $title, $path, $arguments, $workingDirectory, $coverPath, $heroPath, $addedAt, $trackBy);
                """;
            var id = command.Parameters.Add("$id", SqliteType.Text);
            var title = command.Parameters.Add("$title", SqliteType.Text);
            var path = command.Parameters.Add("$path", SqliteType.Text);
            var arguments = command.Parameters.Add("$arguments", SqliteType.Text);
            var workingDirectory = command.Parameters.Add("$workingDirectory", SqliteType.Text);
            var coverPath = command.Parameters.Add("$coverPath", SqliteType.Text);
            var heroPath = command.Parameters.Add("$heroPath", SqliteType.Text);
            var addedAt = command.Parameters.Add("$addedAt", SqliteType.Text);
            var trackBy = command.Parameters.Add("$trackBy", SqliteType.Text);
            foreach (var record in manual)
            {
                id.Value = record.Id;
                title.Value = record.Title;
                path.Value = record.Path;
                arguments.Value = (object?)record.Arguments ?? DBNull.Value;
                workingDirectory.Value = (object?)record.WorkingDirectory ?? DBNull.Value;
                coverPath.Value = (object?)record.CoverPath ?? DBNull.Value;
                heroPath.Value = (object?)record.HeroPath ?? DBNull.Value;
                addedAt.Value = FormatDate(record.AddedAt);
                trackBy.Value = record.TrackBy is { } strategy ? strategy.ToString() : DBNull.Value;
                command.ExecuteNonQuery();
            }
        }

        using (var command = connection.CreateCommand())
        {
            command.Transaction = transaction;
            command.CommandText = $"""
                {verb} INTO game_state (game_id, is_favorite, last_played_at, play_minutes)
                VALUES ($id, $favorite, $lastPlayed, $minutes);
                """;
            var id = command.Parameters.Add("$id", SqliteType.Text);
            var favorite = command.Parameters.Add("$favorite", SqliteType.Integer);
            var lastPlayed = command.Parameters.Add("$lastPlayed", SqliteType.Text);
            var minutes = command.Parameters.Add("$minutes", SqliteType.Real);
            foreach (var (gameId, record) in state)
            {
                id.Value = gameId;
                favorite.Value = record.IsFavorite ? 1 : 0;
                lastPlayed.Value = record.LastPlayedAt is { } at ? FormatDate(at) : DBNull.Value;
                minutes.Value = record.PlayMinutes;
                command.ExecuteNonQuery();
            }
        }
    }

    private static List<ManualGameRecord> ReadManual(SqliteConnection connection)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT id, title, path, arguments, working_directory, cover_path, hero_path, added_at, track_by FROM manual_games ORDER BY rowid;";
        using var reader = command.ExecuteReader();
        var result = new List<ManualGameRecord>();
        while (reader.Read())
        {
            result.Add(new ManualGameRecord
            {
                Id = reader.GetString(0),
                Title = reader.GetString(1),
                Path = reader.GetString(2),
                Arguments = reader.IsDBNull(3) ? null : reader.GetString(3),
                WorkingDirectory = reader.IsDBNull(4) ? null : reader.GetString(4),
                CoverPath = reader.IsDBNull(5) ? null : reader.GetString(5),
                HeroPath = reader.IsDBNull(6) ? null : reader.GetString(6),
                AddedAt = ParseDate(reader.GetString(7)),
                // Valor desconhecido (versão mais nova gravou outro nome) volta ao automático.
                TrackBy = !reader.IsDBNull(8) && Enum.TryParse<GameTrackingStrategy>(reader.GetString(8), out var strategy) ? strategy : null,
            });
        }

        return result;
    }

    private static Dictionary<string, GameStateRecord> ReadState(SqliteConnection connection)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT game_id, is_favorite, last_played_at, play_minutes FROM game_state;";
        using var reader = command.ExecuteReader();
        var result = new Dictionary<string, GameStateRecord>();
        while (reader.Read())
        {
            result[reader.GetString(0)] = new GameStateRecord
            {
                IsFavorite = reader.GetInt64(1) != 0,
                LastPlayedAt = reader.IsDBNull(2) ? null : ParseDate(reader.GetString(2)),
                PlayMinutes = reader.GetDouble(3),
            };
        }

        return result;
    }

    private static void Execute(SqliteConnection connection, SqliteTransaction transaction, string sql)
    {
        using var command = connection.CreateCommand();
        command.Transaction = transaction;
        command.CommandText = sql;
        command.ExecuteNonQuery();
    }

    private static string FormatDate(DateTimeOffset value) => value.ToString("O", CultureInfo.InvariantCulture);

    private static DateTimeOffset ParseDate(string value) => DateTimeOffset.Parse(value, CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind);
}
