//! Port de `GameSettingsStore.cs` ("Configurações deste jogo", UX-007): uma linha por jogo com alguma escolha
//! própria. Voltar tudo ao padrão apaga a linha.

use super::{Result, TakoDatabase, require};
use crate::session::settings::GameSettingsOverride;
use rusqlite::{OptionalExtension, params};

#[derive(Clone)]
pub struct GameSettingsStore {
    db: TakoDatabase,
}

impl GameSettingsStore {
    pub fn new(db: TakoDatabase) -> GameSettingsStore {
        GameSettingsStore { db }
    }

    pub fn load(&self, game_id: &str) -> Result<GameSettingsOverride> {
        require(game_id, "game_id")?;
        let conn = self.db.open()?;
        let row = conn
            .query_row(
                "SELECT use_game_screen, screen_mode, hdr, audio_device_id, audio_device_name, fps_limit, show_fps,
                        extra_arguments, working_directory, environment, before_command, after_command,
                        approved_commands, screen_id, screen_name, vrr, tools, close_tools
                 FROM game_settings WHERE game_id = ?1;",
                [game_id],
                |r| {
                    Ok(GameSettingsOverride {
                        use_game_screen: r.get::<_, Option<i64>>(0)?.map(|v| v != 0),
                        screen_mode: r.get(1)?,
                        hdr: r.get::<_, Option<i64>>(2)?.map(|v| v != 0),
                        audio_device_id: r.get(3)?,
                        audio_device_name: r.get(4)?,
                        fps_limit: r
                            .get::<_, Option<i64>>(5)?
                            .and_then(|v| i32::try_from(v).ok()),
                        show_fps: r.get::<_, Option<i64>>(6)?.map(|v| v != 0),
                        extra_arguments: r.get(7)?,
                        working_directory: r.get(8)?,
                        environment: r.get(9)?,
                        before_command: r.get(10)?,
                        after_command: r.get(11)?,
                        approved_commands: r.get(12)?,
                        screen_id: r.get(13)?,
                        screen_name: r.get(14)?,
                        vrr: r.get::<_, Option<i64>>(15)?.map(|v| v != 0),
                        tools: r.get(16)?,
                        close_tools: r.get::<_, Option<i64>>(17)?.map(|v| v != 0),
                    })
                },
            )
            .optional()?;
        Ok(row.unwrap_or_default())
    }

    pub fn save(&self, game_id: &str, settings: &GameSettingsOverride) -> Result<()> {
        require(game_id, "game_id")?;
        let conn = self.db.open()?;
        // O FPS não conta em `is_default`, mas a linha fica se a alpha gravou um: não perde o dado (D-06).
        if settings.is_default()
            && settings.fps_limit.is_none()
            && settings.show_fps.is_none()
            && settings.approved_commands.is_none()
        {
            conn.execute("DELETE FROM game_settings WHERE game_id = ?1;", [game_id])?;
            return Ok(());
        }
        conn.execute(
            "INSERT OR REPLACE INTO game_settings
                (game_id, use_game_screen, screen_mode, hdr, audio_device_id, audio_device_name, fps_limit, show_fps,
                 extra_arguments, working_directory, environment, before_command, after_command, approved_commands,
                 screen_id, screen_name, vrr, tools, close_tools)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);",
            params![
                game_id,
                settings.use_game_screen,
                settings.screen_mode,
                settings.hdr,
                settings.audio_device_id,
                settings.audio_device_name,
                settings.fps_limit,
                settings.show_fps,
                normalize(settings.extra_arguments.as_deref()),
                normalize(settings.working_directory.as_deref()),
                normalize(settings.environment.as_deref()),
                normalize(settings.before_command.as_deref()),
                normalize(settings.after_command.as_deref()),
                settings.approved_commands,
                normalize(settings.screen_id.as_deref()),
                settings.screen_name,
                settings.vrr,
                normalize(settings.tools.as_deref()),
                settings.close_tools,
            ],
        )?;
        Ok(())
    }
}

/// `GameSettingsResolver.NormalizeArguments`: em branco = nada; o resto sem espaços nas pontas.
fn normalize(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::migrator::tests::count;

    #[test]
    fn store_round_trips_game_settings_and_deletes_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tako.db");
        let store = GameSettingsStore::new(TakoDatabase::new(&path));
        let settings = GameSettingsOverride {
            use_game_screen: Some(true),
            screen_id: Some(r"\\.\DISPLAY1".into()),
            screen_name: Some("Monitor".into()),
            screen_mode: Some(GameSettingsOverride::KEEP_CURRENT_MODE.into()),
            hdr: Some(false),
            vrr: Some(true),
            audio_device_id: Some("{fone}".into()),
            audio_device_name: Some("Fone".into()),
            fps_limit: Some(-1),
            show_fps: Some(true),
            extra_arguments: Some(" -dx11 ".into()),
            working_directory: Some("  ".into()),
            environment: Some("DXVK_HUD=fps".into()),
            before_command: Some(" antes.bat ".into()),
            after_command: None,
            approved_commands: Some("antes.bat\n".into()),
            tools: Some(" C:/FPS/fps.exe ".into()),
            close_tools: Some(true),
        };

        assert_eq!(
            store.load("steam:570").unwrap(),
            GameSettingsOverride::default()
        );
        store.save("steam:570", &settings).unwrap();

        let expected = GameSettingsOverride {
            extra_arguments: Some("-dx11".into()),
            working_directory: None,
            before_command: Some("antes.bat".into()),
            tools: Some("C:/FPS/fps.exe".into()),
            ..settings
        };
        assert_eq!(store.load("steam:570").unwrap(), expected);
        assert_eq!(
            store.load("steam:730").unwrap(),
            GameSettingsOverride::default()
        );

        let only_fps = GameSettingsOverride {
            show_fps: Some(false),
            ..Default::default()
        };
        store.save("steam:570", &only_fps).unwrap();
        assert_eq!(store.load("steam:570").unwrap(), only_fps);

        let folder = GameSettingsOverride {
            working_directory: Some(" D:/Jogos/X ".into()),
            ..Default::default()
        };
        store.save("steam:570", &folder).unwrap();
        assert_eq!(
            store
                .load("steam:570")
                .unwrap()
                .working_directory
                .as_deref(),
            Some("D:/Jogos/X")
        );

        store
            .save("steam:570", &GameSettingsOverride::default())
            .unwrap();
        let conn = TakoDatabase::new(&path).open().unwrap();
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM game_settings;"), 0);
    }
}
