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
        // Como reconhecer o fim do jogo (MVP-026): nome da estratégia ou nulo (automático). ADD COLUMN não aceita
        // IF NOT EXISTS, mas roda na mesma transação que registra a versão, então não fica pela metade.
        new(3, """
            ALTER TABLE manual_games ADD COLUMN track_by TEXT;
            """),
        // Coleções (UX-003) e configurações por jogo (UX-007). Coluna nula = "usar o padrão do Tako". Os ids de jogo
        // não têm chave estrangeira: jogos de loja somem e voltam com a mesma identidade a cada varredura.
        new(4, """
            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS collection_games (
                collection_id TEXT NOT NULL,
                game_id TEXT NOT NULL,
                added_at TEXT NOT NULL,
                PRIMARY KEY (collection_id, game_id)
            );
            CREATE TABLE IF NOT EXISTS game_settings (
                game_id TEXT PRIMARY KEY,
                use_game_screen INTEGER,
                screen_mode TEXT,
                hdr INTEGER,
                audio_device_id TEXT,
                audio_device_name TEXT,
                fps_limit INTEGER,
                show_fps INTEGER,
                extra_arguments TEXT
            );
            """),
        // Sistema da ROM adicionada pelo "Adicionar jogo" (ADR-0022): nulo = jogo de PC.
        new(5, """
            ALTER TABLE manual_games ADD COLUMN platform TEXT;
            """),
        // Escolhas do usuário por jogo (ADR-0023): imagem fixada por tipo (capa, fundo, logo, ícone) e id fixado por
        // fonte ("Fontes e IDs"; id vazio = "nenhum"). O agente de metadados nunca troca o que está aqui.
        new(6, """
            CREATE TABLE IF NOT EXISTS game_art (
                game_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                url TEXT NOT NULL,
                PRIMARY KEY (game_id, kind)
            );
            CREATE TABLE IF NOT EXISTS game_source_ids (
                game_id TEXT NOT NULL,
                source TEXT NOT NULL,
                source_id TEXT NOT NULL,
                PRIMARY KEY (game_id, source)
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
