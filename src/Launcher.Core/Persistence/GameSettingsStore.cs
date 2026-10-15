using Launcher.Core.Session;
using Microsoft.Data.Sqlite;

namespace Launcher.Core.Persistence;

/// <summary>
/// "Configurações deste jogo" (UX-007) no <c>tako.db</c>: uma linha por jogo com alguma escolha própria. Voltar tudo ao
/// padrão apaga a linha.
/// </summary>
public sealed class GameSettingsStore(TakoDatabase database)
{
    public GameSettingsOverride Load(string gameId)
    {
        ArgumentException.ThrowIfNullOrEmpty(gameId);
        using var connection = database.Open();
        using var command = connection.CreateCommand();
        command.CommandText = """
            SELECT use_game_screen, screen_mode, hdr, audio_device_id, audio_device_name, fps_limit, show_fps, extra_arguments
            FROM game_settings WHERE game_id = $id;
            """;
        command.Parameters.AddWithValue("$id", gameId);
        using var reader = command.ExecuteReader();
        if (!reader.Read())
        {
            return GameSettingsOverride.None;
        }

        return new GameSettingsOverride
        {
            UseGameScreen = Bool(reader, 0),
            ScreenMode = Text(reader, 1),
            Hdr = Bool(reader, 2),
            AudioDeviceId = Text(reader, 3),
            AudioDeviceName = Text(reader, 4),
            FpsLimit = reader.IsDBNull(5) ? null : (int)reader.GetInt64(5),
            ShowFps = Bool(reader, 6),
            ExtraArguments = Text(reader, 7),
        };
    }

    public void Save(string gameId, GameSettingsOverride settings)
    {
        ArgumentException.ThrowIfNullOrEmpty(gameId);
        ArgumentNullException.ThrowIfNull(settings);
        using var connection = database.Open();
        using var command = connection.CreateCommand();
        command.Parameters.AddWithValue("$id", gameId);
        if (settings.IsDefault)
        {
            command.CommandText = "DELETE FROM game_settings WHERE game_id = $id;";
            command.ExecuteNonQuery();
            return;
        }

        command.CommandText = """
            INSERT OR REPLACE INTO game_settings
                (game_id, use_game_screen, screen_mode, hdr, audio_device_id, audio_device_name, fps_limit, show_fps, extra_arguments)
            VALUES ($id, $screen, $mode, $hdr, $audioId, $audioName, $fps, $showFps, $args);
            """;
        command.Parameters.AddWithValue("$screen", Value(settings.UseGameScreen));
        command.Parameters.AddWithValue("$mode", (object?)settings.ScreenMode ?? DBNull.Value);
        command.Parameters.AddWithValue("$hdr", Value(settings.Hdr));
        command.Parameters.AddWithValue("$audioId", (object?)settings.AudioDeviceId ?? DBNull.Value);
        command.Parameters.AddWithValue("$audioName", (object?)settings.AudioDeviceName ?? DBNull.Value);
        command.Parameters.AddWithValue("$fps", settings.FpsLimit is { } fps ? fps : DBNull.Value);
        command.Parameters.AddWithValue("$showFps", Value(settings.ShowFps));
        command.Parameters.AddWithValue("$args", (object?)GameSettingsResolver.NormalizeArguments(settings.ExtraArguments) ?? DBNull.Value);
        command.ExecuteNonQuery();
    }

    private static object Value(bool? value) => value is { } b ? (b ? 1 : 0) : DBNull.Value;

    private static bool? Bool(SqliteDataReader reader, int column) => reader.IsDBNull(column) ? null : reader.GetInt64(column) != 0;

    private static string? Text(SqliteDataReader reader, int column) => reader.IsDBNull(column) ? null : reader.GetString(column);
}
