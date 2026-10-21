//! Fila de atividades em segundo plano (UX-006, AtividadeNaBarra): o que o Tako faz sem o usuário pedir agora.
//! Os módulos (lojas, downloads, capas, nuvem, atualização) alimentam a fila; a barra de cima mostra a mais
//! importante. Sem UI e sem SO. O relógio entra como parâmetro (`now`), então os testes não dormem.

use std::time::{Duration, Instant};

/// Quanto tempo uma atividade concluída fica na pílula, com o resumo.
pub const DONE_VISIBLE: Duration = Duration::from_secs(4);

/// Tipos de atividade; a prioridade da pílula está em [`rank`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Download,
    LibraryRefresh,
    UpdateCheck,
    UpdateReady,
    Covers,
    CloudSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Running,
    Done,
    Failed,
}

/// Ação oferecida numa atividade com erro; quem alimenta a fila sabe refazer o trabalho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Retry,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    /// Identifica a atividade para quem a alimenta (ex.: `library`, `download:<id>`); repetir a chave a reinicia.
    pub key: String,
    pub kind: Kind,
    pub state: State,
    pub title: String,
    pub detail: String,
    /// 0 a 100; `None` = sem porcentagem (a pílula gira).
    pub progress: Option<u8>,
    pub action: Option<Action>,
    /// Quando entrou no estado atual (a conclusão expira da pílula a partir daqui).
    since: Instant,
}

/// O que a pílula mostra: a atividade mais importante e quantas outras estão visíveis junto.
#[derive(Debug, PartialEq)]
pub struct Shown<'a> {
    pub activity: &'a Activity,
    pub more: usize,
}

#[derive(Debug, Default)]
pub struct Activities {
    list: Vec<Activity>,
    /// O último início, conclusão ou erro: é o que o leitor de tela anuncia.
    event: Option<Activity>,
}

impl Activities {
    /// Começa (ou reinicia) uma atividade em andamento.
    pub fn start(&mut self, key: &str, kind: Kind, title: &str, detail: &str, now: Instant) {
        let activity = Activity {
            key: key.into(),
            kind,
            state: State::Running,
            title: title.into(),
            detail: detail.into(),
            progress: None,
            action: None,
            since: now,
        };
        self.event = Some(activity.clone());
        match self.list.iter_mut().find(|a| a.key == key) {
            Some(a) => *a = activity,
            None => self.list.push(activity),
        }
    }

    /// Atualiza o andamento; não mexe no que já terminou ou falhou.
    pub fn progress(&mut self, key: &str, percent: Option<u8>, detail: &str) {
        if let Some(a) = self.running(key) {
            a.progress = percent.map(|p| p.min(100));
            a.detail = detail.into();
        }
    }

    /// Conclui com o resumo ("Biblioteca atualizada" · "4 jogos novos").
    pub fn finish(&mut self, key: &str, title: &str, detail: &str, now: Instant) {
        self.end(key, State::Done, title, detail, None, now);
    }

    /// Falha com o motivo em poucas palavras; fica até ser resolvida ou fechada.
    pub fn fail(&mut self, key: &str, reason: &str, action: Option<Action>, now: Instant) {
        self.end(key, State::Failed, reason, "", action, now);
    }

    /// Tira da fila (erro fechado ou resolvido; atividade que deixou de existir).
    pub fn dismiss(&mut self, key: &str) {
        self.list.retain(|a| a.key != key);
    }

    /// "Limpar concluídas": as com erro ficam.
    pub fn clear_done(&mut self) {
        self.list.retain(|a| a.state != State::Done);
    }

    /// Todas, na ordem de chegada (a lista "Atividades").
    pub fn all(&self) -> &[Activity] {
        &self.list
    }

    /// O último início, conclusão ou erro, mesmo que a atividade já tenha saído da pílula.
    pub fn last_event(&self) -> Option<&Activity> {
        self.event.as_ref()
    }

    /// Da mais importante à menos: erro, depois pelo tipo (download → biblioteca → atualização do Tako → o
    /// resto); entre iguais, a que chegou primeiro.
    pub fn ordered(&self) -> Vec<&Activity> {
        let mut all: Vec<&Activity> = self.list.iter().collect();
        all.sort_by_key(|a| (a.state != State::Failed, rank(a.kind)));
        all
    }

    pub fn get(&self, key: &str) -> Option<&Activity> {
        self.list.iter().find(|a| a.key == key)
    }

    /// Em andamento de verdade ("N em andamento"); "Atualização pronta" espera o usuário, não trabalha.
    pub fn running_count(&self) -> usize {
        self.list
            .iter()
            .filter(|a| a.state == State::Running && a.kind != Kind::UpdateReady)
            .count()
    }

    /// Concluída há mais de 4 s já saiu da pílula (continua na lista).
    pub fn is_visible(a: &Activity, now: Instant) -> bool {
        a.state != State::Done || now.saturating_duration_since(a.since) < DONE_VISIBLE
    }

    /// A primeira da [`ordered`](Self::ordered) que ainda está na pílula. `None` = a pílula some.
    pub fn shown(&self, now: Instant) -> Option<Shown<'_>> {
        let visible: Vec<&Activity> = (self.ordered().into_iter())
            .filter(|a| Self::is_visible(a, now))
            .collect();
        Some(Shown {
            activity: *visible.first()?,
            more: visible.len() - 1,
        })
    }

    /// Quando a pílula muda sozinha pelo relógio (uma conclusão expirando); `None` = nada a esperar.
    pub fn next_expiry(&self, now: Instant) -> Option<Instant> {
        self.list
            .iter()
            .filter(|a| a.state == State::Done && Self::is_visible(a, now))
            .map(|a| a.since + DONE_VISIBLE)
            .min()
    }

    /// Só o que está em andamento muda: um aviso atrasado da rede não apaga um erro nem estica uma conclusão.
    fn running(&mut self, key: &str) -> Option<&mut Activity> {
        let found = self
            .list
            .iter_mut()
            .find(|a| a.key == key && a.state == State::Running);
        if found.is_none() {
            tracing::warn!(key, "atividade inexistente ou já encerrada");
        }
        found
    }

    fn end(
        &mut self,
        key: &str,
        state: State,
        title: &str,
        detail: &str,
        action: Option<Action>,
        now: Instant,
    ) {
        if let Some(a) = self.running(key) {
            a.state = state;
            a.title = title.into();
            a.detail = detail.into();
            a.progress = None;
            a.action = action;
            a.since = now;
            self.event = Some(a.clone());
        }
    }
}

/// Prioridade da pílula (menor = mais importante); capas e nuvem empatam e vale a ordem de chegada.
fn rank(kind: Kind) -> u8 {
    match kind {
        Kind::Download => 0,
        Kind::LibraryRefresh => 1,
        Kind::UpdateCheck | Kind::UpdateReady => 2,
        Kind::Covers | Kind::CloudSync => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(t0: Instant, s: u64) -> Instant {
        t0 + Duration::from_secs(s)
    }

    #[test]
    fn empty_queue_hides_the_pill() {
        assert_eq!(Activities::default().shown(Instant::now()), None);
    }

    #[test]
    fn priority_is_error_then_download_then_library_then_update_then_rest() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start("cloud", Kind::CloudSync, "Nuvem", "", t0);
        q.start("covers", Kind::Covers, "Capas", "", t0);
        assert_eq!(q.shown(t0).unwrap().activity.key, "cloud");
        q.start("check", Kind::UpdateCheck, "Procurando", "", t0);
        assert_eq!(q.shown(t0).unwrap().activity.key, "check");
        q.start("library", Kind::LibraryRefresh, "Biblioteca", "", t0);
        assert_eq!(q.shown(t0).unwrap().activity.key, "library");
        q.start("download:1", Kind::Download, "Jogo", "", t0);
        assert_eq!(q.shown(t0).unwrap().activity.key, "download:1");
        q.fail("covers", "Sem rede", Some(Action::Retry), t0);
        let shown = q.shown(t0).unwrap();
        assert_eq!(shown.activity.key, "covers");
        assert_eq!(shown.activity.action, Some(Action::Retry));
    }

    #[test]
    fn ordered_puts_errors_first_then_by_priority_then_by_arrival() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start("cloud", Kind::CloudSync, "Nuvem", "", t0);
        q.start("covers", Kind::Covers, "Capas", "", t0);
        q.start("library", Kind::LibraryRefresh, "Biblioteca", "", t0);
        q.start("check", Kind::UpdateCheck, "Procurando", "", t0);
        q.fail("check", "Sem rede", None, t0);
        let keys: Vec<_> = q.ordered().iter().map(|a| a.key.as_str()).collect();
        assert_eq!(keys, ["check", "library", "cloud", "covers"]);
    }

    #[test]
    fn last_event_is_the_latest_start_finish_or_error_and_never_a_percentage() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        assert_eq!(q.last_event(), None);
        q.start("library", Kind::LibraryRefresh, "Biblioteca", "", t0);
        q.start("download:1", Kind::Download, "Jogo", "", t0);
        q.progress("download:1", Some(40), "40%");
        assert_eq!(q.last_event().unwrap().key, "download:1");
        assert_eq!(q.last_event().unwrap().progress, None);
        // A biblioteca termina com um download na frente: a conclusão dela é o evento.
        q.finish("library", "Biblioteca atualizada", "", t0);
        let e = q.last_event().unwrap();
        assert_eq!((e.key.as_str(), e.state), ("library", State::Done));
        q.fail("download:1", "Sem rede", None, t0);
        assert_eq!(q.last_event().unwrap().state, State::Failed);
    }

    #[test]
    fn several_at_once_count_the_others() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start("library", Kind::LibraryRefresh, "Biblioteca", "", t0);
        q.start("covers", Kind::Covers, "Capas", "", t0);
        q.start("download:1", Kind::Download, "Jogo", "", t0);
        let shown = q.shown(t0).unwrap();
        assert_eq!((shown.activity.key.as_str(), shown.more), ("download:1", 2));
        assert_eq!(q.running_count(), 3);
    }

    #[test]
    fn done_leaves_the_pill_after_four_seconds_but_stays_in_the_list() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start("library", Kind::LibraryRefresh, "Biblioteca", "", t0);
        q.progress("library", Some(150), "Steam · 3 de 5 lojas");
        assert_eq!(q.get("library").unwrap().progress, Some(100));
        q.finish("library", "Biblioteca atualizada", "4 jogos novos", t0);
        assert_eq!(q.next_expiry(t0), Some(secs(t0, 4)));
        assert_eq!(
            q.shown(secs(t0, 3)).unwrap().activity.title,
            "Biblioteca atualizada"
        );
        assert_eq!(q.shown(secs(t0, 4)), None);
        assert_eq!(q.next_expiry(secs(t0, 4)), None);
        assert_eq!(q.all().len(), 1);
        q.clear_done();
        assert!(q.all().is_empty());
    }

    #[test]
    fn done_download_gives_way_to_the_running_library_after_it_expires() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start("library", Kind::LibraryRefresh, "Biblioteca", "", t0);
        q.start("download:1", Kind::Download, "Jogo", "", t0);
        q.finish("download:1", "Jogo instalado", "", t0);
        let shown = q.shown(secs(t0, 1)).unwrap();
        assert_eq!((shown.activity.key.as_str(), shown.more), ("download:1", 1));
        assert_eq!(q.shown(secs(t0, 5)).unwrap().activity.key, "library");
    }

    #[test]
    fn error_stays_until_dismissed_and_survives_clear_done() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start("library", Kind::LibraryRefresh, "Biblioteca", "", t0);
        q.fail("library", "A Epic não respondeu", Some(Action::Retry), t0);
        q.clear_done();
        assert_eq!(
            q.shown(secs(t0, 3600)).unwrap().activity.state,
            State::Failed
        );
        assert_eq!(q.next_expiry(t0), None);
        q.dismiss("library");
        assert_eq!(q.shown(t0), None);
    }

    #[test]
    fn restarting_a_key_replaces_it_and_progress_ignores_finished() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start("covers", Kind::Covers, "Capas", "", t0);
        q.finish("covers", "Capas prontas", "", t0);
        q.progress("covers", Some(50), "x");
        assert_eq!(q.get("covers").unwrap().progress, None);
        q.start("covers", Kind::Covers, "Capas", "12 de 24 jogos", t0);
        assert_eq!(q.all().len(), 1);
        assert_eq!(q.get("covers").unwrap().state, State::Running);
    }

    #[test]
    fn late_finish_neither_clears_an_error_nor_extends_a_done() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start("download:1", Kind::Download, "Jogo", "", t0);
        q.fail("download:1", "Sem espaço", Some(Action::Retry), t0);
        q.finish("download:1", "Jogo instalado", "", secs(t0, 1));
        assert_eq!(q.get("download:1").unwrap().state, State::Failed);
        q.start("library", Kind::LibraryRefresh, "Biblioteca", "", t0);
        q.finish("library", "Biblioteca atualizada", "", t0);
        q.finish("library", "Biblioteca atualizada", "", secs(t0, 3));
        assert_eq!(q.next_expiry(t0), Some(secs(t0, 4)));
    }

    #[test]
    fn update_ready_is_not_counted_as_running() {
        let t0 = Instant::now();
        let mut q = Activities::default();
        q.start(
            "update-ready",
            Kind::UpdateReady,
            "Atualização pronta",
            "",
            t0,
        );
        assert_eq!(q.running_count(), 0);
        assert_eq!(q.shown(t0).unwrap().more, 0);
    }
}
