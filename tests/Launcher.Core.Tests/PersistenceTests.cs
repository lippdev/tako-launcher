using Launcher.Core.Library;
using Launcher.Core.Persistence;
using Microsoft.Data.Sqlite;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class PersistenceTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), "tako-tests-" + Guid.NewGuid().ToString("N"));

    public PersistenceTests() => Directory.CreateDirectory(_root);

    private string DbPath => Path.Combine(_root, "tako.db");

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }

    [Fact]
    public void Migrator_upgrades_a_v1_database_keeping_its_data()
    {
        using (var connection = Connect())
        {
            SchemaMigrator.Migrate(connection, targetVersion: 1);
            Assert.Equal(1, SchemaMigrator.CurrentVersion(connection));
            Exec(connection, "INSERT INTO game_state (game_id, is_favorite, play_minutes) VALUES ('steam:570', 1, 12.5);");
            Assert.False(TableExists(connection, "settings"));

            SchemaMigrator.Migrate(connection);

            Assert.Equal(SchemaMigrator.LatestVersion, SchemaMigrator.CurrentVersion(connection));
            Assert.True(TableExists(connection, "settings"));
            Assert.Equal(1L, Scalar(connection, "SELECT COUNT(*) FROM game_state;"));

            // Idempotente: rodar de novo não reaplica nada.
            SchemaMigrator.Migrate(connection);
            Assert.Equal((long)SchemaMigrator.Migrations.Count, Scalar(connection, "SELECT COUNT(*) FROM schema_migrations;"));
        }
    }

    [Fact]
    public void Migrations_are_in_strictly_increasing_order()
    {
        var versions = SchemaMigrator.Migrations.Select(m => m.Version).ToList();
        Assert.Equal(Enumerable.Range(1, versions.Count), versions);
    }

    [Fact]
    public void Sqlite_store_round_trips_manual_games_and_state()
    {
        var added = new DateTimeOffset(2026, 10, 15, 12, 0, 0, TimeSpan.FromHours(-3));
        var store = new SqliteLibraryStore(new TakoDatabase(DbPath));
        store.Load();
        store.Manual.Add(new ManualGameRecord { Id = "manual:a", Title = "Jogo A", Path = "C:/a.exe", Arguments = "-x", AddedAt = added });
        store.Manual.Add(new ManualGameRecord { Id = "manual:b", Title = "Jogo B", Path = "steam://run/1", AddedAt = added });
        store.State["steam:570"] = new GameStateRecord { IsFavorite = true, LastPlayedAt = added, PlayMinutes = 42.5 };
        store.Save();

        var reopened = new SqliteLibraryStore(new TakoDatabase(DbPath));
        reopened.Load();

        Assert.Equal(["manual:a", "manual:b"], reopened.Manual.Select(m => m.Id));
        Assert.Equal("-x", reopened.Manual[0].Arguments);
        Assert.Null(reopened.Manual[1].Arguments);
        Assert.Equal(added, reopened.Manual[0].AddedAt);
        Assert.Equal(new GameStateRecord { IsFavorite = true, LastPlayedAt = added, PlayMinutes = 42.5 }, reopened.State["steam:570"]);

        reopened.Manual.RemoveAt(0);
        reopened.State.Clear();
        reopened.Save();
        var third = new SqliteLibraryStore(new TakoDatabase(DbPath));
        third.Load();
        Assert.Equal(["manual:b"], third.Manual.Select(m => m.Id));
        Assert.Empty(third.State);
    }

    [Fact]
    public void Local_library_works_on_top_of_sqlite()
    {
        var time = new FixedTimeProvider(new DateTimeOffset(2026, 10, 15, 12, 0, 0, TimeSpan.Zero));
        var library = new LocalGameLibrary(() => [], new SqliteLibraryStore(new TakoDatabase(DbPath)), time);
        library.Load();
        var game = library.AddManual(ManualGameRecord.Create("Meu Jogo", Path.Combine(_root, "jogo.exe"), time.GetUtcNow()));
        library.ToggleFavorite(game.Id);

        var reopened = new LocalGameLibrary(() => [], new SqliteLibraryStore(new TakoDatabase(DbPath)), time);
        reopened.Load();

        Assert.True(reopened.Find(game.Id)!.IsFavorite);
    }

    [Fact]
    public void First_run_imports_library_json_and_renames_it()
    {
        string json = Path.Combine(_root, "library.json");
        var legacy = new LibraryStore(json);
        legacy.Manual.Add(new ManualGameRecord { Id = "manual:velho", Title = "Velho", Path = "C:/velho.exe", AddedAt = DateTimeOffset.UnixEpoch });
        legacy.State["steam:10"] = new GameStateRecord { IsFavorite = true, PlayMinutes = 3 };
        legacy.Save();

        var store = new SqliteLibraryStore(new TakoDatabase(DbPath), json);
        store.Load();

        Assert.Equal("Velho", Assert.Single(store.Manual).Title);
        Assert.True(store.State["steam:10"].IsFavorite);
        Assert.False(File.Exists(json));
        Assert.True(File.Exists(json + SqliteLibraryStore.ImportedSuffix));

        // Segunda abertura: nada a importar, os dados continuam no banco.
        var again = new SqliteLibraryStore(new TakoDatabase(DbPath), json);
        again.Load();
        Assert.Single(again.Manual);
    }

    [Fact]
    public void Reimport_never_overwrites_newer_data_in_the_database()
    {
        string json = Path.Combine(_root, "library.json");
        var store = new SqliteLibraryStore(new TakoDatabase(DbPath));
        store.Load();
        store.State["steam:10"] = new GameStateRecord { IsFavorite = false, PlayMinutes = 99 };
        store.Save();
        var legacy = new LibraryStore(json);
        legacy.State["steam:10"] = new GameStateRecord { IsFavorite = true, PlayMinutes = 1 };
        legacy.Save();

        var importing = new SqliteLibraryStore(new TakoDatabase(DbPath), json);
        importing.Load();

        Assert.Equal(99, importing.State["steam:10"].PlayMinutes);
    }

    [Fact]
    public void Corrupted_legacy_json_is_still_moved_aside()
    {
        string json = Path.Combine(_root, "library.json");
        File.WriteAllText(json, "{ isto não é json");

        var store = new SqliteLibraryStore(new TakoDatabase(DbPath), json);
        store.Load();

        Assert.Empty(store.Manual);
        Assert.True(File.Exists(json + ".corrompido"));
        Assert.True(File.Exists(json + SqliteLibraryStore.ImportedSuffix));
    }

    [Fact]
    public void Corrupted_database_is_renamed_and_starts_empty()
    {
        File.WriteAllText(DbPath, "isto não é um banco SQLite, só texto qualquer com tamanho suficiente para o cabeçalho");
        var database = new TakoDatabase(DbPath);
        var store = new SqliteLibraryStore(database);

        store.Load();

        Assert.Empty(store.Manual);
        Assert.True(database.RecoveredFromCorruption);
        Assert.True(File.Exists(DbPath + ".corrompido"));
        store.State["x"] = new GameStateRecord { IsFavorite = true };
        store.Save();
        var reopened = new SqliteLibraryStore(new TakoDatabase(DbPath));
        reopened.Load();
        Assert.True(reopened.State["x"].IsFavorite);
    }

    [Fact]
    public void Settings_store_upserts_and_reads_back()
    {
        var settings = new SettingsStore(new TakoDatabase(DbPath));
        Assert.Empty(settings.LoadAll());

        settings.Set("DynamicColor", "false");
        settings.Set("DynamicColor", "true");
        settings.Set("ReducedMotion", "true");

        var reopened = new SettingsStore(new TakoDatabase(DbPath)).LoadAll();
        Assert.Equal("true", reopened["DynamicColor"]);
        Assert.Equal("true", reopened["ReducedMotion"]);
        Assert.Equal(2, reopened.Count);
    }

    private SqliteConnection Connect()
    {
        var connection = new SqliteConnection(new SqliteConnectionStringBuilder { DataSource = DbPath, Pooling = false }.ToString());
        connection.Open();
        return connection;
    }

    private static void Exec(SqliteConnection connection, string sql)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        command.ExecuteNonQuery();
    }

    private static object? Scalar(SqliteConnection connection, string sql)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        return command.ExecuteScalar();
    }

    private static bool TableExists(SqliteConnection connection, string table)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = $name;";
        command.Parameters.AddWithValue("$name", table);
        return (long)command.ExecuteScalar()! == 1;
    }
}
