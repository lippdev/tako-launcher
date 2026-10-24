//! Perfil em uso e perfis locais pela ponte (UX-004, WEB-14c): os números da tela Perfil (`profile.*`) e a gestão da
//! lista de perfis (`profiles.*`), com as mesmas regras do app Slint (`tako-ui/src/shell/profile.rs` e `profiles.rs`).
//!
//! - **Dados do perfil.** O núcleo abre a pasta de um perfil só (`DataPaths::for_profile`); `profile.*` fala dele.
//!   Trocar de perfil grava a escolha em `profiles.toml` e vale na próxima abertura: `restartRequired` avisa a
//!   interface enquanto o gravado difere do que está em uso.
//! - **Dias locais.** A semana, o mês e o agrupamento por dia usam o fuso do computador, como o Slint.
//! - **Demonstração.** Nada é lido nem gravado no disco: a lista de perfis vive na memória.

use crate::protocol::{
    BridgeError, Call, ErrorCode, LibraryGame, LibraryListRequest, ProfileCollection,
    ProfileCollections, ProfileDay, ProfileEntry, ProfileSession, ProfileSessions, ProfileSummary,
    ProfileWeekDay, ProfilesList, Reply, SessionPeriod,
};
use crate::{Core, library, organize};
use chrono::{Datelike, Local, NaiveDate, SecondsFormat};
use std::path::PathBuf;
use std::time::Duration;
use tako_core::library::{Game, PlaySession};
use tako_core::paths::DataPaths;
use tako_core::profiles::{self, MAIN, ProfileError, Profiles};

/// Sessão com menos de 1 minuto: "abriu e fechou", fora dos totais.
const MIN_SESSION: Duration = Duration::from_secs(60);
const RECENT: usize = 5;
/// A guia Sessões mostra as 300 mais recentes do período.
const MAX_SESSIONS: usize = 300;
const TOP: usize = 3;
const COVERS: usize = 4;
const MAX_ID_CHARS: usize = 128;

/// Onde ficam os perfis e qual deles esta abertura do Tako usa.
pub(crate) struct ProfileState {
    machine: PathBuf,
    /// O perfil cujos dados o núcleo abriu.
    opened: String,
    /// Só na demonstração: a lista não toca o disco.
    memory: Option<Profiles>,
}

impl ProfileState {
    pub(crate) fn open(paths: &DataPaths, simulate: bool) -> Self {
        Self {
            machine: paths.machine.clone(),
            opened: if simulate {
                MAIN.to_owned()
            } else {
                Profiles::load(&paths.machine).active
            },
            memory: simulate.then(Profiles::default),
        }
    }

    fn load(&self) -> Profiles {
        self.memory
            .clone()
            .unwrap_or_else(|| Profiles::load(&self.machine))
    }

    fn save(&mut self, list: &Profiles) -> Result<(), BridgeError> {
        match &mut self.memory {
            Some(memory) => {
                *memory = list.clone();
                Ok(())
            }
            None => list.save(&self.machine).map_err(organize::internal),
        }
    }

    fn entries(&self, list: &Profiles) -> Vec<ProfileEntry> {
        list.list
            .iter()
            .map(|p| ProfileEntry {
                id: p.id.clone(),
                name: p.name.clone(),
                main: p.id == MAIN,
                in_use: p.id == self.opened,
                next: p.id == list.active,
            })
            .collect()
    }

    fn reply(&self) -> ProfilesList {
        let list = self.load();
        ProfilesList {
            profiles: self.entries(&list),
            ask_on_start: list.ask_on_start,
            max_profiles: profiles::MAX_PROFILES,
            max_name_chars: profiles::MAX_NAME,
            restart_required: list.active != self.opened,
        }
    }
}

fn bad(message: &str) -> BridgeError {
    BridgeError::new(ErrorCode::BadRequest, message)
}

fn profile_error(error: ProfileError) -> BridgeError {
    match error {
        ProfileError::InvalidName => bad("nome de perfil inválido"),
        ProfileError::DuplicateName => {
            BridgeError::new(ErrorCode::Conflict, "já existe um perfil com esse nome")
        }
        ProfileError::TooMany => BridgeError::new(ErrorCode::Conflict, "perfis demais"),
        ProfileError::Unknown => BridgeError::new(ErrorCode::NotFound, "perfil não encontrado"),
        ProfileError::Protected => BridgeError::new(
            ErrorCode::Conflict,
            "o perfil principal e o perfil em uso não podem ser apagados",
        ),
    }
}

fn check_id(id: &str) -> Result<(), BridgeError> {
    if id.trim().is_empty() || id.chars().count() > MAX_ID_CHARS || id.contains('\0') {
        return Err(bad("id de perfil inválido"));
    }
    Ok(())
}

pub(crate) fn handle(core: &mut Core, call: Call) -> Result<Reply, BridgeError> {
    match call {
        Call::ProfileSummary => Ok(Reply::ProfileSummary(Box::new(summary(
            core,
            Local::now().date_naive(),
        )))),
        Call::ProfileSessions(request) => Ok(Reply::ProfileSessions(sessions(
            core,
            request.period,
            Local::now().date_naive(),
        ))),
        Call::ProfileCollections => Ok(Reply::ProfileCollections(collections(core))),
        Call::ProfilesList => Ok(Reply::ProfilesList(core.profiles.reply())),
        Call::ProfilesCreate(request) => {
            let mut list = core.profiles.load();
            list.add(&request.name).map_err(profile_error)?;
            core.profiles.save(&list)?;
            Ok(Reply::ProfilesCreate(core.profiles.reply()))
        }
        Call::ProfilesRename(request) => {
            check_id(&request.profile_id)?;
            let mut list = core.profiles.load();
            list.rename(&request.profile_id, &request.name)
                .map_err(profile_error)?;
            core.profiles.save(&list)?;
            Ok(Reply::ProfilesRename(core.profiles.reply()))
        }
        Call::ProfilesDelete(request) => {
            check_id(&request.profile_id)?;
            let mut list = core.profiles.load();
            if list.find(&request.profile_id).is_none() {
                return Err(profile_error(ProfileError::Unknown));
            }
            // Os dados do perfil aberto estão em uso, mesmo que a próxima abertura use outro.
            if request.profile_id == core.profiles.opened {
                return Err(profile_error(ProfileError::Protected));
            }
            list.remove(&request.profile_id).map_err(profile_error)?;
            core.profiles.save(&list)?;
            if core.profiles.memory.is_none()
                && let Err(e) = profiles::delete_data(&core.profiles.machine, &request.profile_id)
            {
                // O perfil já saiu da lista; sobram arquivos que o usuário pode apagar à mão.
                tracing::warn!("dados do perfil não apagados: {e}");
            }
            Ok(Reply::ProfilesDelete(core.profiles.reply()))
        }
        Call::ProfilesAskSet(request) => {
            let mut list = core.profiles.load();
            list.ask_on_start = request.on;
            core.profiles.save(&list)?;
            Ok(Reply::ProfilesAskSet(core.profiles.reply()))
        }
        Call::ProfilesSwitch(request) => {
            check_id(&request.profile_id)?;
            let mut list = core.profiles.load();
            list.set_active(&request.profile_id)
                .map_err(profile_error)?;
            core.profiles.save(&list)?;
            if core.profiles.memory.is_none() {
                // A interface reaberta pela troca não pergunta "Quem está jogando?" de novo.
                profiles::mark_chosen(&core.profiles.machine);
            }
            Ok(Reply::ProfilesSwitch(core.profiles.reply()))
        }
        _ => Err(bad("chamada fora do perfil")),
    }
}

fn local_day(s: &PlaySession) -> NaiveDate {
    s.started_at.with_timezone(&Local).date_naive()
}

/// O tempo que conta nos totais (sessão de menos de 1 minuto não soma).
fn counted(s: &PlaySession) -> Duration {
    if s.duration < MIN_SESSION {
        Duration::ZERO
    } else {
        s.duration
    }
}

/// Segunda-feira da semana de `day`.
fn monday(day: NaiveDate) -> NaiveDate {
    day - chrono::Days::new(u64::from(day.weekday().num_days_from_monday()))
}

fn date_text(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

fn instant(at: chrono::DateTime<chrono::Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Todas as sessões do perfil (o histórico é por jogo), da mais recente para a mais antiga.
fn all_sessions(core: &Core) -> Vec<(&Game, &PlaySession)> {
    let mut all: Vec<(&Game, &PlaySession)> = core
        .library
        .games()
        .iter()
        .flat_map(|g| core.library.sessions(&g.id).iter().map(move |s| (g, s)))
        .collect();
    all.sort_by_key(|(_, s)| std::cmp::Reverse(s.started_at));
    all
}

fn session(game: &Game, s: &PlaySession) -> ProfileSession {
    ProfileSession {
        game: library::row(game),
        started_at: instant(s.started_at),
        duration_seconds: s.duration.as_secs(),
        quick: s.duration < MIN_SESSION,
    }
}

/// Tempo de cada dia da semana de `today`, de segunda a domingo.
fn week<'a>(
    sessions: impl IntoIterator<Item = &'a PlaySession>,
    today: NaiveDate,
) -> Vec<ProfileWeekDay> {
    let start = monday(today);
    let mut days = [Duration::ZERO; 7];
    for s in sessions {
        let i = (local_day(s) - start).num_days();
        if (0..7).contains(&i) {
            days[i as usize] += counted(s);
        }
    }
    days.iter()
        .enumerate()
        .map(|(i, d)| ProfileWeekDay {
            day: date_text(start + chrono::Days::new(i as u64)),
            seconds: d.as_secs(),
        })
        .collect()
}

fn summary(core: &Core, today: NaiveDate) -> ProfileSummary {
    let games = core.library.games();
    let sessions = all_sessions(core);
    let list = core.profiles.load();
    let opened = list
        .find(&core.profiles.opened)
        .cloned()
        .unwrap_or_else(|| list.active());
    let profile = ProfileEntry {
        main: opened.id == MAIN,
        in_use: true,
        next: opened.id == list.active,
        id: opened.id,
        name: opened.name,
    };

    let mut top: Vec<&Game> = games.iter().filter(|g| !g.play_time.is_zero()).collect();
    top.sort_by_key(|g| std::cmp::Reverse(g.play_time));
    let mut latest: Vec<&Game> = games.iter().collect();
    latest.sort_by_key(|g| std::cmp::Reverse(g.added_at));

    ProfileSummary {
        profile,
        last_played_at: games
            .iter()
            .filter_map(|g| g.last_played_at)
            .max()
            .map(instant),
        total_play_seconds: games.iter().map(|g| g.play_time.as_secs()).sum(),
        game_count: games.len(),
        today: date_text(today),
        week: week(sessions.iter().map(|(_, s)| *s), today),
        top_games: top.into_iter().take(TOP).map(library::row).collect(),
        latest_games: latest.into_iter().take(TOP).map(library::row).collect(),
        recent_sessions: sessions
            .iter()
            .take(RECENT)
            .map(|(g, s)| session(g, s))
            .collect(),
    }
}

fn sessions(core: &Core, period: SessionPeriod, today: NaiveDate) -> ProfileSessions {
    let since = match period {
        SessionPeriod::Week => Some(monday(today)),
        SessionPeriod::Month => today.with_day(1),
        SessionPeriod::All => None,
    };
    let in_period: Vec<(&Game, &PlaySession)> = all_sessions(core)
        .into_iter()
        .filter(|(_, s)| since.is_none_or(|d| local_day(s) >= d))
        .collect();
    let mut days: Vec<ProfileDay> = Vec::new();
    for (g, s) in in_period.iter().take(MAX_SESSIONS) {
        let day = date_text(local_day(s));
        if days.last().is_none_or(|d| d.day != day) {
            days.push(ProfileDay {
                day,
                total_seconds: 0,
                sessions: Vec::new(),
            });
        }
        if let Some(d) = days.last_mut() {
            d.total_seconds += counted(s).as_secs();
            d.sessions.push(session(g, s));
        }
    }
    ProfileSessions {
        today: date_text(today),
        count: in_period.len(),
        total_seconds: in_period.iter().map(|(_, s)| counted(s).as_secs()).sum(),
        days,
    }
}

/// Os jogos de uma coleção (do usuário ou automática) pelo mesmo caminho de `library.list`.
fn collection_games(
    core: &Core,
    id: &str,
    installed: Option<bool>,
    limit: u32,
) -> (usize, Vec<LibraryGame>) {
    let page = library::list(
        core,
        LibraryListRequest {
            collection: Some(id.to_owned()),
            installed,
            limit,
            ..Default::default()
        },
    );
    (page.total, page.rows)
}

fn collections(core: &Core) -> ProfileCollections {
    let manual = core
        .collections
        .collections()
        .iter()
        .map(|c| (c.id.clone(), c.name.clone(), false));
    let automatic = organize::smart_collections(core)
        .into_iter()
        .map(|c| (c.id, c.name, true));
    let list = manual
        .chain(automatic)
        .map(|(id, name, automatic)| {
            let (game_count, covers) = collection_games(core, &id, None, COVERS as u32);
            let (installed_count, _) = collection_games(core, &id, Some(true), 0);
            ProfileCollection {
                id,
                name,
                automatic,
                game_count,
                installed_count,
                covers,
            }
        })
        .collect();
    let in_manual: std::collections::BTreeSet<&str> = core
        .collections
        .collections()
        .iter()
        .flat_map(|c| c.game_ids.iter().map(String::as_str))
        .filter(|id| core.library.find(id).is_some())
        .collect();
    ProfileCollections {
        collections: list,
        game_count: in_manual.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(day: NaiveDate, hour: u32, minutes: u64) -> PlaySession {
        let start = Local
            .from_local_datetime(&day.and_hms_opt(hour, 0, 0).unwrap())
            .unwrap();
        PlaySession {
            started_at: start.with_timezone(&chrono::Utc),
            duration: Duration::from_secs(minutes * 60),
        }
    }

    #[test]
    fn the_week_runs_monday_to_sunday_and_short_sessions_do_not_count() {
        // Quarta-feira, 14/10/2026.
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let s = [
            at(NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(), 20, 40), // segunda
            at(NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(), 9, 60),
            at(NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(), 21, 35),
            at(today, 10, 0), // abriu e fechou
            at(NaiveDate::from_ymd_opt(2026, 10, 11).unwrap(), 22, 90), // domingo anterior
            at(NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(), 10, 30), // próxima semana
        ];
        let days = week(&s, today);
        let minutes: Vec<u64> = days.iter().map(|d| d.seconds / 60).collect();
        assert_eq!(minutes, [40, 95, 0, 0, 0, 0, 0]);
        assert_eq!(days[0].day, "2026-10-12");
        assert_eq!(days[6].day, "2026-10-18");
        // Domingo é o último dia da mesma semana.
        let sunday = NaiveDate::from_ymd_opt(2026, 10, 18).unwrap();
        assert_eq!(week(&s, sunday)[0].seconds, 40 * 60);
        assert_eq!(week(&[], today).len(), 7);
    }
}
