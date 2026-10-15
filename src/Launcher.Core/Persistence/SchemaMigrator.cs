using System.Globalization;
using Microsoft.Data.Sqlite;

namespace Launcher.Core.Persistence;

/// <summary>Uma versão do esquema. O SQL é idempotente (<c>IF NOT EXISTS</c>) para tolerar uma migração interrompida.</summary>
public sealed record Migration(int Version, string Sql);

/// <summary>
/// Migrações versionadas do <c>tako.db</c>: a tabela <c>schema_migrations</c> guarda as versões aplicadas e cada
/// migração pendente roda em ordem, na própria transação. Nunca edite uma migração publicada; acrescente outra.
/// </summary>
public static class SchemaMigrator
{
    public static IReadOnlyList<Migration> Migrations { get; } =
    [
        new(1, """
            CREATE TABLE IF NOT EXISTS manual_games (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                path TEXT NOT NULL,
                arguments TEXT,
                working_directory TEXT,
                cover_path TEXT,
                hero_path TEXT,
                added_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS game_state (
                game_id TEXT PRIMARY KEY,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                last_played_at TEXT,
                play_minutes REAL NOT NULL DEFAULT 0
            );
            """),
        new(2, """
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            """),
    ];

    public static int LatestVersion => Migrations[^1].Version;

    /// <summary>Versão atual do banco (0 quando ainda não foi migrado).</summary>
    public static int CurrentVersion(SqliteConnection connection)
    {
        ArgumentNullException.ThrowIfNull(connection);
        EnsureVersionTable(connection);
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT COALESCE(MAX(version), 0) FROM schema_migrations;";
        return Convert.ToInt32(command.ExecuteScalar(), CultureInfo.InvariantCulture);
    }

    /// <summary>Aplica as migrações pendentes, em ordem, até <paramref name="targetVersion"/> (padrão: a mais recente).</summary>
    public static void Migrate(SqliteConnection connection, int? targetVersion = null)
    {
        int current = CurrentVersion(connection);
        int target = targetVersion ?? LatestVersion;
        foreach (var migration in Migrations.Where(m => m.Version > current && m.Version <= target).OrderBy(m => m.Version))
        {
            using var transaction = connection.BeginTransaction();
            using (var command = connection.CreateCommand())
            {
                command.Transaction = transaction;
                command.CommandText = migration.Sql;
                command.ExecuteNonQuery();
            }

            using (var record = connection.CreateCommand())
            {
                record.Transaction = transaction;
                record.CommandText = "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES ($version, $at);";
                record.Parameters.AddWithValue("$version", migration.Version);
                record.Parameters.AddWithValue("$at", DateTimeOffset.UtcNow.ToString("O", CultureInfo.InvariantCulture));
                record.ExecuteNonQuery();
            }

            transaction.Commit();
        }
    }

    private static void EnsureVersionTable(SqliteConnection connection)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);";
        command.ExecuteNonQuery();
    }
}
