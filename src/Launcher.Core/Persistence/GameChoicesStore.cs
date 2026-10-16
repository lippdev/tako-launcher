using Launcher.Core.Library.Metadata.Art;

namespace Launcher.Core.Persistence;

/// <summary>
/// Escolhas do usuário por jogo no <c>tako.db</c> (ADR-0023): a imagem fixada por tipo e o id fixado por fonte. Ficam
/// fora do cache (limpar o cache de imagens ou de metadados não as apaga).
/// </summary>
public sealed class GameChoicesStore(TakoDatabase database)
{
    /// <summary>Todas as imagens fixadas (lidas uma vez ao montar a biblioteca: uma consulta, não uma por jogo).</summary>
    public IReadOnlyDictionary<string, IReadOnlyDictionary<ArtKind, Uri>> LoadAllArt()
    {
        var result = new Dictionary<string, Dictionary<ArtKind, Uri>>(StringComparer.Ordinal);
        using var connection = database.Open();
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT game_id, kind, url FROM game_art;";
        using var reader = command.ExecuteReader();
        while (reader.Read())
        {
            if (ArtKinds.FromKey(reader.GetString(1)) is { } kind && Uri.TryCreate(reader.GetString(2), UriKind.Absolute, out var url))
            {
                string gameId = reader.GetString(0);
                if (!result.TryGetValue(gameId, out var kinds))
                {
                    result[gameId] = kinds = [];
                }

                kinds[kind] = url;
            }
        }

        return result.ToDictionary(kv => kv.Key, kv => (IReadOnlyDictionary<ArtKind, Uri>)kv.Value, StringComparer.Ordinal);
    }

    /// <summary>Fixa a imagem do tipo; nula volta para a automática.</summary>
    public void SetArt(string gameId, ArtKind kind, Uri? url)
    {
        ArgumentException.ThrowIfNullOrEmpty(gameId);
        using var connection = database.Open();
        using var command = connection.CreateCommand();
        command.Parameters.AddWithValue("$id", gameId);
        command.Parameters.AddWithValue("$kind", ArtKinds.Key(kind));
        if (url is null)
        {
            command.CommandText = "DELETE FROM game_art WHERE game_id = $id AND kind = $kind;";
        }
        else
        {
            command.CommandText = "INSERT OR REPLACE INTO game_art (game_id, kind, url) VALUES ($id, $kind, $url);";
            command.Parameters.AddWithValue("$url", url.AbsoluteUri);
        }

        command.ExecuteNonQuery();
    }

    /// <summary>Ids fixados por fonte; id vazio = "nenhum" (a fonte fica de fora deste jogo).</summary>
    public IReadOnlyDictionary<string, string> LoadIds(string gameId)
    {
        ArgumentException.ThrowIfNullOrEmpty(gameId);
        var result = new Dictionary<string, string>(StringComparer.Ordinal);
        using var connection = database.Open();
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT source, source_id FROM game_source_ids WHERE game_id = $id;";
        command.Parameters.AddWithValue("$id", gameId);
        using var reader = command.ExecuteReader();
        while (reader.Read())
        {
            result[reader.GetString(0)] = reader.GetString(1);
        }

        return result;
    }

    /// <summary>Fixa o id na fonte (vazio = "nenhum"); nulo volta para a correspondência automática.</summary>
    public void SetId(string gameId, string source, string? sourceId)
    {
        ArgumentException.ThrowIfNullOrEmpty(gameId);
        ArgumentException.ThrowIfNullOrEmpty(source);
        using var connection = database.Open();
        using var command = connection.CreateCommand();
        command.Parameters.AddWithValue("$id", gameId);
        command.Parameters.AddWithValue("$source", source);
        if (sourceId is null)
        {
            command.CommandText = "DELETE FROM game_source_ids WHERE game_id = $id AND source = $source;";
        }
        else
        {
            command.CommandText = "INSERT OR REPLACE INTO game_source_ids (game_id, source, source_id) VALUES ($id, $source, $sourceId);";
            command.Parameters.AddWithValue("$sourceId", sourceId.Trim());
        }

        command.ExecuteNonQuery();
    }
}
