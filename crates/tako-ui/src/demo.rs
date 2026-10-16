//! Biblioteca de demonstração (`--simulate`): testes, capturas de tela e o CI sem lojas instaladas.

use chrono::{Duration as Days, Utc};
use std::time::Duration;
use tako_core::library::{CollectionInfo, CollectionStore, Game, GameStore};

const TITLES: [(&str, &str, GameStore); 12] = [
    ("Hollow Knight", "#3B4A6B", GameStore::Steam),
    ("Celeste", "#7A3FD1", GameStore::Steam),
    ("Hades", "#C8102E", GameStore::Epic),
    ("Stardew Valley", "#2E7D32", GameStore::Gog),
    ("Forza Horizon 5", "#E65100", GameStore::Xbox),
    ("Cyberpunk 2077", "#C9B200", GameStore::Gog),
    ("Elden Ring", "#8D6E63", GameStore::Steam),
    ("Dead Cells", "#B71C1C", GameStore::Steam),
    ("Ori and the Will of the Wisps", "#1565C0", GameStore::Xbox),
    ("Spelunky 2", "#6D4C41", GameStore::Epic),
    ("Balatro", "#AD1457", GameStore::Steam),
    ("Meu Emulador", "#3D5A80", GameStore::Manual),
];

pub fn games() -> Vec<Game> {
    let now = Utc::now();
    TITLES
        .iter()
        .enumerate()
        .map(|(i, (title, accent, store))| Game {
            id: format!("{}:demo{i}", store.id_prefix()),
            title: (*title).into(),
            store: Some(*store),
            accent: (*accent).into(),
            is_installed: i % 5 != 4,
            launch_target: Some(format!("demo://{i}")),
            added_at: Some(now - Days::days(i as i64 * 3)),
            last_played_at: (i < 4).then(|| now - Days::hours(i as i64 * 20 + 2)),
            play_time: Duration::from_secs(3600 * (12 - i as u64)),
            genres: vec![
                "Ação".into(),
                if i % 2 == 0 {
                    "Plataforma".into()
                } else {
                    "RPG".into()
                },
            ],
            description: format!("{title}: jogo de demonstração do Tako."),
            developer: Some("Estúdio".into()),
            size_label: Some(format!("{} GB", 2 + i * 3)),
            ..Default::default()
        })
        .collect()
}

#[derive(Default)]
pub struct MemoryCollections(Vec<CollectionInfo>);

impl CollectionStore for MemoryCollections {
    fn load_all(&mut self) -> Result<Vec<CollectionInfo>, String> {
        Ok(self.0.clone())
    }
    fn insert(&mut self, c: &CollectionInfo) -> Result<(), String> {
        self.0.push(c.clone());
        Ok(())
    }
    fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        self.0
            .iter_mut()
            .filter(|c| c.id == id)
            .for_each(|c| c.name = name.into());
        Ok(())
    }
    fn delete(&mut self, id: &str) -> Result<(), String> {
        self.0.retain(|c| c.id != id);
        Ok(())
    }
    fn add_game(&mut self, cid: &str, gid: &str, _at: chrono::DateTime<Utc>) -> Result<(), String> {
        self.0.iter_mut().filter(|c| c.id == cid).for_each(|c| {
            c.game_ids.insert(gid.into());
        });
        Ok(())
    }
    fn remove_game(&mut self, cid: &str, gid: &str) -> Result<(), String> {
        self.0.iter_mut().filter(|c| c.id == cid).for_each(|c| {
            c.game_ids.remove(gid);
        });
        Ok(())
    }
}
