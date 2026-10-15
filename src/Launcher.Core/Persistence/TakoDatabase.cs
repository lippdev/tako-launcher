using Microsoft.Data.Sqlite;

namespace Launcher.Core.Persistence;

/// <summary>
/// Banco local do Tako (<c>tako.db</c>, SQLite): entrega conexões já migradas. Arquivo corrompido não derruba o
/// launcher: é renomeado para <c>.corrompido</c> e o banco recomeça vazio.
/// </summary>
public sealed class TakoDatabase
{
    // SQLITE_CORRUPT e SQLITE_NOTADB.
    private const int SqliteCorrupt = 11;
    private const int SqliteNotADatabase = 26;

    private readonly Lock _gate = new();
    private readonly string _connectionString;
    private bool _ready;

    public TakoDatabase(string path)
    {
        FilePath = path;
        // Sem pool: o arquivo fica livre entre as operações (renomear, apagar pastas temporárias dos testes).
        _connectionString = new SqliteConnectionStringBuilder { DataSource = path, Pooling = false }.ToString();
    }

    public string FilePath { get; }

    /// <summary>Verdadeiro quando o arquivo estava corrompido e foi trocado por um banco vazio nesta execução.</summary>
    public bool RecoveredFromCorruption { get; private set; }

    /// <summary>Conexão aberta com o esquema na versão mais recente. Quem chama descarta.</summary>
    public SqliteConnection Open()
    {
        EnsureReady();
        return Connect();
    }

    private void EnsureReady()
    {
        lock (_gate)
        {
            if (_ready)
            {
                return;
            }

            try
            {
                Initialize();
            }
            catch (SqliteException ex) when (ex.SqliteErrorCode is SqliteCorrupt or SqliteNotADatabase)
            {
                QuarantineCorruptedFile();
                Initialize();
            }

            _ready = true;
        }
    }

    private void Initialize()
    {
        string? directory = Path.GetDirectoryName(FilePath);
        if (!string.IsNullOrEmpty(directory))
        {
            Directory.CreateDirectory(directory);
        }

        using var connection = Connect();
        using (var check = connection.CreateCommand())
        {
            // Página danificada nem sempre falha ao abrir; a verificação rápida percorre o arquivo (que é pequeno).
            check.CommandText = "PRAGMA quick_check;";
            if (check.ExecuteScalar() is not "ok")
            {
                throw new SqliteException("Banco corrompido (quick_check).", SqliteCorrupt);
            }
        }

        SchemaMigrator.Migrate(connection);
    }

    private void QuarantineCorruptedFile()
    {
        File.Move(FilePath, FilePath + ".corrompido", overwrite: true);
        foreach (string suffix in (string[])["-journal", "-wal", "-shm"])
        {
            File.Delete(FilePath + suffix);
        }

        RecoveredFromCorruption = true;
    }

    private SqliteConnection Connect()
    {
        var connection = new SqliteConnection(_connectionString);
        connection.Open();
        return connection;
    }
}
