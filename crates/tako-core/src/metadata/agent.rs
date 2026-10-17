//! Port de `MetadataAgent.cs`: para cada jogo, consulta as fontes ligadas na ordem do tipo de jogo,
//! casa primeiro por ids e só então pelo título, e combina campo a campo. Não guarda nada.

use super::art::SourceIdParse;
use super::http::Clock;
use super::merge::{self, MetadataKind, applies, kind_of, sort_by_priority};
use super::{GameMetadata, MetadataLookup, MetadataMatch, MetadataProvider, title_matcher};
use crate::i18n::{self, Lang};
use crate::library::Game;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Cache velho: consulta de novo depois de 30 dias (7 quando não houve correspondência).
pub const MAX_AGE_DAYS: i64 = 30;
pub const NO_MATCH_RETRY_DAYS: i64 = 7;

/// Resultado de uma atualização: os metadados novos (`None` = nada a gravar) e se consultou alguém.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataUpdate {
    pub metadata: Option<GameMetadata>,
    pub queried: bool,
}

/// Ids fixados pelo usuário por fonte ("Fontes e IDs"); vazio = "nenhum".
pub type PinnedIds = BTreeMap<String, String>;

pub struct MetadataAgent {
    providers: Vec<Arc<dyn MetadataProvider>>,
    clock: Arc<dyn Clock>,
    /// Idioma preferido na descrição; `None` = o da interface.
    pub lang: Option<Lang>,
}

impl MetadataAgent {
    pub fn new(providers: Vec<Arc<dyn MetadataProvider>>, clock: Arc<dyn Clock>) -> Self {
        MetadataAgent {
            providers,
            clock,
            lang: None,
        }
    }

    /// Provedores ligados agora (a IGDB some quando não há chave).
    pub fn enabled(&self) -> Vec<Arc<dyn MetadataProvider>> {
        self.providers
            .iter()
            .filter(|p| p.is_enabled())
            .cloned()
            .collect()
    }

    /// "Steam" ou "Steam e IGDB", para os textos da interface.
    pub fn enabled_names(&self) -> String {
        let names: Vec<String> = self.enabled().iter().map(|p| p.name().to_owned()).collect();
        match names.as_slice() {
            [] => i18n::t("metadata.no_source"),
            [one] => one.clone(),
            [rest @ .., last] => i18n::tf(
                "metadata.source_list",
                &[("list", &rest.join(", ")), ("last", last)],
            ),
        }
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.providers
            .iter()
            .any(|p| p.name() == name && p.is_enabled())
    }

    /// Cache gravado antes da combinação (só Steam) vira uma combinação de uma parte só.
    pub fn normalize(cached: Option<GameMetadata>) -> Option<GameMetadata> {
        let mut c = cached?;
        if c.parts.is_empty() && c.consulted.is_empty() {
            c.consulted = vec![c.source.clone()];
            if !c.is_empty() {
                let mut part = c.clone();
                part.consulted.clear();
                c.parts = vec![part];
            }
        }
        Some(c)
    }

    /// Precisa consultar: sem cache, cache velho, fonte ligada ainda não consultada ou id fixado mudou.
    pub fn needs_update(
        &self,
        game: &Game,
        cached: Option<&GameMetadata>,
        force: bool,
        pinned: &PinnedIds,
    ) -> bool {
        let Some(cached) = Self::normalize(cached.cloned()) else {
            return true;
        };
        let sources = self.for_kind(kind_of(game));
        force
            || self.is_stale(&cached)
            || sources
                .iter()
                .any(|p| !p.is_paused() && !cached.consulted.iter().any(|c| c == p.name()))
            || pin_changed(&cached, pinned, &sources)
    }

    /// Fontes ligadas que atendem o jogo ("Fontes e IDs"), na ordem do tipo; inclui as de artes.
    pub fn sources(&self, game: &Game) -> Vec<Arc<dyn MetadataProvider>> {
        let kind = kind_of(game);
        let mut v: Vec<_> = self
            .enabled()
            .into_iter()
            .filter(|p| applies(p.name(), kind))
            .collect();
        sort_by_priority(&mut v, |p| p.name(), &merge::order(kind));
        v
    }

    /// Busca o termo numa fonte só ("Fontes e IDs"); sem resultados, tenta o termo limpo.
    pub fn search_source(&self, game: &Game, source: &str, term: &str) -> Vec<MetadataMatch> {
        let Some(p) = self.sources(game).into_iter().find(|p| p.name() == source) else {
            return Vec::new();
        };
        if p.is_paused() || term.trim().is_empty() {
            return Vec::new();
        }
        let lookup = MetadataLookup {
            title: term.trim().into(),
            ..MetadataLookup::for_game(game)
        };
        search_provider(p.as_ref(), &lookup)
    }

    /// Id digitado ou colado → id na fonte e os dados que ele representa (para mostrar o nome antes).
    pub fn validate(&self, source: &str, input: &SourceIdParse) -> Option<(String, GameMetadata)> {
        if !input.is_valid() {
            return None;
        }
        let p = self
            .providers
            .iter()
            .find(|p| p.name() == source && p.is_enabled())?;
        let id = input
            .id
            .clone()
            .or_else(|| input.slug.as_deref().and_then(|s| p.id_from_slug(s)))?;
        let m = p.fetch(&id)?;
        Some((id, m))
    }

    /// Lote antes da rodada: provedores que resolvem muitos jogos de uma vez (IGDB).
    pub fn prefetch(&self, games: &[Game]) {
        let lookups: Vec<MetadataLookup> = games
            .iter()
            .map(MetadataLookup::for_game)
            .filter(|l| !l.external_ids.is_empty())
            .collect();
        if lookups.is_empty() {
            return;
        }
        for p in self.enabled() {
            p.prefetch(&lookups);
        }
    }

    /// Consulta as fontes que faltam. `pinned` vale mais que os ids da loja e os aprendidos; a fonte
    /// fixada não é buscada pelo nome e id vazio tira a fonte deste jogo.
    pub fn update(
        &self,
        game: &Game,
        cached: Option<&GameMetadata>,
        force: bool,
        pinned: &PinnedIds,
    ) -> MetadataUpdate {
        let cached = Self::normalize(cached.cloned());
        if !self.needs_update(game, cached.as_ref(), force, pinned) {
            return MetadataUpdate {
                metadata: None,
                queried: false,
            };
        }
        let kind = kind_of(game);
        let enabled = self.for_kind(kind);
        let legacy_pinned = cached.as_ref().is_some_and(|c| c.is_pinned);
        let refresh_all = match &cached {
            None => true,
            Some(c) => force || self.is_stale(c) || pin_changed(c, pinned, &enabled),
        };
        let has = |name: &str| enabled.iter().any(|e| e.name() == name);
        let (keep, mut consulted): (Vec<GameMetadata>, Vec<String>) = match (&cached, refresh_all) {
            (Some(c), false) => (
                c.parts.iter().filter(|p| has(&p.source)).cloned().collect(),
                c.consulted.iter().filter(|n| has(n)).cloned().collect(),
            ),
            _ => (Vec::new(), Vec::new()),
        };
        let mut ids = super::external_ids::for_game(game);
        if legacy_pinned && let Some(c) = &cached {
            ids.extend(c.source_ids.clone()); // escolha do usuário vale mais que o appid da loja
        }
        for (s, id) in pinned.iter().filter(|(_, v)| !v.is_empty()) {
            ids.insert(s.clone(), id.clone()); // fixado em "Fontes e IDs" vale mais que todos
        }
        for part in &keep {
            learn(&mut ids, part);
        }

        let mut parts = keep;
        let before = consulted.len();
        let mut queried = false;
        for p in &enabled {
            if consulted.iter().any(|c| c == p.name()) || p.is_paused() {
                continue;
            }
            let none = pinned.get(p.name()).is_some_and(String::is_empty);
            if none
                || (p.art_fallback().is_some()
                    && full_art(&parts)
                    && !pinned.contains_key(p.name()))
            {
                // "Nenhum" escolhido, ou fonte de artes para um jogo que já tem os quatro tipos.
                consulted.push(p.name().to_owned());
                continue;
            }
            queried = true;
            let title_search = !legacy_pinned && !pinned.contains_key(p.name());
            let (metadata, transient) = consult(p.as_ref(), game, &ids, title_search);
            if transient || p.is_paused() {
                // Limite da API, fora do ar ou pausado durante a consulta: fica para a próxima rodada.
                continue;
            }
            consulted.push(p.name().to_owned());
            if let Some(m) = metadata {
                learn(&mut ids, &m);
                parts.push(m);
            }
        }
        if consulted.len() == before {
            return MetadataUpdate {
                metadata: None,
                queried,
            };
        }
        // Só parte das fontes consultada agora: o prazo continua o da rodada completa anterior.
        let fetched_at = match (&cached, refresh_all) {
            (Some(c), false) => c.fetched_at.unwrap_or_else(|| self.clock.now()),
            _ => self.clock.now(),
        };
        let lang = self.lang.unwrap_or_else(i18n::lang);
        let mut combined = merge::combine(&parts, kind, fetched_at, Some(&consulted), lang);
        combined.is_pinned = legacy_pinned;
        for (s, id) in pinned {
            if id.is_empty() {
                combined.source_ids.remove(s);
            } else {
                combined.source_ids.insert(s.clone(), id.clone());
            }
        }
        MetadataUpdate {
            metadata: Some(combined),
            queried,
        }
    }

    /// "Corrigir correspondência": resultados de todas as fontes ligadas, na ordem do tipo.
    pub fn search(&self, game: &Game) -> Vec<MetadataMatch> {
        let lookup = MetadataLookup::for_game(game);
        self.for_kind(kind_of(game))
            .iter()
            .filter(|p| !p.is_paused() && p.art_fallback().is_none())
            .flat_map(|p| search_provider(p.as_ref(), &lookup))
            .collect()
    }

    /// Fontes ligadas que atendem o tipo, na ordem dele; as de artes só com "usar quando faltar capa".
    fn for_kind(&self, kind: MetadataKind) -> Vec<Arc<dyn MetadataProvider>> {
        let mut v: Vec<_> = self
            .enabled()
            .into_iter()
            .filter(|p| applies(p.name(), kind) && p.art_fallback() != Some(false))
            .collect();
        sort_by_priority(&mut v, |p| p.name(), &merge::order(kind));
        v
    }

    fn is_stale(&self, cached: &GameMetadata) -> bool {
        let days = if cached.is_empty() {
            NO_MATCH_RETRY_DAYS
        } else {
            MAX_AGE_DAYS
        };
        cached
            .fetched_at
            .is_none_or(|at| self.clock.now() - at >= chrono::Duration::days(days))
    }
}

/// Algum id fixado (de uma fonte consultada) difere do cache, ou "nenhum" com dados no cache.
fn pin_changed(
    cached: &GameMetadata,
    pinned: &PinnedIds,
    sources: &[Arc<dyn MetadataProvider>],
) -> bool {
    pinned.iter().any(|(source, id)| {
        let relevant = sources
            .iter()
            .any(|p| p.name() == source && p.art_fallback().is_none());
        let part = cached.parts.iter().find(|p| &p.source == source);
        relevant
            && if id.is_empty() {
                part.is_some()
            } else {
                part.and_then(|p| p.id_in(source)).as_ref() != Some(id)
            }
    })
}

fn learn(ids: &mut BTreeMap<String, String>, m: &GameMetadata) {
    for (s, id) in &m.source_ids {
        ids.entry(s.clone()).or_insert_with(|| id.clone());
    }
}

/// As fontes já trazem capa, fundo, logo e ícone (ART-001)? A Steam traz os quatro pelo appid; sem isso, a fonte de
/// artes (SteamGridDB) completa o que faltar.
fn full_art(parts: &[GameMetadata]) -> bool {
    use crate::library::media::{ICON, LOGO};
    let any = |f: &dyn Fn(&GameMetadata) -> bool| parts.iter().any(f);
    any(&|x| x.steam_app_id.is_some())
        || (any(&|x| x.cover_url.is_some())
            && any(&|x| x.background_url.is_some())
            && any(&|x| x.media.contains_key(LOGO))
            && any(&|x| x.media.contains_key(ICON)))
}

/// (metadados, falha passageira).
fn consult(
    p: &dyn MetadataProvider,
    game: &Game,
    ids: &BTreeMap<String, String>,
    title_search: bool,
) -> (Option<GameMetadata>, bool) {
    let lookup = MetadataLookup {
        external_ids: ids.clone(),
        ..MetadataLookup::for_game(game)
    };
    let mut id = ids.get(p.name()).cloned().or_else(|| p.find_exact(&lookup));
    let mut by_title = false;
    if id.is_none() && title_search {
        let results = search_provider(p, &lookup);
        let exact = results.iter().find(|r| r.is_exact);
        by_title = exact.is_none();
        id = exact
            .or_else(|| title_matcher::best(&game.title, &results))
            .map(|r| r.id.clone());
    }
    let Some(id) = id else {
        return (None, false); // sem correspondência nesta fonte
    };
    let m = p.fetch(&id).map(|m| GameMetadata {
        matched_by_title: by_title,
        ..m
    });
    let transient = m.is_none();
    (m, transient)
}

/// Busca pelo título; sem resultados, tenta o título limpo (a loja não acha "… GOTY").
fn search_provider(p: &dyn MetadataProvider, lookup: &MetadataLookup) -> Vec<MetadataMatch> {
    for term in title_matcher::search_terms(&lookup.title) {
        let results = p.search(&MetadataLookup {
            title: term,
            ..lookup.clone()
        });
        if !results.is_empty() {
            return results;
        }
    }
    Vec::new()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::library::GameStore;
    use crate::metadata::merge::{MetadataKind, combine};
    use crate::metadata::test_support::{TestClock, now};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    type Fetch = Box<dyn Fn(&str) -> Option<GameMetadata> + Send + Sync>;

    pub struct FakeProvider {
        pub name: String,
        pub fetch: Fetch,
        pub exact_key: Option<String>,
        pub enabled: AtomicBool,
        pub results: Vec<MetadataMatch>,
        pub searches: AtomicUsize,
        pub fetched: Mutex<Vec<String>>,
        pub art: Option<AtomicBool>,
        pub exact_fixed: Option<String>,
        pub slug: Option<(String, String)>,
    }

    impl FakeProvider {
        pub fn new(name: &str, result: Option<GameMetadata>) -> Self {
            Self::with(name, Box::new(move |_| result.clone()))
        }
        pub fn with(name: &str, fetch: Fetch) -> Self {
            FakeProvider {
                name: name.into(),
                fetch,
                exact_key: None,
                enabled: AtomicBool::new(true),
                results: Vec::new(),
                searches: AtomicUsize::new(0),
                fetched: Mutex::new(Vec::new()),
                art: None,
                exact_fixed: None,
                slug: None,
            }
        }
        pub fn exact(mut self, key: &str) -> Self {
            self.exact_key = Some(key.into());
            self
        }
        pub fn results(mut self, r: Vec<MetadataMatch>) -> Self {
            self.results = r;
            self
        }
        pub fn searches(&self) -> usize {
            self.searches.load(Ordering::SeqCst)
        }
        pub fn fetched(&self) -> Vec<String> {
            self.fetched.lock().unwrap().clone()
        }
    }

    impl MetadataProvider for FakeProvider {
        fn name(&self) -> &str {
            &self.name
        }
        fn is_enabled(&self) -> bool {
            self.enabled.load(Ordering::SeqCst)
        }
        fn art_fallback(&self) -> Option<bool> {
            self.art.as_ref().map(|a| a.load(Ordering::SeqCst))
        }
        fn search(&self, _: &MetadataLookup) -> Vec<MetadataMatch> {
            self.searches.fetch_add(1, Ordering::SeqCst);
            self.results.clone()
        }
        fn find_exact(&self, lookup: &MetadataLookup) -> Option<String> {
            if self.exact_fixed.is_some() {
                return self.exact_fixed.clone();
            }
            let key = self.exact_key.as_ref()?;
            lookup
                .external_ids
                .get(key)
                .cloned()
                .or_else(|| (key == "GOG").then(|| "gog-1".to_owned()))
        }
        fn fetch(&self, id: &str) -> Option<GameMetadata> {
            self.fetched.lock().unwrap().push(id.into());
            (self.fetch)(id)
        }
        fn id_from_slug(&self, slug: &str) -> Option<String> {
            self.slug
                .as_ref()
                .filter(|(s, _)| s == slug)
                .map(|(_, id)| id.clone())
        }
    }

    pub fn game(store: GameStore, title: &str, app: Option<u32>) -> Game {
        Game {
            id: format!("{}:1", store.id_prefix()),
            title: title.into(),
            store: Some(store),
            steam_app_id: app,
            ..Default::default()
        }
    }

    fn agent(providers: Vec<Arc<dyn MetadataProvider>>, clock: Arc<TestClock>) -> MetadataAgent {
        let mut a = MetadataAgent::new(providers, clock);
        a.lang = Some(Lang::PtBr);
        a
    }

    fn steam_part() -> GameMetadata {
        crate::metadata::merge::tests::steam_part()
    }
    fn igdb_part() -> GameMetadata {
        crate::metadata::merge::tests::igdb_part()
    }

    fn part(source: &str, id: &str) -> GameMetadata {
        GameMetadata {
            source_ids: [(source.to_owned(), id.to_owned())].into(),
            steam_app_id: (source == "Steam").then(|| id.parse().unwrap()),
            title: Some("Lanterna Distante".into()),
            cover_url: (source == "IGDB").then(|| "https://images.igdb.com/c.jpg".into()),
            ..GameMetadata::new(source)
        }
    }

    const NONE: &PinnedIds = &BTreeMap::new();

    #[test]
    fn epic_game_matches_igdb_by_title_and_steam_by_the_id_igdb_reports() {
        let steam = Arc::new(FakeProvider::new("Steam", Some(steam_part())).exact("Steam"));
        let igdb = Arc::new(FakeProvider::new("IGDB", Some(igdb_part())).results(vec![
            MetadataMatch::new("IGDB", "900001", "Lanterna Distante"),
        ]));
        let a = agent(vec![steam.clone(), igdb], Arc::new(TestClock::at(now())));
        let u = a.update(
            &game(GameStore::Epic, "Lanterna Distante™", None),
            None,
            false,
            NONE,
        );
        assert!(u.queried);
        let m = u.metadata.unwrap();
        assert_eq!(m.source, "IGDB + Steam");
        assert_eq!(m.consulted, ["IGDB", "Steam"]);
        assert_eq!(steam.searches(), 0); // casou pelo appid que a IGDB informou
        assert_eq!(steam.fetched(), ["4000001"]);
    }

    #[test]
    fn igdb_off_keeps_steam_only_and_turning_it_on_consults_only_igdb() {
        let steam = Arc::new(FakeProvider::new("Steam", Some(steam_part())).exact("Steam"));
        let igdb = Arc::new(FakeProvider::new("IGDB", Some(igdb_part())).exact("Steam"));
        igdb.enabled.store(false, Ordering::SeqCst);
        let clock = Arc::new(TestClock::at(now()));
        let a = agent(vec![steam.clone(), igdb.clone()], clock.clone());
        let g = game(GameStore::Steam, "Lanterna Distante", Some(4000001));

        let first = a.update(&g, None, false, NONE).metadata.unwrap();
        assert_eq!(first.source, "Steam");
        assert_eq!(a.enabled_names(), "Steam");
        assert!(!a.needs_update(&g, Some(&first), false, NONE));

        igdb.enabled.store(true, Ordering::SeqCst);
        clock.advance(chrono::Duration::days(1));
        assert!(a.needs_update(&g, Some(&first), false, NONE));
        let second = a.update(&g, Some(&first), false, NONE).metadata.unwrap();
        assert_eq!(second.source, "Steam + IGDB");
        assert_eq!(steam.fetched().len(), 1); // a parte da Steam foi reaproveitada
        assert_eq!(first.fetched_at, second.fetched_at);
        assert_eq!(
            a.enabled_names(),
            i18n::tf(
                "metadata.source_list",
                &[("list", "Steam"), ("last", "IGDB")]
            )
        );
    }

    #[test]
    fn transient_failures_are_retried_and_misses_wait_a_week() {
        let clock = Arc::new(TestClock::at(now()));
        let a = agent(
            vec![Arc::new(FakeProvider::new("Steam", None))],
            clock.clone(),
        );
        let g = game(GameStore::Manual, "Nada parecido", None);
        let miss = a.update(&g, None, false, NONE).metadata.unwrap();
        assert!(miss.is_empty());
        assert!(!a.needs_update(&g, Some(&miss), false, NONE));
        clock.advance(chrono::Duration::days(8));
        assert!(a.needs_update(&g, Some(&miss), false, NONE));

        let flaky = Arc::new(FakeProvider::new("Steam", None).exact("Steam"));
        let t = agent(vec![flaky], clock).update(
            &game(GameStore::Steam, "x", Some(1)),
            None,
            false,
            NONE,
        );
        assert!(t.queried);
        assert_eq!(t.metadata, None); // nada gravado: tenta de novo na próxima rodada
    }

    #[test]
    fn legacy_pinned_match_is_kept_on_refresh() {
        let steam = Arc::new(
            FakeProvider::new("Steam", Some(steam_part()))
                .exact("Steam")
                .results(vec![MetadataMatch::steam("Errado", 1)]),
        );
        let igdb = Arc::new(FakeProvider::new("IGDB", Some(igdb_part())));
        let clock = Arc::new(TestClock::at(now()));
        let a = agent(vec![steam.clone(), igdb.clone()], clock.clone());
        let g = game(GameStore::Manual, "lanterna.exe", None);
        let mut pinned = combine(
            &[igdb_part()],
            MetadataKind::Pc,
            now(),
            Some(&["IGDB".to_owned()]),
            Lang::PtBr,
        );
        pinned.is_pinned = true;
        clock.advance(chrono::Duration::days(40));
        let r = a.update(&g, Some(&pinned), false, NONE).metadata.unwrap();
        assert!(r.is_pinned);
        assert_eq!(r.id_in("IGDB").as_deref(), Some("900001"));
        assert_eq!(r.steam_app_id, Some(4000001));
        assert_eq!(steam.searches() + igdb.searches(), 0);
    }

    #[test]
    fn fix_match_lists_results_from_every_enabled_source_in_priority_order() {
        let steam = Arc::new(
            FakeProvider::new("Steam", Some(steam_part()))
                .results(vec![MetadataMatch::steam("Lanterna Distante", 4000001)]),
        );
        let igdb = Arc::new(FakeProvider::new("IGDB", Some(igdb_part())).results(vec![
            MetadataMatch::new("IGDB", "900001", "Lanterna Distante"),
        ]));
        let a = agent(vec![steam, igdb], Arc::new(TestClock::at(now())));
        let sources = |s| {
            a.search(&game(s, "Lanterna Distante", None))
                .into_iter()
                .map(|m| m.source)
                .collect::<Vec<_>>()
        };
        assert_eq!(sources(GameStore::Epic), ["IGDB", "Steam"]);
        assert_eq!(sources(GameStore::Steam), ["Steam", "IGDB"]);
    }

    #[test]
    fn legacy_steam_only_cache_becomes_a_one_part_combination() {
        let legacy = GameMetadata {
            steam_app_id: Some(570),
            fetched_at: Some(now()),
            description: Some("Antigo".into()),
            ..GameMetadata::new("Steam")
        };
        let n = MetadataAgent::normalize(Some(legacy)).unwrap();
        assert_eq!(n.consulted, ["Steam"]);
        assert_eq!(n.parts.len(), 1);
        assert_eq!(n.id_in("Steam").as_deref(), Some("570"));
        let empty = GameMetadata {
            fetched_at: Some(now()),
            ..GameMetadata::new("Steam")
        };
        assert_eq!(
            MetadataAgent::normalize(Some(empty)).unwrap().consulted,
            ["Steam"]
        );
    }

    #[test]
    fn pinned_id_wins_over_the_store_and_is_not_replaced_by_a_title_search() {
        let steam = Arc::new(
            FakeProvider::with("Steam", Box::new(|id| Some(part("Steam", id))))
                .exact("Steam")
                .results(vec![MetadataMatch::steam("Errado", 1)]),
        );
        let igdb = Arc::new(
            FakeProvider::with("IGDB", Box::new(|id| Some(part("IGDB", id))))
                .results(vec![MetadataMatch::new("IGDB", "111", "Outro")]),
        );
        let a = agent(vec![steam, igdb.clone()], Arc::new(TestClock::at(now())));
        let g = game(GameStore::Steam, "Lanterna Distante", Some(4000001));
        let pin = |v: &str| PinnedIds::from([("IGDB".to_owned(), v.to_owned())]);

        let first = a.update(&g, None, false, &pin("900001")).metadata.unwrap();
        assert_eq!(first.id_in("IGDB").as_deref(), Some("900001"));
        assert_eq!(igdb.searches(), 0);
        assert!(!a.needs_update(&g, Some(&first), false, &pin("900001")));

        assert!(a.needs_update(&g, Some(&first), false, &pin("900002")));
        let second = a
            .update(&g, Some(&first), false, &pin("900002"))
            .metadata
            .unwrap();
        assert_eq!(second.id_in("IGDB").as_deref(), Some("900002"));
        assert_eq!(second.id_in("Steam").as_deref(), Some("4000001"));

        assert!(a.needs_update(&g, Some(&second), false, &pin("")));
        let third = a
            .update(&g, Some(&second), false, &pin(""))
            .metadata
            .unwrap();
        assert_eq!(third.id_in("IGDB"), None);
        assert!(!third.parts.iter().any(|p| p.source == "IGDB"));
    }

    #[test]
    fn steamgriddb_completes_logo_and_icon_without_replacing_the_cover() {
        let igdb = Arc::new(
            FakeProvider::with("IGDB", Box::new(|id| Some(part("IGDB", id)))).results(vec![
                MetadataMatch::new("IGDB", "7", "Lanterna Distante Remaster"),
            ]),
        );
        let mut sgdb = FakeProvider::with(
            "SteamGridDB",
            Box::new(|id| {
                Some(GameMetadata {
                    source_ids: [("SteamGridDB".to_owned(), id.to_owned())].into(),
                    cover_url: Some("https://cdn2.steamgriddb.com/grid/a.png".into()),
                    media: [
                        (
                            "logo".to_owned(),
                            "https://cdn2.steamgriddb.com/logo/l.png".to_owned(),
                        ),
                        (
                            "icon".to_owned(),
                            "https://cdn2.steamgriddb.com/icon/i.png".to_owned(),
                        ),
                    ]
                    .into(),
                    ..GameMetadata::new("SteamGridDB")
                })
            }),
        );
        sgdb.art = Some(AtomicBool::new(true));
        sgdb.exact_fixed = Some("5249".into());
        let sgdb = Arc::new(sgdb);
        let a = agent(vec![igdb, sgdb.clone()], Arc::new(TestClock::at(now())));
        let g = Game {
            id: "manual:1".into(),
            title: "Lanterna Distante".into(),
            store: Some(GameStore::Manual),
            ..Default::default()
        };
        let m = a.update(&g, None, false, NONE).metadata.unwrap();
        assert_eq!(sgdb.fetched(), ["5249"]);
        assert_eq!(
            m.cover_url.as_deref(),
            Some("https://images.igdb.com/c.jpg")
        );
        assert_eq!(
            m.media.get("logo").map(String::as_str),
            Some("https://cdn2.steamgriddb.com/logo/l.png")
        );
        assert!(m.media.contains_key("icon"));
        // A IGDB achou pelo nome (sem id da loja): a revisão de capas mostra para conferir.
        let igdb_part = m.parts.iter().find(|p| p.source == "IGDB").unwrap();
        assert!(igdb_part.matched_by_title);
        assert!(
            !m.parts
                .iter()
                .find(|p| p.source == "SteamGridDB")
                .unwrap()
                .matched_by_title
        );
    }

    #[test]
    fn steamgriddb_only_fills_games_left_without_cover_and_only_when_turned_on() {
        let igdb = Arc::new(
            FakeProvider::with(
                "IGDB",
                Box::new(|id| {
                    Some(GameMetadata {
                        cover_url: None,
                        ..part("IGDB", id)
                    })
                }),
            )
            .exact("GOG"),
        );
        let mut sgdb = FakeProvider::with(
            "SteamGridDB",
            Box::new(|id| {
                Some(GameMetadata {
                    source_ids: [("SteamGridDB".to_owned(), id.to_owned())].into(),
                    cover_url: Some("https://cdn2.steamgriddb.com/grid/a.png".into()),
                    ..GameMetadata::new("SteamGridDB")
                })
            }),
        );
        sgdb.art = Some(AtomicBool::new(false));
        sgdb.exact_fixed = Some("5249".into());
        let sgdb = Arc::new(sgdb);
        let a = agent(vec![igdb, sgdb.clone()], Arc::new(TestClock::at(now())));
        let g = Game {
            id: "gog:1".into(),
            title: "Lanterna".into(),
            store: Some(GameStore::Gog),
            ..Default::default()
        };

        let off = a.update(&g, None, false, NONE).metadata.unwrap();
        assert_eq!(off.cover_url, None);
        assert!(sgdb.fetched().is_empty());

        sgdb.art.as_ref().unwrap().store(true, Ordering::SeqCst);
        assert!(a.needs_update(&g, Some(&off), false, NONE));
        let on = a.update(&g, Some(&off), false, NONE).metadata.unwrap();
        assert_eq!(
            on.cover_url.as_deref(),
            Some("https://cdn2.steamgriddb.com/grid/a.png")
        );

        // Jogo da Steam já tem capa: o SteamGridDB nem é consultado.
        let steam = Arc::new(
            FakeProvider::with("Steam", Box::new(|id| Some(part("Steam", id)))).exact("Steam"),
        );
        let before = sgdb.fetched().len();
        let r = agent(vec![steam, sgdb.clone()], Arc::new(TestClock::at(now())))
            .update(
                &game(GameStore::Steam, "Lanterna Distante", Some(4000001)),
                None,
                false,
                NONE,
            )
            .metadata
            .unwrap();
        assert_eq!(sgdb.fetched().len(), before);
        assert!(r.consulted.iter().any(|c| c == "SteamGridDB"));
        assert_eq!(r.cover_url.as_deref(), Some("steam-art://4000001/cover"));

        assert!(!a.search(&g).iter().any(|m| m.source == "SteamGridDB"));
        assert!(a.sources(&g).iter().any(|p| p.name() == "SteamGridDB"));
    }

    #[test]
    fn validation_resolves_slugs_and_returns_the_name() {
        let mut igdb = FakeProvider::with(
            "IGDB",
            Box::new(|id| {
                (id == "113112").then(|| GameMetadata {
                    title: Some("Hades".into()),
                    ..part("IGDB", id)
                })
            }),
        );
        igdb.slug = Some(("hades".into(), "113112".into()));
        let a = agent(vec![Arc::new(igdb)], Arc::new(TestClock::at(now())));
        let parse = |t| crate::metadata::art::parse_source_id("IGDB", t);
        let (id, m) = a.validate("IGDB", &parse("igdb.com/games/hades")).unwrap();
        assert_eq!(id, "113112");
        assert_eq!(m.title.as_deref(), Some("Hades"));
        assert!(a.validate("IGDB", &parse("999")).is_none());
    }
}
