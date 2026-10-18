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
            size_bytes: Some((2 + i as u64 * 3) << 30),
            ..Default::default()
        })
        .collect()
}

/// Jogos da conta que ainda não estão no computador, de três lojas e um baixando (SRC-005): capturas e testes da
/// seção NÃO INSTALADOS.
#[cfg(test)]
pub fn not_installed() -> Vec<Game> {
    use tako_core::library::InstallProgress;
    let rows: [(&str, &str, GameStore, Option<f32>); 14] = [
        ("Hollow Knight: Silksong", "#2B4F6B", GameStore::Steam, None),
        ("Disco Elysium", "#6B2B3A", GameStore::Steam, Some(0.42)),
        ("Tunic", "#2B6B6B", GameStore::Steam, None),
        ("Inscryption", "#6B5A2B", GameStore::Steam, None),
        ("Terraria", "#2F6B3A", GameStore::Steam, None),
        ("Cuphead", "#6B602B", GameStore::Steam, None),
        ("Vampire Survivors", "#5A2B6B", GameStore::Steam, None),
        ("Alan Wake 2", "#2B3A6B", GameStore::Epic, None),
        ("Control", "#6B2B2B", GameStore::Epic, None),
        ("Fortnite", "#2B5A6B", GameStore::Epic, None),
        ("Rocket League", "#3A6B2B", GameStore::Epic, None),
        ("The Witcher 3: Wild Hunt", "#6B3A2B", GameStore::Gog, None),
        (
            "Heroes of Might and Magic 3",
            "#4A6B2B",
            GameStore::Gog,
            None,
        ),
        (
            "Disco Elysium: The Final Cut",
            "#6B2B5A",
            GameStore::Gog,
            None,
        ),
    ];
    rows.iter()
        .enumerate()
        .map(|(i, (title, accent, store, progress))| Game {
            id: format!("{}:ni{i}", store.id_prefix()),
            title: (*title).into(),
            store: Some(*store),
            accent: (*accent).into(),
            is_installed: false,
            install_uri: Some(format!("demo://install/{i}")),
            installing: progress.map(|f| InstallProgress { fraction: Some(f) }),
            install_size: Some((3 + i as u64) << 30),
            ..Default::default()
        })
        .collect()
}

/// Sessões e coleções de demonstração para a captura da TelaPerfil: alguns jogos nos últimos dias.
#[cfg(test)]
pub fn sessions(ctx: &mut crate::shell::Ctx) {
    use tako_core::library::{GameStateRecord, PlaySession};
    type StateChange = Box<dyn FnOnce(Option<GameStateRecord>) -> GameStateRecord>;
    let ids: Vec<String> = ctx.library.games().iter().map(|g| g.id.clone()).collect();
    let now = Utc::now();
    // (jogo, dias atrás, horas atrás no dia, minutos)
    let plan: [(usize, i64, i64, u64); 8] = [
        (2, 0, 1, 122),
        (1, 0, 5, 80),
        (0, 1, 3, 40),
        (2, 2, 2, 95),
        (3, 3, 4, 20),
        (2, 4, 1, 120),
        (1, 5, 2, 50),
        (0, 9, 2, 75),
    ];
    let state: Vec<(String, StateChange)> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let list: Vec<PlaySession> = plan
                .iter()
                .rev()
                .filter(|p| p.0 == i)
                .map(|&(_, d, h, m)| PlaySession {
                    started_at: now - Days::days(d) - Days::hours(h),
                    duration: Duration::from_secs(m * 60),
                })
                .collect();
            let change: StateChange = Box::new(move |s: Option<GameStateRecord>| GameStateRecord {
                sessions: list,
                ..s.unwrap_or_default()
            });
            (id.clone(), change)
        })
        .collect();
    ctx.library.import(Vec::new(), state).unwrap();
    for (name, games) in [("Roguelikes", &[2, 7, 9][..]), ("Sofá", &[0, 1, 3, 4][..])] {
        if let Ok(c) = ctx.collections.create(name, Some(&ids[games[0]])) {
            let cid = c.id.clone();
            for g in &games[1..] {
                let _ = ctx.collections.toggle(&cid, &ids[*g]);
            }
        }
    }
    ctx.sync_collections();
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
