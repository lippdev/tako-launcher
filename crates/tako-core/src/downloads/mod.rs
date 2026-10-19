//! Downloads por torrent a partir de fontes do usuário (DL-001, ADR-0028).
//!
//! O core tem o modelo, as regras (fila, um download ativo por jogo, sem semear ao concluir) e o contrato
//! [`Downloader`] do motor; o motor de verdade fica em `tako-downloads`, e [`fake::MemoryDownloader`] serve aos
//! testes. Ao terminar, o Tako só mostra "Abrir pasta": nunca executa nem aponta nada do conteúdo baixado (§6.7).

pub mod fake;
pub mod paths;
pub use paths::is_inside_root;
pub mod source;

use std::path::{Path, PathBuf};

/// Id do download no motor.
pub type EngineId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadError {
    InvalidUri,
    UnsafePaths,
    NoSpace { missing: u64 },
    GameBusy,
    Duplicate,
    NotFound,
    Engine(String),
}

impl std::error::Error for DownloadError {}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::InvalidUri => {
                write!(f, "{}", crate::i18n::t("downloads.error.invalid_uri"))
            }
            DownloadError::UnsafePaths => {
                write!(f, "{}", crate::i18n::t("downloads.error.unsafe_paths"))
            }
            DownloadError::NoSpace { missing } => {
                let m = missing.to_string();
                write!(
                    f,
                    "{}",
                    crate::i18n::tf("downloads.error.no_space", &[("missing", &m)])
                )
            }
            DownloadError::GameBusy => write!(f, "{}", crate::i18n::t("downloads.error.game_busy")),
            DownloadError::Duplicate => {
                write!(f, "{}", crate::i18n::t("downloads.error.duplicate"))
            }
            DownloadError::NotFound => write!(f, "{}", crate::i18n::t("downloads.error.not_found")),
            DownloadError::Engine(err) => {
                write!(
                    f,
                    "{}",
                    crate::i18n::tf("downloads.error.engine", &[("error", err)])
                )
            }
        }
    }
}

/// O que o motor sabe de um download.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EngineStatus {
    pub done: u64,
    /// `None` até os metadados chegarem (magnet).
    pub total: Option<u64>,
    pub down_bps: u64,
    pub finished: bool,
    pub error: Option<DownloadError>,
}

/// Motor de download (torrent hoje; HTTP do DL-002 depois). Nada aqui bloqueia a interface: metadados, validação
/// dos caminhos e espaço livre acontecem no motor, e o resultado aparece em [`Downloader::status`].
pub trait Downloader: Send {
    /// Começa gravando só dentro de `folder`. `resume` = o download já é do Tako e já começou antes (reabrir,
    /// tentar de novo): só então o motor pode escrever sobre os arquivos que já estão na pasta.
    fn start(&mut self, uri: &str, folder: &Path, resume: bool) -> Result<EngineId, DownloadError>;
    fn pause(&mut self, id: EngineId);
    fn resume(&mut self, id: EngineId);
    /// Tira do motor (sem semear depois) e, se pedido, apaga os arquivos.
    fn remove(&mut self, id: EngineId, delete_files: bool);
    /// Tira um download concluído do motor e limpa metadados privados da pasta, sem apagar os arquivos.
    fn finish(&mut self, id: EngineId) {
        self.remove(id, false);
    }
    fn status(&self, id: EngineId) -> Option<EngineStatus>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadState {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed(String),
}

impl DownloadState {
    pub fn key(&self) -> &'static str {
        match self {
            DownloadState::Queued => "queued",
            DownloadState::Downloading => "downloading",
            DownloadState::Paused => "paused",
            DownloadState::Completed => "completed",
            DownloadState::Failed(_) => "failed",
        }
    }

    /// Ao ler do banco: o que estava baixando volta pausado (ADR-0028 §3).
    pub fn from_key(key: &str, error: &str) -> DownloadState {
        match key {
            "queued" => DownloadState::Queued,
            "completed" => DownloadState::Completed,
            "failed" => DownloadState::Failed(error.to_owned()),
            _ => DownloadState::Paused,
        }
    }

    fn open(&self) -> bool {
        !matches!(self, DownloadState::Completed | DownloadState::Failed(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub id: u64,
    /// Título do item na fonte (texto simples).
    pub title: String,
    /// Jogo da biblioteca escolhido pelo usuário (`Game.id`); o título da fonte só sugere.
    pub game_id: Option<String>,
    /// Nome da fonte de onde veio (procedência).
    pub source: String,
    pub uri: String,
    pub folder: PathBuf,
    pub state: DownloadState,
    pub done: u64,
    pub total: Option<u64>,
    pub speed: u64,
    engine: Option<EngineId>,
}

impl Download {
    pub fn fraction(&self) -> Option<f32> {
        self.total
            .filter(|t| *t > 0)
            .map(|t| (self.done as f64 / t as f64).clamp(0., 1.) as f32)
    }

    /// Para ler do banco.
    #[allow(clippy::too_many_arguments)]
    pub fn stored(
        id: u64,
        title: String,
        game_id: Option<String>,
        source: String,
        uri: String,
        folder: PathBuf,
        state: DownloadState,
        done: u64,
        total: Option<u64>,
    ) -> Download {
        Download {
            id,
            title,
            game_id,
            source,
            uri,
            folder,
            state,
            done,
            total,
            speed: 0,
            engine: None,
        }
    }
}

/// Pedido de download: o item escolhido na fonte e o jogo escolhido pelo usuário.
#[derive(Debug, Clone)]
pub struct NewDownload {
    pub title: String,
    pub game_id: Option<String>,
    pub source: String,
    pub uri: String,
}

/// Resumo para a pílula da barra e o topo do painel.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Summary {
    pub downloading: usize,
    pub queued: usize,
    pub speed: u64,
    pub done: u64,
    pub total: u64,
}

/// Downloads do usuário: fila (um baixando por vez), estados e vínculo com o jogo.
pub struct Downloads {
    engine: Box<dyn Downloader>,
    items: Vec<Download>,
    root: PathBuf,
    max_active: usize,
}

impl Downloads {
    /// `root` = pasta onde cada download ganha a sua subpasta; `saved` = o que estava no banco.
    pub fn new(engine: Box<dyn Downloader>, root: PathBuf, saved: Vec<Download>) -> Downloads {
        let items = saved
            .into_iter()
            .map(|mut d| {
                if !paths::is_inside_root(&root, &d.folder) {
                    d.state = DownloadState::Failed(crate::i18n::t(
                        "downloads.error.folder_outside_root",
                    ));
                } else if d.state == DownloadState::Downloading {
                    d.state = DownloadState::Paused;
                }
                d.engine = None;
                d.speed = 0;
                d
            })
            .collect();
        let mut downloads = Downloads {
            engine,
            items,
            root,
            max_active: 1,
        };
        downloads.fill_queue();
        downloads
    }

    pub fn list(&self) -> &[Download] {
        &self.items
    }

    pub fn get(&self, id: u64) -> Option<&Download> {
        self.items.iter().find(|d| d.id == id)
    }

    /// O download aberto (não concluído nem com erro) do jogo.
    pub fn for_game(&self, game_id: &str) -> Option<&Download> {
        self.items
            .iter()
            .find(|d| d.state.open() && d.game_id.as_deref() == Some(game_id))
    }

    pub fn summary(&self) -> Summary {
        let mut s = Summary::default();
        for d in self.items.iter().filter(|d| d.state.open()) {
            match d.state {
                DownloadState::Downloading => s.downloading += 1,
                DownloadState::Queued => s.queued += 1,
                _ => {}
            }
            s.speed += d.speed;
            if let Some(t) = d.total {
                s.done += d.done.min(t);
                s.total += t;
            }
        }
        s
    }

    /// Há algo baixando, pausado ou na fila (a pílula aparece).
    pub fn has_open(&self) -> bool {
        self.items.iter().any(|d| d.state.open())
    }

    pub fn add(&mut self, new: NewDownload) -> Result<u64, DownloadError> {
        let uri = source::valid_uri(&new.uri).ok_or(DownloadError::InvalidUri)?;
        if let Some(g) = &new.game_id
            && self.for_game(g).is_some()
        {
            return Err(DownloadError::GameBusy);
        }
        let hash = source::infohash(&uri);
        if self.items.iter().any(|d| {
            d.state.open() && (d.uri == uri || (hash.is_some() && source::infohash(&d.uri) == hash))
        }) {
            return Err(DownloadError::Duplicate);
        }
        let id = self.items.iter().map(|d| d.id).max().unwrap_or(0) + 1;
        let mut folder = self.root.join(paths::folder_name(&new.title));
        // Nunca grava numa pasta que já existe (do usuário ou de outro item): ganha " (2)", " (3)"...
        let base = folder.clone();
        let mut n = 2;
        while self.items.iter().any(|d| d.folder == folder)
            || std::fs::symlink_metadata(&folder).is_ok()
        {
            folder = PathBuf::from(format!("{} ({n})", base.display()));
            n += 1;
        }
        self.items.push(Download {
            id,
            title: new.title,
            game_id: new.game_id,
            source: new.source,
            uri,
            folder,
            state: DownloadState::Queued,
            done: 0,
            total: None,
            speed: 0,
            engine: None,
        });
        self.fill_queue();
        Ok(id)
    }

    fn active(&self) -> usize {
        self.items
            .iter()
            .filter(|d| d.state == DownloadState::Downloading)
            .count()
    }

    fn run(&mut self, index: usize) {
        let d = &mut self.items[index];
        if !paths::is_inside_root(&self.root, &d.folder) {
            d.engine = None;
            d.speed = 0;
            d.state = DownloadState::Failed(crate::i18n::t("downloads.error.folder_outside_root"));
            return;
        }
        // Só o que saiu da fila alguma vez já tem arquivos do Tako na pasta.
        let resume = d.state != DownloadState::Queued;
        let started = match d.engine {
            Some(e) => {
                self.engine.resume(e);
                Ok(e)
            }
            None => self.engine.start(&d.uri, &d.folder, resume),
        };
        match started {
            Ok(e) => {
                d.engine = Some(e);
                d.state = DownloadState::Downloading;
            }
            Err(e) => d.state = DownloadState::Failed(e.to_string()),
        }
    }

    fn fill_queue(&mut self) {
        while self.active() < self.max_active {
            let Some(i) = self
                .items
                .iter()
                .position(|d| d.state == DownloadState::Queued)
            else {
                return;
            };
            self.run(i);
        }
    }

    fn index(&self, id: u64) -> Result<usize, DownloadError> {
        self.items
            .iter()
            .position(|d| d.id == id)
            .ok_or(DownloadError::NotFound)
    }

    pub fn pause(&mut self, id: u64) -> Result<(), DownloadError> {
        let i = self.index(id)?;
        let d = &mut self.items[i];
        if d.state == DownloadState::Downloading {
            if let Some(e) = d.engine {
                self.engine.pause(e);
            }
            d.state = DownloadState::Paused;
            d.speed = 0;
        }
        Ok(())
    }

    /// Retoma um pausado (ou com erro), ou "Baixar agora" de um na fila: a decisão é do usuário, passa da fila.
    pub fn resume(&mut self, id: u64) -> Result<(), DownloadError> {
        let i = self.index(id)?;
        if matches!(
            self.items[i].state,
            DownloadState::Paused | DownloadState::Queued | DownloadState::Failed(_)
        ) {
            if matches!(self.items[i].state, DownloadState::Failed(_)) {
                self.items[i].engine = None;
            }
            self.run(i);
        }
        Ok(())
    }

    /// Cancela (ou tira um concluído da lista). Devolve o download para quem chama desfazer o vínculo.
    pub fn cancel(&mut self, id: u64, delete_files: bool) -> Result<Download, DownloadError> {
        let i = self.index(id)?;
        let d = self.items.remove(i);
        if let Some(e) = d.engine {
            self.engine.remove(e, delete_files);
        }
        self.fill_queue();
        Ok(d)
    }

    /// Lê o motor. Concluído sai do motor na hora (sem semear); a fila anda. `true` = algo mudou.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        for d in &mut self.items {
            let (Some(e), DownloadState::Downloading) = (d.engine, &d.state) else {
                continue;
            };
            let Some(s) = self.engine.status(e) else {
                continue;
            };
            let before = (d.done, d.total, d.speed);
            d.done = s.done;
            d.total = s.total.or(d.total);
            d.speed = s.down_bps;
            changed |= before != (d.done, d.total, d.speed);
            if let Some(err) = s.error {
                self.engine.remove(e, false);
                d.engine = None;
                d.speed = 0;
                d.state = DownloadState::Failed(err.to_string());
                changed = true;
            } else if s.finished {
                self.engine.finish(e);
                d.engine = None;
                d.speed = 0;
                if let Some(t) = d.total {
                    d.done = t;
                }
                d.state = DownloadState::Completed;
                changed = true;
            }
        }
        if changed {
            self.fill_queue();
        }
        changed
    }
}

/// Itens das fontes que parecem com o título do jogo, do mais parecido ao menos (no máximo `limit`): só sugestão,
/// quem decide é o usuário.
pub fn suggest<'a>(
    game_title: &str,
    sources: impl IntoIterator<Item = (&'a str, &'a source::Source)>,
    limit: usize,
) -> Vec<(&'a str, &'a source::SourceItem)> {
    let game_tokens = crate::library::title_matcher::tokens(game_title);
    if game_tokens.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(f64, &str, &source::SourceItem)> = sources
        .into_iter()
        .flat_map(|(name, s)| s.items.iter().map(move |i| (name, i)))
        .map(|(n, i)| {
            (
                crate::library::title_matcher::score_tokens(&game_tokens, &i.tokens),
                n,
                i,
            )
        })
        .filter(|(s, _, _)| *s >= 0.5)
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, n, i)| (n, i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::fake::MemoryDownloader;
    use super::*;

    const A: &str = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567";
    const B: &str = "magnet:?xt=urn:btih:89abcdef0123456789abcdef0123456789abcdef";

    fn new(title: &str, game: Option<&str>, uri: &str) -> NewDownload {
        NewDownload {
            title: title.into(),
            game_id: game.map(Into::into),
            source: "Minha fonte".into(),
            uri: uri.into(),
        }
    }

    fn downloads() -> (Downloads, MemoryDownloader) {
        let engine = MemoryDownloader::default();
        (
            Downloads::new(Box::new(engine.clone()), PathBuf::from("/d/Tako"), vec![]),
            engine,
        )
    }

    #[test]
    fn resume_failed_download_outside_root_does_not_call_engine() {
        let engine = MemoryDownloader::default();
        let root = PathBuf::from("/d/Tako");
        let saved = vec![Download {
            id: 1,
            title: "Malicioso".into(),
            game_id: None,
            source: "Fonte".into(),
            uri: A.into(),
            folder: PathBuf::from("/etc/downloads"),
            state: DownloadState::Failed("erro anterior".into()),
            done: 0,
            total: None,
            speed: 0,
            engine: None,
        }];
        let mut d = Downloads::new(Box::new(engine.clone()), root, saved);
        assert!(matches!(d.get(1).unwrap().state, DownloadState::Failed(_)));
        assert!(engine.started().is_empty());

        d.resume(1).unwrap();
        assert!(matches!(d.get(1).unwrap().state, DownloadState::Failed(_)));
        assert!(
            engine.started().is_empty(),
            "motor nunca deve ser chamado para downloads fora da raiz"
        );
    }

    #[test]
    fn saved_downloads_outside_root_become_failed_and_do_not_start() {
        let engine = MemoryDownloader::default();
        let root = PathBuf::from("/d/Tako");
        let saved = vec![
            Download {
                id: 1,
                title: "Malicioso".into(),
                game_id: None,
                source: "Fonte".into(),
                uri: A.into(),
                folder: PathBuf::from("/etc"),
                state: DownloadState::Queued,
                done: 0,
                total: None,
                speed: 0,
                engine: None,
            },
            Download {
                id: 2,
                title: "Travessia".into(),
                game_id: None,
                source: "Fonte".into(),
                uri: B.into(),
                folder: PathBuf::from("/d/Tako/../../outside"),
                state: DownloadState::Downloading,
                done: 0,
                total: None,
                speed: 0,
                engine: None,
            },
            Download {
                id: 3,
                title: "Legítimo".into(),
                game_id: None,
                source: "Fonte".into(),
                uri: A.into(),
                folder: PathBuf::from("/d/Tako/Jogo C"),
                state: DownloadState::Downloading,
                done: 0,
                total: None,
                speed: 0,
                engine: None,
            },
        ];
        let d = Downloads::new(Box::new(engine), root, saved);
        assert!(matches!(d.get(1).unwrap().state, DownloadState::Failed(_)));
        assert!(matches!(d.get(2).unwrap().state, DownloadState::Failed(_)));
        assert_eq!(d.get(3).unwrap().state, DownloadState::Paused);
        assert_eq!(d.summary().downloading, 0);
    }

    #[test]
    fn one_downloads_at_a_time_and_the_queue_moves_when_it_finishes() {
        let (mut d, engine) = downloads();
        let a = d.add(new("Jogo A", Some("g1"), A)).unwrap();
        let b = d.add(new("Jogo B", Some("g2"), B)).unwrap();
        assert_eq!(d.get(a).unwrap().state, DownloadState::Downloading);
        assert_eq!(d.get(b).unwrap().state, DownloadState::Queued);
        assert_eq!(d.get(a).unwrap().folder, PathBuf::from("/d/Tako/Jogo A"));

        engine.set(1, 50, Some(100), false);
        assert!(d.poll());
        assert_eq!(d.get(a).unwrap().fraction(), Some(0.5));
        assert_eq!(d.summary().downloading, 1);
        assert_eq!(d.summary().queued, 1);

        engine.set(1, 100, Some(100), true);
        assert!(d.poll());
        assert_eq!(d.get(a).unwrap().state, DownloadState::Completed);
        // Concluído sai do motor (não semeia) e o próximo começa.
        assert!(engine.removed().contains(&(1, false)));
        assert_eq!(d.get(b).unwrap().state, DownloadState::Downloading);
    }

    #[test]
    fn pause_resume_and_cancel() {
        let (mut d, engine) = downloads();
        let a = d.add(new("A", Some("g1"), A)).unwrap();
        d.pause(a).unwrap();
        assert_eq!(d.get(a).unwrap().state, DownloadState::Paused);
        assert!(engine.paused(1));
        d.resume(a).unwrap();
        assert_eq!(d.get(a).unwrap().state, DownloadState::Downloading);
        assert!(!engine.paused(1));
        let gone = d.cancel(a, true).unwrap();
        assert_eq!(gone.game_id.as_deref(), Some("g1"));
        assert!(engine.removed().contains(&(1, true)));
        assert!(d.list().is_empty());
        assert_eq!(d.cancel(a, false), Err(DownloadError::NotFound));
    }

    #[test]
    fn a_game_has_one_open_download_and_an_item_is_not_added_twice() {
        let (mut d, _) = downloads();
        d.add(new("A", Some("g1"), A)).unwrap();
        assert_eq!(d.add(new("B", Some("g1"), B)), Err(DownloadError::GameBusy));
        assert_eq!(
            d.add(new("A de novo", None, &A.to_uppercase())),
            Err(DownloadError::Duplicate)
        );
        assert_eq!(
            d.add(new("X", None, "file:///etc/passwd")),
            Err(DownloadError::InvalidUri)
        );
    }

    #[test]
    fn engine_errors_fail_the_download_and_retry_starts_again() {
        let (mut d, engine) = downloads();
        let a = d.add(new("A", None, A)).unwrap();
        engine.fail(1, DownloadError::UnsafePaths);
        d.poll();
        assert!(matches!(d.get(a).unwrap().state, DownloadState::Failed(_)));
        assert!(d.for_game("g").is_none());
        d.resume(a).unwrap();
        assert_eq!(d.get(a).unwrap().state, DownloadState::Downloading);
        assert_eq!(engine.started().len(), 2);
    }

    #[test]
    fn reopening_turns_running_downloads_into_paused_and_same_titles_get_their_own_folder() {
        let saved = vec![Download::stored(
            7,
            "A".into(),
            Some("g1".into()),
            "f".into(),
            A.into(),
            "/d/Tako/A".into(),
            DownloadState::Downloading,
            10,
            Some(100),
        )];
        let engine = MemoryDownloader::default();
        let mut d = Downloads::new(Box::new(engine.clone()), "/d/Tako".into(), saved);
        assert_eq!(d.get(7).unwrap().state, DownloadState::Paused);
        assert!(engine.started().is_empty());
        let b = d.add(new("A", None, B)).unwrap();
        assert_eq!(b, 8);
        assert_eq!(d.get(b).unwrap().folder, PathBuf::from("/d/Tako/A (2)"));
    }

    #[test]
    fn an_existing_folder_on_disk_is_never_reused_and_only_resumes_may_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Jogo")).unwrap();
        std::fs::create_dir_all(dir.path().join("Jogo (2)")).unwrap();
        let engine = MemoryDownloader::default();
        let mut d = Downloads::new(Box::new(engine.clone()), dir.path().into(), vec![]);
        let a = d.add(new("Jogo", None, A)).unwrap();
        assert_eq!(d.get(a).unwrap().folder, dir.path().join("Jogo (3)"));
        assert_eq!(engine.resumes(), vec![false]);
        d.pause(a).unwrap();
        d.cancel(a, false).unwrap();
        // Item que já tinha começado (lido do banco, pausado): retomar pode escrever sobre a pasta dele.
        let saved = vec![Download::stored(
            9,
            "B".into(),
            None,
            "f".into(),
            B.into(),
            dir.path().join("B"),
            DownloadState::Paused,
            10,
            Some(100),
        )];
        let mut d = Downloads::new(Box::new(engine.clone()), dir.path().into(), saved);
        d.resume(9).unwrap();
        assert_eq!(engine.resumes(), vec![false, true]);
    }

    #[test]
    fn suggestions_only_list_similar_titles() {
        let src = source::parse(
            format!(
                r#"{{"name":"F","downloads":[{{"title":"Jogo Exemplo Deluxe Edition","uris":["{A}"]}},{{"title":"Outra Coisa","uris":["{B}"]}}]}}"#
            )
            .as_bytes(),
        )
        .unwrap();
        let s = suggest("Jogo Exemplo", [("F", &src)], 5);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].1.title, "Jogo Exemplo Deluxe Edition");
    }

    #[test]
    fn reopening_resumes_queued_downloads_automatically() {
        let saved = vec![
            Download::stored(
                1,
                "A".into(),
                None,
                "f".into(),
                A.into(),
                "/d/Tako/A".into(),
                DownloadState::Downloading,
                10,
                Some(100),
            ),
            Download::stored(
                2,
                "B".into(),
                None,
                "f".into(),
                B.into(),
                "/d/Tako/B".into(),
                DownloadState::Queued,
                0,
                Some(200),
            ),
        ];
        let engine = MemoryDownloader::default();
        let d = Downloads::new(Box::new(engine.clone()), "/d/Tako".into(), saved);
        assert_eq!(d.get(1).unwrap().state, DownloadState::Paused);
        assert_eq!(d.get(2).unwrap().state, DownloadState::Downloading);
        assert_eq!(engine.started().len(), 1);
        assert_eq!(
            engine.started()[0],
            (B.to_string(), PathBuf::from("/d/Tako/B"))
        );
        assert_eq!(engine.resumes(), vec![false]);
    }
}
