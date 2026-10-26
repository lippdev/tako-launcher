//! Port de `SteamStoreProvider.cs`: API pública da loja Steam (sem chave). Pede os textos no idioma
//! da interface. A Steam limita ~200 consultas a cada 5 minutos: o worker espaça as consultas.

use super::http::{Http, HttpRequest, escape, parse_json};
use super::{GameMetadata, MetadataLookup, MetadataMatch, MetadataProvider, external_ids, sources};
use crate::i18n::{self, Lang};
use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

pub struct SteamStoreProvider {
    http: Arc<dyn Http>,
}

impl SteamStoreProvider {
    pub fn new(http: Arc<dyn Http>) -> Self {
        SteamStoreProvider { http }
    }

    fn get_json(&self, url: &str) -> Option<Value> {
        let r = self.http.send(&HttpRequest::get(url)).ok()?;
        // 429 (limite) ou indisponível: tenta de novo numa próxima atualização.
        r.is_success().then(|| parse_json(&r.body)).flatten()
    }
}

/// Parâmetros de idioma e país da loja.
fn store_locale(lang: Lang) -> (&'static str, &'static str) {
    match lang {
        Lang::PtBr => ("brazilian", "BR"),
        Lang::EnUs => ("english", "US"),
    }
}

impl MetadataProvider for SteamStoreProvider {
    fn name(&self) -> &str {
        sources::STEAM
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn search(&self, lookup: &MetadataLookup) -> Vec<MetadataMatch> {
        if lookup
            .platform
            .as_deref()
            .is_some_and(|p| !p.is_empty() && !p.eq_ignore_ascii_case("pc"))
        {
            return Vec::new(); // a loja Steam só tem jogos de PC
        }
        let (l, cc) = store_locale(i18n::lang());
        let url = format!(
            "https://store.steampowered.com/api/storesearch/?term={}&l={l}&cc={cc}",
            escape(&lookup.title)
        );
        self.get_json(&url)
            .map(|doc| parse_search(&doc))
            .unwrap_or_default()
    }

    /// O appid já é o id da Steam.
    fn find_exact(&self, lookup: &MetadataLookup) -> Option<String> {
        lookup.external_ids.get(external_ids::STEAM).cloned()
    }

    fn fetch(&self, id: &str) -> Option<GameMetadata> {
        let app: u32 = id.parse().ok()?;
        let lang = i18n::lang();
        let (l, cc) = store_locale(lang);
        let url =
            format!("https://store.steampowered.com/api/appdetails?appids={app}&l={l}&cc={cc}");
        parse(&self.get_json(&url)?, app, Utc::now(), lang)
    }
}

/// Resultado de `storesearch`: só apps.
pub fn parse_search(doc: &Value) -> Vec<MetadataMatch> {
    let Some(items) = doc.get("items").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter(|i| i.get("type").and_then(Value::as_str) == Some("app"))
        .filter_map(|i| {
            let id = u32::try_from(i.get("id")?.as_u64()?).ok()?;
            let name = i.get("name").and_then(Value::as_str).unwrap_or_default();
            let mut m = MetadataMatch::steam(name, id);
            m.cover_url = i
                .get("tiny_image")
                .and_then(Value::as_str)
                .filter(|u| u.starts_with("https://"))
                .map(str::to_owned);
            Some(m)
        })
        .collect()
}

/// Resposta de `appdetails` → metadados. `lang` é o idioma pedido à loja.
pub fn parse(root: &Value, app_id: u32, now: DateTime<Utc>, lang: Lang) -> Option<GameMetadata> {
    let entry = root.get(app_id.to_string())?;
    if entry.get("success").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let data = entry.get("data")?;
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    let first = |k: &str| {
        data.get(k)
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    // A loja só traduz a descrição de jogos com o idioma entre os suportados.
    let language = match lang {
        Lang::PtBr
            if text(data, "supported_languages")
                .is_some_and(|s| s.to_lowercase().contains("português")) =>
        {
            "pt-BR"
        }
        _ => "en",
    };
    Some(GameMetadata {
        fetched_at: Some(now),
        steam_app_id: Some(app_id),
        source_ids: [(sources::STEAM.to_owned(), app_id.to_string())].into(),
        title: text(data, "name"),
        description: text(data, "short_description").map(|d| html_decode(&d).trim().to_owned()),
        description_language: Some(language.into()),
        genres: data
            .get("genres")
            .and_then(Value::as_array)
            .map(|g| {
                g.iter()
                    .filter_map(|x| text(x, "description"))
                    .take(3)
                    .collect()
            })
            .unwrap_or_default(),
        release_date: data
            .get("release_date")
            .and_then(|r| text(r, "date"))
            .and_then(|d| parse_date(&d)),
        developer: first("developers"),
        publisher: first("publishers"),
        metacritic_score: data
            .get("metacritic")
            .and_then(|m| m.get("score"))
            .and_then(Value::as_i64)
            .and_then(|s| i32::try_from(s).ok()),
        ..GameMetadata::new(sources::STEAM)
    })
}

/// Datas da loja: "4/set./2025", "24 de out. de 2026" (português) ou "4 Sep, 2025" / "Sep 4, 2025".
pub fn parse_date(text: &str) -> Option<NaiveDate> {
    const PT: [&str; 12] = [
        "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
    ];
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*(?P<day>\d{1,2})\s*(?:/|de)\s*(?P<month>[a-zç]{3})[a-zç]*\.?\s*(?:/|de)\s*(?P<year>\d{4})").ok()
    });
    if let Some(c) = re.as_ref().and_then(|r| r.captures(text)) {
        let month = c["month"].to_lowercase();
        if let Some(m) = PT.iter().position(|p| *p == month) {
            return NaiveDate::from_ymd_opt(
                c["year"].parse().ok()?,
                m as u32 + 1,
                c["day"].parse().ok()?,
            );
        }
    }
    ["%d %b, %Y", "%b %d, %Y", "%d %B, %Y", "%B %d, %Y"]
        .iter()
        .find_map(|f| NaiveDate::parse_from_str(text, f).ok())
}

/// Entidades HTML comuns da loja (`&quot;`, `&amp;`, `&#39;`...).
pub fn html_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let semicolon = rest.char_indices().take(12).find(|(_, c)| *c == ';');
        let Some((end, _)) = semicolon else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "quot" => Some('"'),
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::test_support::{FakeHttp, now};

    #[test]
    fn parses_store_dates() {
        for (text, y, m, d) in [
            ("4/set./2025", 2025, 9, 4),
            ("24 de out. de 2026", 2026, 10, 24),
            ("9 Jul, 2013", 2013, 7, 9),
            ("Jul 9, 2013", 2013, 7, 9),
        ] {
            assert_eq!(parse_date(text), NaiveDate::from_ymd_opt(y, m, d), "{text}");
        }
        for text in ["Em breve", "", "99/abc/2020"] {
            assert_eq!(parse_date(text), None);
        }
    }

    #[test]
    fn parses_appdetails_in_portuguese() {
        let json: Value = serde_json::from_str(
            r#"{"1030300":{"success":true,"data":{"name":"Hollow Knight: Silksong",
              "short_description":"Descubra um reino &quot;vasto&quot;",
              "supported_languages":"Inglês, Português - Brasil",
              "genres":[{"id":"1","description":"Ação"},{"id":"25","description":"Aventura"}],
              "release_date":{"coming_soon":false,"date":"4/set./2025"},
              "developers":["Team Cherry"],"publishers":["Team Cherry"],
              "metacritic":{"score":90,"url":"https://www.metacritic.com/"}}}}"#,
        )
        .unwrap();
        let m = parse(&json, 1030300, now(), Lang::PtBr).unwrap();
        assert_eq!(
            m.description.as_deref(),
            Some("Descubra um reino \"vasto\"")
        );
        assert_eq!(m.description_language.as_deref(), Some("pt-BR"));
        assert_eq!(m.genres, ["Ação", "Aventura"]);
        assert_eq!(m.release_date, NaiveDate::from_ymd_opt(2025, 9, 4));
        assert_eq!(m.developer.as_deref(), Some("Team Cherry"));
        assert_eq!(m.metacritic_score, Some(90));
        assert_eq!(
            parse(&json, 1030300, now(), Lang::EnUs)
                .unwrap()
                .description_language
                .as_deref(),
            Some("en")
        );
        let failed: Value = serde_json::from_str(r#"{"1":{"success":false}}"#).unwrap();
        assert!(parse(&failed, 1, now(), Lang::PtBr).is_none());
        // Tipos errados não derrubam nada.
        let odd: Value = serde_json::from_str(
            r#"{"1":{"success":true,"data":{"name":5,"genres":"x","metacritic":{"score":"90"}}}}"#,
        )
        .unwrap();
        assert_eq!(parse(&odd, 1, now(), Lang::PtBr).unwrap().title, None);
    }

    #[test]
    fn search_keeps_only_apps() {
        let http = Arc::new(FakeHttp::new());
        http.push(
            200,
            r#"{"total":2,"items":[{"type":"app","name":"Hollow Knight: Silksong","id":1030300},{"type":"sub","name":"Pacote","id":5}]}"#,
        );
        let r = SteamStoreProvider::new(http).search(&MetadataLookup::new("silksong"));
        assert_eq!(
            r,
            [MetadataMatch::steam("Hollow Knight: Silksong", 1030300)]
        );
    }

    #[test]
    fn rate_limited_and_broken_requests_return_nothing() {
        let http = Arc::new(FakeHttp::with_fallback(429, "{}"));
        assert!(SteamStoreProvider::new(http).fetch("570").is_none());
        let http = Arc::new(FakeHttp::with_fallback(200, "<html>"));
        let p = SteamStoreProvider::new(http);
        assert!(p.fetch("570").is_none());
        assert!(p.search(&MetadataLookup::new("x")).is_empty());
        assert!(p.fetch("abc").is_none());
    }

    #[test]
    fn decodes_html_entities() {
        assert_eq!(
            html_decode("a &amp; b &#39;c&#x27; &bogus; &"),
            "a & b 'c' &bogus; &"
        );
    }
}
