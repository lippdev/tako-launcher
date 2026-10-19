//! Port de `Presentation/GameText.cs`: textos derivados do jogo (loja, última vez jogado, tempo, datas, estado).

use super::{tr, trn};
use crate::i18n::Lang;
use crate::library::{Game, GameStore};
use chrono::{DateTime, NaiveDate, Utc};
use std::time::Duration;

/// Nome da loja como o usuário vê. Só os adicionados mudam com o idioma.
pub fn store_name(lang: Lang, store: GameStore) -> String {
    match store {
        GameStore::Steam => "Steam".into(),
        GameStore::Epic => "Epic Games".into(),
        GameStore::Gog => "GOG".into(),
        GameStore::Xbox => "Xbox".into(),
        GameStore::Ubisoft => "Ubisoft Connect".into(),
        GameStore::Ea => "EA app".into(),
        GameStore::BattleNet => "Battle.net".into(),
        GameStore::Amazon => "Amazon Games".into(),
        GameStore::Heroic => "Heroic".into(),
        GameStore::Lutris => "Lutris".into(),
        GameStore::Bottles => "Bottles".into(),
        GameStore::Itch => "itch.io".into(),
        GameStore::Emulators => tr(lang, "presentation.store.emulators", &[]),
        GameStore::Manual => tr(lang, "presentation.store.manual", &[]),
    }
}

/// Gênero do nome em português ("do GOG", "da Steam").
fn masculine(store: GameStore) -> bool {
    matches!(
        store,
        GameStore::Gog
            | GameStore::Xbox
            | GameStore::Ea
            | GameStore::BattleNet
            | GameStore::Heroic
            | GameStore::Lutris
            | GameStore::Bottles
            | GameStore::Itch
    )
}

fn gendered(lang: Lang, key: &str, name: &str, masculine: bool) -> String {
    let form = if masculine { "masculine" } else { "feminine" };
    tr(
        lang,
        &format!("presentation.{key}.{form}"),
        &[("store", name)],
    )
}

/// "da Steam", "do GOG" (para "jogos da Steam"); em inglês, "from Steam".
pub fn store_of(lang: Lang, store: GameStore) -> String {
    if store == GameStore::Emulators {
        return tr(lang, "presentation.store_of.emulators", &[]);
    }
    gendered(lang, "store_of", &store_name(lang, store), masculine(store))
}

/// "pela Steam", "pelo GOG" (para "Abrindo pelo EA app"); em inglês, "via Steam".
pub fn store_via(lang: Lang, store: GameStore) -> String {
    if store == GameStore::Emulators {
        return tr(lang, "presentation.store_via.emulators", &[]);
    }
    gendered(
        lang,
        "store_via",
        &store_name(lang, store),
        masculine(store),
    )
}

/// "na Steam", "no GOG" (para "Instalando na Steam…"); em inglês, "on Steam".
pub fn store_in(lang: Lang, store: GameStore) -> String {
    if store == GameStore::Emulators {
        return tr(lang, "presentation.store_in.emulators", &[]);
    }
    gendered(lang, "store_in", &store_name(lang, store), masculine(store))
}

/// "pelo GOG Galaxy" e afins: nome livre, masculino.
pub(crate) fn via_name(lang: Lang, name: &str) -> String {
    gendered(lang, "store_via", name, true)
}

/// "Agora mesmo", "Jogado ontem", "Há 3 dias", "Há 2 semanas"...
pub fn last_played(lang: Lang, last_played: Option<DateTime<Utc>>, now: DateTime<Utc>) -> String {
    let Some(at) = last_played else {
        return tr(lang, "presentation.last_played.never", &[]);
    };
    let days = (now - at).num_seconds().div_euclid(86_400);
    match days {
        ..1 => tr(lang, "presentation.last_played.now", &[]),
        1 => tr(lang, "presentation.last_played.yesterday", &[]),
        2..7 => trn(lang, "presentation.last_played.days", days, &[]),
        7..30 => trn(lang, "presentation.last_played.weeks", days / 7, &[]),
        30..60 => trn(lang, "presentation.last_played.months", 1, &[]),
        _ => trn(lang, "presentation.last_played.months", days / 30, &[]),
    }
}

/// "—", "25 min", "14 h".
pub fn play_time(lang: Lang, play_time: Duration) -> String {
    let minutes = play_time.as_secs() / 60;
    if play_time.is_zero() {
        "—".into()
    } else if minutes < 60 {
        tr(
            lang,
            "presentation.play_time.minutes",
            &[("n", &minutes.max(1).to_string())],
        )
    } else {
        tr(
            lang,
            "presentation.play_time.hours",
            &[("n", &(minutes / 60).to_string())],
        )
    }
}

/// Duração de uma sessão no aviso de retorno: "menos de 1 min jogado", "25 min jogados", "1 h 05 min jogados".
pub fn session_played(lang: Lang, played: Duration) -> String {
    let minutes = (played.as_secs() / 60) as i64;
    match minutes {
        ..1 => tr(lang, "presentation.session.less_than_minute", &[]),
        1..60 => trn(lang, "presentation.session.minutes", minutes, &[]),
        _ if minutes % 60 == 0 => tr(
            lang,
            "presentation.session.hours",
            &[("h", &(minutes / 60).to_string())],
        ),
        _ => tr(
            lang,
            "presentation.session.hours_minutes",
            &[
                ("h", &(minutes / 60).to_string()),
                ("m", &format!("{:02}", minutes % 60)),
            ],
        ),
    }
}

/// Data no formato do idioma ("29/10/2026", "10/29/2026").
pub fn date(lang: Lang, date: NaiveDate) -> String {
    date.format(&tr(lang, "presentation.date.long", &[]))
        .to_string()
}

/// Dia e mês ("29/10", "10/29").
pub fn short_date(lang: Lang, date: NaiveDate) -> String {
    date.format(&tr(lang, "presentation.date.short", &[]))
        .to_string()
}

pub fn genres(game: &Game) -> String {
    game.genres.join(" · ")
}

/// "Instalado", "Não instalado", "Lança em 29/10/2026", "ROM no disco"...
pub fn status(lang: Lang, game: &Game) -> String {
    if let Some(up) = game.upcoming_release {
        tr(
            lang,
            "presentation.status.upcoming",
            &[("date", &date(lang, up))],
        )
    } else if game.is_rom() {
        tr(
            lang,
            if game.is_installed {
                "presentation.status.rom"
            } else {
                "presentation.status.rom_missing"
            },
            &[],
        )
    } else {
        tr(
            lang,
            if game.is_installed {
                "presentation.status.installed"
            } else {
                "presentation.status.not_installed"
            },
            &[],
        )
    }
}

/// "Corrigir correspondência": "Hollow Knight (2017) · SNES · IGDB", com "✓" quando casou pelo id da loja.
pub fn match_label(
    title: &str,
    year: Option<i32>,
    platforms: Option<&str>,
    is_exact: bool,
    source: &str,
) -> String {
    let title = year.map_or_else(|| title.to_owned(), |y| format!("{title} ({y})"));
    let platforms = platforms
        .filter(|p| !p.is_empty() && *p != "PC")
        .map(|p| format!(" · {p}"))
        .unwrap_or_default();
    format!(
        "{}{title}{platforms} · {source}",
        if is_exact { "✓ " } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::sample::now;
    use chrono::TimeDelta;

    #[test]
    fn last_played_uses_relative_text() {
        let at = |days: f64| Some(now() - TimeDelta::seconds((days * 86_400.0) as i64));
        for (days, pt, en) in [
            (0.1, "Agora mesmo", "Just now"),
            (1.2, "Jogado ontem", "Played yesterday"),
            (3.5, "Há 3 dias", "3 days ago"),
            (9.0, "Há 1 semana", "1 week ago"),
            (21.0, "Há 3 semanas", "3 weeks ago"),
            (45.0, "Há 1 mês", "1 month ago"),
            (65.0, "Há 2 meses", "2 months ago"),
        ] {
            assert_eq!(last_played(Lang::PtBr, at(days), now()), pt);
            assert_eq!(last_played(Lang::EnUs, at(days), now()), en);
        }
        assert_eq!(last_played(Lang::PtBr, None, now()), "Nunca jogado");
        assert_eq!(last_played(Lang::EnUs, None, now()), "Never played");
    }

    #[test]
    fn dates_use_the_language_format() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        assert_eq!(date(Lang::PtBr, d), "29/10/2026");
        assert_eq!(date(Lang::EnUs, d), "10/29/2026");
        assert_eq!(short_date(Lang::PtBr, d), "29/10");
        assert_eq!(short_date(Lang::EnUs, d), "10/29");
    }

    #[test]
    fn session_length_reads_naturally() {
        for (minutes, pt, en) in [
            (0.5, "menos de 1 min jogado", "less than 1 min played"),
            (1.9, "1 min jogado", "1 min played"),
            (25.0, "25 min jogados", "25 min played"),
            (60.0, "1 h jogadas", "1 h played"),
            (65.0, "1 h 05 min jogados", "1 h 05 min played"),
            (150.0, "2 h 30 min jogados", "2 h 30 min played"),
        ] {
            let d = Duration::from_secs_f64(minutes * 60.0);
            assert_eq!(session_played(Lang::PtBr, d), pt);
            assert_eq!(session_played(Lang::EnUs, d), en);
        }
    }

    #[test]
    fn play_time_and_store_phrases() {
        assert_eq!(play_time(Lang::PtBr, Duration::ZERO), "—");
        assert_eq!(play_time(Lang::PtBr, Duration::from_secs(20)), "1 min");
        assert_eq!(
            play_time(Lang::EnUs, Duration::from_secs(14 * 3600 + 59)),
            "14 h"
        );
        assert_eq!(store_of(Lang::PtBr, GameStore::Steam), "da Steam");
        assert_eq!(store_of(Lang::PtBr, GameStore::Gog), "do GOG");
        assert_eq!(store_of(Lang::PtBr, GameStore::Bottles), "do Bottles");
        assert_eq!(store_of(Lang::PtBr, GameStore::Itch), "do itch.io");
        assert_eq!(store_via(Lang::PtBr, GameStore::Ea), "pelo EA app");
        assert_eq!(store_via(Lang::EnUs, GameStore::Ea), "via EA app");
        assert_eq!(store_in(Lang::PtBr, GameStore::Steam), "na Steam");
        assert_eq!(store_in(Lang::PtBr, GameStore::Gog), "no GOG");
        assert_eq!(store_in(Lang::EnUs, GameStore::Steam), "on Steam");
        assert_eq!(store_name(Lang::EnUs, GameStore::Manual), "Added");
        assert_eq!(store_name(Lang::EnUs, GameStore::Bottles), "Bottles");
        assert_eq!(store_name(Lang::EnUs, GameStore::Itch), "itch.io");
    }

    #[test]
    fn status_and_match_label() {
        let mut g = Game {
            is_installed: true,
            ..Game::default()
        };
        assert_eq!(status(Lang::PtBr, &g), "Instalado");
        g.rom_path = Some("/roms/a.sfc".into());
        assert_eq!(status(Lang::EnUs, &g), "ROM on disk");
        g.upcoming_release = NaiveDate::from_ymd_opt(2026, 10, 29);
        assert_eq!(status(Lang::PtBr, &g), "Lança em 29/10/2026");
        assert_eq!(
            match_label("Hollow Knight", Some(2017), Some("SNES"), true, "IGDB"),
            "✓ Hollow Knight (2017) · SNES · IGDB"
        );
        assert_eq!(
            match_label("Hades", None, Some("PC"), false, "Steam"),
            "Hades · Steam"
        );
    }
}
