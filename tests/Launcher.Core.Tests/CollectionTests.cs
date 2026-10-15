using Launcher.Core.Library;
using Launcher.Core.Persistence;
using Launcher.Tests;
using Microsoft.Data.Sqlite;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>UX-003: coleções do usuário, gravadas no tako.db e usadas como filtro da biblioteca.</summary>
public sealed class CollectionTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), "tako-tests-" + Guid.NewGuid().ToString("N"));

    public CollectionTests() => Directory.CreateDirectory(_root);

    private string DbPath => Path.Combine(_root, "tako.db");

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }

    [Fact]
    public void Migration_4_adds_collections_and_game_settings_to_a_v3_database()
    {
        using var connection = new SqliteConnection(new SqliteConnectionStringBuilder { DataSource = DbPath, Pooling = false }.ToString());
        connection.Open();
        SchemaMigrator.Migrate(connection, targetVersion: 3);
        Exec(connection, "INSERT INTO manual_games (id, title, path, added_at, track_by) VALUES ('manual:a', 'A', 'C:/a.exe', '2026-10-15T00:00:00Z', 'Process');");
        Assert.False(TableExists(connection, "collections"));

        SchemaMigrator.Migrate(connection);

        Assert.True(SchemaMigrator.CurrentVersion(connection) >= 4);
        Assert.True(TableExists(connection, "collections"));
        Assert.True(TableExists(connection, "collection_games"));
        Assert.True(TableExists(connection, "game_settings"));
        Assert.Equal(1L, Scalar(connection, "SELECT COUNT(*) FROM manual_games;"));
    }

    [Theory]
    [InlineData("", "Dê um nome à coleção.")]
    [InlineData("   ", "Dê um nome à coleção.")]
    [InlineData("rpg", "Já existe uma coleção com esse nome.")]
    [InlineData("Ação", "Já existe uma coleção com esse nome.")] // sem acento/caixa: "acao"
    [InlineData("Para jogar com a família no sábado à noite", "Use até 40 letras.")]
    [InlineData("  Corrida   rápida ", null)]
    public void Collection_names_are_checked(string name, string? problem)
    {
        CollectionInfo[] existing = [Collection("1", "RPG"), Collection("2", "acao")];

        Assert.Equal(problem, CollectionNames.Problem(name, existing));
    }

    [Fact]
    public void Renaming_may_keep_the_same_name()
    {
        CollectionInfo[] existing = [Collection("1", "RPG")];

        Assert.Null(CollectionNames.Problem("rpg", existing, exceptId: "1"));
        Assert.Equal("Corrida rápida", CollectionNames.Clean("  Corrida   rápida "));
    }

    [Fact]
    public void Collections_are_created_renamed_filled_and_deleted_on_disk()
    {
        var time = TimeProvider.System;
        var library = new CollectionLibrary(new CollectionStore(new TakoDatabase(DbPath)), time);
        library.Load();
        int changes = 0;
        library.Changed += (_, _) => changes++;

        var rpg = library.Create("  RPG ", firstGameId: "steam:570");
        var racing = library.Create("Corrida");
        Assert.True(library.Toggle(racing.Id, "steam:730"));
        Assert.True(library.Toggle(racing.Id, "steam:570"));
        Assert.False(library.Toggle(racing.Id, "steam:730")); // tirou
        library.Rename(rpg.Id, "Aventura");
        Assert.Throws<ArgumentException>(() => library.Create("corrida"));
        Assert.Throws<ArgumentException>(() => library.Rename(rpg.Id, " "));

        var reopened = new CollectionLibrary(new CollectionStore(new TakoDatabase(DbPath)), time);
        reopened.Load();

        Assert.Equal(["Aventura", "Corrida"], reopened.Collections.Select(c => c.Name)); // em ordem de nome
        Assert.Equal(["steam:570"], reopened.Find(rpg.Id)!.GameIds);
        Assert.Equal(["steam:570"], reopened.Find(racing.Id)!.GameIds);
        Assert.Equal(2, reopened.Of("steam:570").Count);
        Assert.Empty(reopened.Of("steam:730"));
        Assert.Equal(6, changes);

        reopened.Delete(racing.Id);
        var third = new CollectionLibrary(new CollectionStore(new TakoDatabase(DbPath)), time);
        third.Load();
        Assert.Equal(["Aventura"], third.Collections.Select(c => c.Name));
        using var connection = new TakoDatabase(DbPath).Open();
        Assert.Equal(1L, Scalar(connection, "SELECT COUNT(*) FROM collection_games;"));
    }

    [Fact]
    public void Collections_filter_the_library_with_the_other_filters()
    {
        var games = SampleLibrary.Load(TimeProvider.System);
        var library = new CollectionLibrary(new CollectionStore(new TakoDatabase(DbPath)), TimeProvider.System);
        library.Load();
        var couch = library.Create("Sofá", games[0].Id);
        library.Toggle(couch.Id, games[1].Id);
        var solo = library.Create("Sozinho", games[2].Id);
        library.ApplyTo(games);

        var criteria = LibraryCriteria.None with { Sort = LibrarySort.Name };
        var onlyCouch = LibraryQuery.Apply(games, LibraryFilter.All, criteria.ToggleCollection(couch.Id));
        var either = LibraryQuery.Apply(games, LibraryFilter.All, criteria.ToggleCollection(couch.Id).ToggleCollection(solo.Id));

        Assert.Equal(new[] { games[0].Id, games[1].Id }.Order(), onlyCouch.Select(g => g.Id).Order());
        Assert.Equal(3, either.Count);
        Assert.Equal(1, criteria.ToggleCollection(couch.Id).ActiveFilterCount);
        Assert.Empty(criteria.ToggleCollection(couch.Id).ClearFilters().Collections);
        Assert.Empty(criteria.ToggleCollection("apagada").KeepCollections([couch.Id]).Collections);
        Assert.Contains(couch.Id, games[0].Collections);
        Assert.Empty(games[3].Collections);
    }

    private static CollectionInfo Collection(string id, string name) => new(id, name, DateTimeOffset.UnixEpoch, new HashSet<string>());

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
