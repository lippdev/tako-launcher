//! Contrato do perfil e dos perfis locais (`profile.*`, `profiles.*`) pelo mesmo parser JSON do hospedeiro.
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, Local, TimeDelta, TimeZone, Utc};
use serde_json::{Value, json};
use std::time::Duration;
use tako_app::{
    AppService, Core,
    protocol::{self, Call, Reply, Response},
};
use tako_core::library::{Game, GameStore, PlaySession};
use tako_core::paths::DataPaths;
use tako_core::persistence::{self as db, TakoDatabase};
use tako_core::profiles::{MAIN, Profiles};
use tako_platform::Platform;

struct SnapshotSource(Vec<Game>);

impl tako_sources::LibrarySource for SnapshotSource {
    fn store(&self) -> GameStore {
        GameStore::Steam
    }

    fn scan(&self) -> tako_sources::LibraryScan {
        tako_sources::LibraryScan {
            games: self.0.clone(),
            ..Default::default()
        }
    }
}

fn settled(core: Core) -> AppService {
    let mut app = AppService::with_core("test", core);
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while matches!(
        app.handle(Call::LibrarySources),
        Ok(Reply::LibrarySources(s)) if s.scanning
    ) {
        assert!(
            std::time::Instant::now() < deadline,
            "leitura de abertura não terminou"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    app
}

/// Núcleo real (banco na pasta temporária) com os jogos do instantâneo; sem lojas desta máquina.
fn open(dir: &tempfile::TempDir, games: Vec<Game>, simulate: bool) -> AppService {
    let paths = DataPaths::at(dir.path());
    if !simulate {
        std::fs::create_dir_all(paths.cache()).unwrap();
        std::fs::write(
            paths.cache().join("found-games.json"),
            serde_json::to_vec(&games).unwrap(),
        )
        .unwrap();
    }
    let core = Core::open_with_sources(
        paths,
        Platform::simulated(),
        simulate,
        vec![Box::new(SnapshotSource(if simulate {
            Vec::new()
        } else {
            games
        }))],
    )
    .unwrap();
    settled(core)
}

fn wire(app: &mut AppService, name: &str, payload: Option<Value>) -> Value {
    let mut value = json!({"v":1,"id":7,"name":name});
    if let Some(payload) = payload {
        value["payload"] = payload;
    }
    let response = match protocol::parse_request(&value.to_string()) {
        Ok(request) => match app.handle(request.call) {
            Ok(reply) => Response::ok(request.id, reply),
            Err(error) => Response::err(request.id, error),
        },
        Err((id, error)) => Response::err(id, error),
    };
    serde_json::to_value(response).unwrap()
}

fn ok(app: &mut AppService, name: &str, payload: Option<Value>) -> Value {
    let reply = wire(app, name, payload);
    assert_eq!(reply["ok"], true, "{name}: {reply}");
    reply["reply"]["data"].clone()
}

fn err(app: &mut AppService, name: &str, payload: Option<Value>, code: &str) {
    let reply = wire(app, name, payload);
    assert_eq!(reply["ok"], false, "{name}: {reply}");
    assert_eq!(reply["error"]["code"], code, "{name}: {reply}");
}

fn game(id: &str, title: &str, minutes: u64, installed: bool, added_days_ago: i64) -> Game {
    Game {
        id: id.into(),
        title: title.into(),
        store: Some(GameStore::Steam),
        is_installed: installed,
        play_time: Duration::from_secs(minutes * 60),
        added_at: Some(Utc::now() - TimeDelta::days(added_days_ago)),
        ..Default::default()
    }
}

/// Hoje (ou `days_ago` dias atrás) no horário local, para os dias não dependerem da hora em que o teste roda.
fn local(days_ago: u64, hour: u32, minute: u32) -> DateTime<Utc> {
    let day = Local::now().date_naive() - chrono::Days::new(days_ago);
    Local
        .from_local_datetime(&day.and_hms_opt(hour, minute, 0).unwrap())
        .earliest()
        .unwrap()
        .with_timezone(&Utc)
}

fn session(at: DateTime<Utc>, seconds: u64) -> PlaySession {
    PlaySession {
        started_at: at,
        duration: Duration::from_secs(seconds),
    }
}

/// Quatro jogos, cinco sessões (duas hoje, uma delas rápida) e uma coleção do usuário.
fn seeded() -> (tempfile::TempDir, AppService) {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::at(dir.path());
    let database = TakoDatabase::new(paths.database());
    let store = db::LibraryStore::new(database.clone(), None);
    let mut data = store.load().unwrap();
    let state = |sessions: Vec<PlaySession>| db::GameStateRow {
        last_played_at: sessions.iter().map(|s| s.started_at).max(),
        sessions,
        ..Default::default()
    };
    data.state.insert(
        "steam:1".into(),
        state(vec![
            session(local(40, 20, 0), 90 * 60),
            session(local(0, 12, 0), 45 * 60),
            session(local(0, 8, 0), 30),
        ]),
    );
    data.state.insert(
        "steam:2".into(),
        state(vec![session(local(40, 10, 0), 60 * 60)]),
    );
    data.state.insert(
        "steam:3".into(),
        state(vec![session(local(0, 10, 0), 10 * 60)]),
    );
    store.save(&data).unwrap();
    let now = Utc::now();
    let collections = db::CollectionStore::new(database);
    collections.insert("sofa", "Sofá", now).unwrap();
    collections.add_game("sofa", "steam:1", now).unwrap();
    collections.add_game("sofa", "steam:2", now).unwrap();
    collections.insert("vazia", "Vazia", now).unwrap();
    let games = vec![
        game("steam:1", "Hades II", 135, true, 5),
        game("steam:2", "Elden Ring", 60, false, 1),
        game("steam:3", "Celeste", 10, true, 30),
        game("steam:4", "Nunca jogado", 0, true, 2),
    ];
    let app = open(&dir, games, false);
    (dir, app)
}

fn ids(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap())
        .collect()
}

#[test]
fn summary_gives_totals_the_week_top_games_and_recent_sessions() {
    let (_dir, mut app) = seeded();
    let summary = ok(&mut app, "profile.summary", None);
    assert_eq!(summary["gameCount"], 4);
    assert_eq!(summary["totalPlaySeconds"], (135 + 60 + 10) * 60);
    assert_eq!(summary["profile"]["id"], MAIN);
    assert_eq!(summary["profile"]["main"], true);
    assert_eq!(summary["profile"]["name"], "");
    // Mais jogados: por tempo, sem os nunca jogados.
    assert_eq!(ids(&summary["topGames"]), ["steam:1", "steam:2", "steam:3"]);
    // Últimos adicionados: o mais recente primeiro.
    assert_eq!(
        ids(&summary["latestGames"]),
        ["steam:2", "steam:4", "steam:1"]
    );
    // A semana tem sete dias seguidos, de segunda a domingo, e hoje soma 45 + 10 min (a rápida não conta).
    let week = summary["week"].as_array().unwrap();
    assert_eq!(week.len(), 7);
    let today = summary["today"].as_str().unwrap();
    let today_seconds = week
        .iter()
        .find(|d| d["day"] == today)
        .map(|d| d["seconds"].as_u64().unwrap());
    assert_eq!(today_seconds, Some(55 * 60));
    // As outras sessões têm 40 dias: a semana só soma as de hoje.
    assert_eq!(
        week.iter()
            .map(|d| d["seconds"].as_u64().unwrap())
            .sum::<u64>(),
        55 * 60
    );
    // Sessões recentes: da mais recente para a mais antiga, no máximo cinco.
    let recent = summary["recentSessions"].as_array().unwrap();
    assert_eq!(recent.len(), 5);
    let starts: Vec<&str> = recent
        .iter()
        .map(|s| s["startedAt"].as_str().unwrap())
        .collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(starts, sorted);
    assert_eq!(recent[0]["game"]["id"], "steam:1");
    assert_eq!(recent[0]["durationSeconds"], 45 * 60);
    assert_eq!(recent[0]["quick"], false);
    let quick: Vec<&Value> = recent.iter().filter(|s| s["quick"] == true).collect();
    assert_eq!(quick.len(), 1);
    assert_eq!(quick[0]["durationSeconds"], 30);
    assert!(summary["lastPlayedAt"].is_string());
}

#[test]
fn sessions_are_grouped_by_local_day_and_filtered_by_period() {
    let (_dir, mut app) = seeded();
    let all = ok(&mut app, "profile.sessions", Some(json!({"period": "all"})));
    assert_eq!(all["count"], 5);
    // A sessão rápida entra na contagem, mas não no tempo: 45 + 10 + 90 + 60 min.
    assert_eq!(all["totalSeconds"], (45 + 10 + 90 + 60) * 60);
    let days = all["days"].as_array().unwrap();
    assert_eq!(days.len(), 2);
    assert_eq!(days[0]["day"], all["today"]);
    assert_eq!(days[0]["sessions"].as_array().unwrap().len(), 3);
    assert_eq!(days[0]["totalSeconds"], 55 * 60);
    // As duas de 40 dias atrás caem no mesmo dia local (20:00 e 10:00).
    assert_eq!(days[1]["sessions"].as_array().unwrap().len(), 2);
    assert_eq!(days[1]["totalSeconds"], 150 * 60);

    let week = ok(
        &mut app,
        "profile.sessions",
        Some(json!({"period": "week"})),
    );
    assert_eq!(week["count"], 3);
    assert_eq!(week["totalSeconds"], 55 * 60);
    assert_eq!(week["days"].as_array().unwrap().len(), 1);
    // 40 dias atrás nunca cabe no mês.
    let month = ok(
        &mut app,
        "profile.sessions",
        Some(json!({"period": "month"})),
    );
    assert_eq!(month["count"], 3);
}

#[test]
fn collections_count_games_installed_and_cover_the_mosaic() {
    let (_dir, mut app) = seeded();
    ok(
        &mut app,
        "collections.smart.set",
        Some(json!({
            "collectionId": null,
            "name": "Instalados",
            "criteria": {"library": {"installed": true}}
        })),
    );
    let data = ok(&mut app, "profile.collections", None);
    let list = data["collections"].as_array().unwrap();
    assert_eq!(list.len(), 3);
    let by_name = |name: &str| list.iter().find(|c| c["name"] == name).unwrap();
    let sofa = by_name("Sofá");
    assert_eq!(sofa["automatic"], false);
    assert_eq!(sofa["gameCount"], 2);
    assert_eq!(sofa["installedCount"], 1);
    assert_eq!(sofa["covers"].as_array().unwrap().len(), 2);
    assert_eq!(sofa["id"], "sofa");
    let empty = by_name("Vazia");
    assert_eq!(empty["gameCount"], 0);
    assert!(empty["covers"].as_array().unwrap().is_empty());
    let smart = by_name("Instalados");
    assert_eq!(smart["automatic"], true);
    assert_eq!(smart["gameCount"], 3);
    assert_eq!(smart["installedCount"], 3);
    // Só as coleções do usuário entram na conta dos jogos.
    assert_eq!(data["gameCount"], 2);
}

#[test]
fn an_empty_profile_answers_with_zeros_and_no_lists() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, Vec::new(), false);
    let summary = ok(&mut app, "profile.summary", None);
    assert_eq!(summary["gameCount"], 0);
    assert_eq!(summary["totalPlaySeconds"], 0);
    assert!(summary["lastPlayedAt"].is_null());
    assert!(summary["topGames"].as_array().unwrap().is_empty());
    assert!(summary["latestGames"].as_array().unwrap().is_empty());
    assert!(summary["recentSessions"].as_array().unwrap().is_empty());
    let week = summary["week"].as_array().unwrap();
    assert_eq!(week.len(), 7);
    assert!(week.iter().all(|d| d["seconds"] == 0));
    let sessions = ok(&mut app, "profile.sessions", Some(json!({"period": "all"})));
    assert_eq!(sessions["count"], 0);
    assert!(sessions["days"].as_array().unwrap().is_empty());
    let collections = ok(&mut app, "profile.collections", None);
    assert!(collections["collections"].as_array().unwrap().is_empty());
    assert_eq!(collections["gameCount"], 0);
}

#[test]
fn profile_calls_reject_strange_input() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, Vec::new(), false);
    for name in ["profile.summary", "profile.collections", "profiles.list"] {
        err(&mut app, name, Some(json!({})), "bad_request");
    }
    for payload in [
        json!([]),
        json!("all"),
        json!({}),
        json!({"period": "decade"}),
        json!({"period": "all", "extra": 1}),
    ] {
        err(&mut app, "profile.sessions", Some(payload), "bad_request");
    }
    err(&mut app, "profile.sessions", None, "bad_request");
    for (name, payload) in [
        ("profiles.create", json!([])),
        ("profiles.create", json!({})),
        ("profiles.create", json!({"name": 7})),
        ("profiles.create", json!({"name": "A", "x": 1})),
        ("profiles.rename", json!({"profileId": "x"})),
        ("profiles.delete", json!({"profileId": ""})),
        ("profiles.delete", json!({"profileId": "x\u{0}y"})),
        ("profiles.switch", json!({"profileId": "a".repeat(200)})),
        ("profiles.ask.set", json!({"on": "sim"})),
    ] {
        err(&mut app, name, Some(payload), "bad_request");
    }
}

#[test]
fn profiles_are_created_renamed_switched_and_deleted_with_the_core_rules() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, Vec::new(), false);
    let list = ok(&mut app, "profiles.list", None);
    assert_eq!(list["profiles"].as_array().unwrap().len(), 1);
    assert_eq!(list["profiles"][0]["id"], MAIN);
    assert_eq!(list["profiles"][0]["inUse"], true);
    assert_eq!(list["maxProfiles"], 8);
    assert_eq!(list["maxNameChars"], 32);
    assert_eq!(list["restartRequired"], false);
    assert_eq!(list["askOnStart"], false);

    // Criar: grava na pasta da instalação; nome repetido (sem olhar maiúsculas) ou inválido recusa.
    let created = ok(&mut app, "profiles.create", Some(json!({"name": "  Ana "})));
    assert_eq!(created["profiles"][1]["name"], "Ana");
    assert_eq!(Profiles::load(dir.path()).list.len(), 2);
    err(
        &mut app,
        "profiles.create",
        Some(json!({"name": "ana"})),
        "conflict",
    );
    err(
        &mut app,
        "profiles.create",
        Some(json!({"name": "   "})),
        "bad_request",
    );
    err(
        &mut app,
        "profiles.create",
        Some(json!({"name": "x".repeat(33)})),
        "bad_request",
    );
    err(
        &mut app,
        "profiles.create",
        Some(json!({"name": "a\u{7}b"})),
        "bad_request",
    );
    let ana = created["profiles"][1]["id"].as_str().unwrap().to_owned();

    // Renomear (inclusive o principal) e perfil que não existe.
    let renamed = ok(
        &mut app,
        "profiles.rename",
        Some(json!({"profileId": ana, "name": "Ana Clara"})),
    );
    assert_eq!(renamed["profiles"][1]["name"], "Ana Clara");
    let main = ok(
        &mut app,
        "profiles.rename",
        Some(json!({"profileId": MAIN, "name": "Filipe"})),
    );
    assert_eq!(main["profiles"][0]["name"], "Filipe");
    err(
        &mut app,
        "profiles.rename",
        Some(json!({"profileId": "nada", "name": "X"})),
        "not_found",
    );
    err(
        &mut app,
        "profiles.rename",
        Some(json!({"profileId": ana, "name": "filipe"})),
        "conflict",
    );

    // Perguntar ao abrir.
    let ask = ok(&mut app, "profiles.ask.set", Some(json!({"on": true})));
    assert_eq!(ask["askOnStart"], true);
    assert!(Profiles::load(dir.path()).ask_on_start);

    // Trocar grava para a próxima abertura e marca a escolha; os dados em uso seguem os do principal.
    let switched = ok(&mut app, "profiles.switch", Some(json!({"profileId": ana})));
    assert_eq!(switched["restartRequired"], true);
    assert_eq!(switched["profiles"][0]["inUse"], true);
    assert_eq!(switched["profiles"][1]["next"], true);
    assert_eq!(Profiles::load(dir.path()).active, ana);
    assert!(tako_core::profiles::take_chosen(dir.path()));
    err(
        &mut app,
        "profiles.switch",
        Some(json!({"profileId": "nada"})),
        "not_found",
    );
    // O gravado para a próxima abertura e o aberto agora não se apagam.
    err(
        &mut app,
        "profiles.delete",
        Some(json!({"profileId": ana})),
        "conflict",
    );
    err(
        &mut app,
        "profiles.delete",
        Some(json!({"profileId": MAIN})),
        "conflict",
    );
    err(
        &mut app,
        "profiles.delete",
        Some(json!({"profileId": "nada"})),
        "not_found",
    );
    // Voltar ao principal cancela a troca; aí a Ana pode sair, com a pasta de dados.
    let back = ok(
        &mut app,
        "profiles.switch",
        Some(json!({"profileId": MAIN})),
    );
    assert_eq!(back["restartRequired"], false);
    let folder = dir.path().join("profiles").join(&ana);
    std::fs::create_dir_all(&folder).unwrap();
    let deleted = ok(&mut app, "profiles.delete", Some(json!({"profileId": ana})));
    assert_eq!(deleted["profiles"].as_array().unwrap().len(), 1);
    assert!(!folder.exists());
    assert!(Profiles::load(dir.path()).find(&ana).is_none());
}

#[test]
fn the_list_is_limited_to_eight_profiles() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, Vec::new(), false);
    for i in 1..8 {
        ok(
            &mut app,
            "profiles.create",
            Some(json!({"name": format!("Perfil {i}")})),
        );
    }
    err(
        &mut app,
        "profiles.create",
        Some(json!({"name": "Nono"})),
        "conflict",
    );
    assert_eq!(Profiles::load(dir.path()).list.len(), 8);
}

#[test]
fn a_profile_opened_after_a_switch_reports_its_own_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = open(&dir, Vec::new(), false);
    let created = ok(&mut first, "profiles.create", Some(json!({"name": "Bia"})));
    let bia = created["profiles"][1]["id"].as_str().unwrap().to_owned();
    ok(
        &mut first,
        "profiles.switch",
        Some(json!({"profileId": bia})),
    );
    drop(first);
    // Reabre como o hospedeiro faz: a pasta do perfil escolhido.
    let machine = DataPaths::at(dir.path());
    let paths = Profiles::load(dir.path()).paths(&machine);
    let core = Core::open_with_sources(
        paths,
        Platform::simulated(),
        false,
        vec![Box::new(SnapshotSource(Vec::new()))],
    )
    .unwrap();
    let mut app = settled(core);
    let summary = ok(&mut app, "profile.summary", None);
    assert_eq!(summary["profile"]["id"], bia);
    assert_eq!(summary["profile"]["name"], "Bia");
    assert_eq!(summary["profile"]["main"], false);
    let list = ok(&mut app, "profiles.list", None);
    assert_eq!(list["restartRequired"], false);
    assert_eq!(list["profiles"][1]["inUse"], true);
    // O principal agora pode ser apagado? Não: é protegido; e o em uso também.
    err(
        &mut app,
        "profiles.delete",
        Some(json!({"profileId": MAIN})),
        "conflict",
    );
    err(
        &mut app,
        "profiles.delete",
        Some(json!({"profileId": bia})),
        "conflict",
    );
}

#[test]
fn the_demo_keeps_profiles_in_memory_and_never_touches_the_disk() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = open(&dir, Vec::new(), true);
    let created = ok(&mut app, "profiles.create", Some(json!({"name": "Demo"})));
    assert_eq!(created["profiles"].as_array().unwrap().len(), 2);
    let id = created["profiles"][1]["id"].as_str().unwrap().to_owned();
    ok(&mut app, "profiles.switch", Some(json!({"profileId": id})));
    ok(&mut app, "profiles.ask.set", Some(json!({"on": true})));
    let list = ok(&mut app, "profiles.list", None);
    assert_eq!(list["restartRequired"], true);
    assert_eq!(list["askOnStart"], true);
    assert!(!Profiles::file(dir.path()).exists());
    assert!(!dir.path().join("profile-chosen").exists());
}
