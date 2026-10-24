//! Contexto compartilhado pelas páginas e camadas: biblioteca, configurações, plataforma e o que a interface
//! pede ao resto do app (efeitos).

use super::Cmd;
use super::LAUNCH_SLOW;
use crate::gamepad::{PadEvent, PadFamily};
use crate::model::Hint;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tako_core::i18n::{self, Lang};
use tako_core::input::profiles::{Profile, Profiles};
use tako_core::input::{AppCommand, GamepadButtons};
use tako_core::library::{CollectionLibrary, GameLibrary};
use tako_core::paths::DataPaths;
use tako_core::persistence::{GameChoicesStore, GameSettingsStore, SettingsStore, TakoDatabase};
use tako_platform::Platform;

/// Pedido da interface ao app (tarefas longas, outras janelas, sair).
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Abrir o jogo (o app resolve configurações e passa a sessão ao supervisor).
    Launch(String),
    /// Voltar ao jogo que está rodando.
    ResumeGame,
    /// Encerrar o jogo que está rodando.
    EndGame,
    /// O jogo não fechou com o pedido: encerrar à força.
    ForceEndGame,
    /// Central › Início: a biblioteca abre e o jogo continua rodando.
    GoHomeKeepingGame,
    /// O usuário mudou o ponteiro na Central (esconder / mover com o analógico).
    PointerChanged,
    /// Releitura da biblioteca disparada internamente (mudança de emulador, chave da Steam, primeiro uso).
    Rescan,
    /// Releitura da biblioteca pedida pelo usuário (botão no desktop, tela cheia ou configurações).
    UserRescan,
    /// Escolher um arquivo para "Adicionar jogo".
    AddGame,
    OpenFolder(String),
    OpenUrl(String),
    CheckUpdates,
    InstallUpdate,
    RefreshMetadata(Option<String>),
    ClearArtCache,
    ApplyTheme(String),
    /// Escolher um pacote `.takotheme` e instalá-lo (`tako_themes::install`), depois redesenhar a lista.
    InstallTheme,
    /// Sair do modo seguro: reabrir o Tako normalmente (temas e opções voltam).
    ExitSafeMode,
    /// Outro perfil passou a ser o em uso (UX-004): reabrir o Tako com os dados dele.
    SwitchProfile,
    ImportPlaynite,
    SaveDiagnostics,
    /// Aplicar a tela de jogo para o usuário confirmar (configurações).
    TestGameScreen,
    /// Mudou algo que exige reler a biblioteca (lojas ligadas/desligadas).
    ReloadLibrary,
    ExitToDesktop,
    /// Trocar entre o modo desktop e a tela cheia (ADR-0027).
    SwitchMode(crate::mode::UiMode),
    /// Botão da janela acionado pelo teclado: `min`, `max` ou `close` (o mesmo `window-action` do mouse).
    Window(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputKind {
    #[default]
    Gamepad,
    Keyboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// Jogo rodando em segundo plano (o usuário voltou à Início sem fechar).
#[derive(Debug, Clone, PartialEq)]
pub struct RunningGame {
    pub id: String,
    pub title: String,
    /// Processo raiz, quando o Tako abriu o executável direto (o código de saída dele, no fim).
    pub pid: Option<u32>,
    /// Quando o Tako abriu o jogo ("Sessão atual 42 min"); `None` se a interface reabriu com o jogo já rodando.
    pub started: Option<std::time::Instant>,
}

pub use tako_sources::report::LibrarySourceReport as SourceReport;

/// Configurações do app: `settings` do banco (chave/valor, mesmo formato das alphas), com cache em memória.
pub struct Settings {
    store: Option<SettingsStore>,
    values: HashMap<String, String>,
    pub locked: bool,
}

impl Settings {
    pub fn new(store: Option<SettingsStore>) -> Settings {
        let values = store
            .as_ref()
            .and_then(|s| s.load_all().ok())
            .unwrap_or_default();
        Settings {
            store,
            values,
            locked: false,
        }
    }

    /// Nenhuma configuração gravada (instalação nova).
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn string(&self, key: &str) -> String {
        self.get(key).unwrap_or_default().to_owned()
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        self.get(key)
            .and_then(tako_core::persistence::parse_bool)
            .unwrap_or(default)
    }

    pub fn set(&mut self, key: &str, value: &str) {
        if self.locked {
            tracing::warn!(key, "ignorado: configurações travadas durante restauração");
            return;
        }
        self.values.insert(key.to_owned(), value.to_owned());
        if let Some(s) = &self.store
            && let Err(e) = s.set(key, value)
        {
            tracing::error!("não foi possível gravar a configuração {key}: {e}");
        }
    }

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.set(key, if value { "true" } else { "false" });
    }

    pub fn toggle(&mut self, key: &str, default: bool) -> bool {
        let v = !self.bool(key, default);
        self.set_bool(key, v);
        v
    }
}

/// Glifo de um botão do controle na família dele.
pub fn button_glyph(pad: PadFamily, b: GamepadButtons) -> &'static str {
    type B = GamepadButtons;
    match (pad, b) {
        (PadFamily::PlayStation, B::A) => "✕",
        (PadFamily::PlayStation, B::B) => "○",
        (PadFamily::PlayStation, B::X) => "□",
        (PadFamily::PlayStation, B::Y) => "△",
        (PadFamily::PlayStation, B::LEFT_SHOULDER) => "L1",
        (PadFamily::PlayStation, B::RIGHT_SHOULDER) => "R1",
        (PadFamily::PlayStation, B::START) => "Options",
        (PadFamily::PlayStation, B::GUIDE) => "PS",
        // Nintendo: A e B (e X e Y) trocados de lugar (o SDL entrega por posição).
        (PadFamily::Nintendo, B::A) => "B",
        (PadFamily::Nintendo, B::B) => "A",
        (PadFamily::Nintendo, B::X) => "Y",
        (PadFamily::Nintendo, B::Y) => "X",
        (PadFamily::Nintendo, B::LEFT_SHOULDER) => "L",
        (PadFamily::Nintendo, B::RIGHT_SHOULDER) => "R",
        (PadFamily::Nintendo, B::START) => "+",
        (_, B::A) => "A",
        (_, B::B) => "B",
        (_, B::X) => "X",
        (_, B::Y) => "Y",
        (_, B::LEFT_SHOULDER) => "LB",
        (_, B::RIGHT_SHOULDER) => "RB",
        (_, B::START) => "☰",
        (_, B::GUIDE) => "⌂",
        _ => "",
    }
}

/// Nomes das configurações (iguais aos das alphas C#, para a migração).
pub mod keys {
    pub const DYNAMIC_COLOR: &str = "DynamicColor";
    /// Desktop, tema TAKO: "Cores da arte do jogo" (a tela cheia tem a sua, `DYNAMIC_COLOR`). Ligado por padrão.
    pub const DESKTOP_ART_COLORS: &str = "DesktopArtColors";
    pub const REDUCED_MOTION: &str = "ReducedMotion";
    /// Modo desktop: sem vidro (tema `solido`), para quem prefere superfícies opacas.
    pub const REDUCED_TRANSPARENCY: &str = "ReducedTransparency";
    /// TituloDoJogo (desktop): "text" = sempre o nome em texto; qualquer outro valor = logo quando houver. Por jogo,
    /// `TITLE_LOGO` + ":" + id guarda "logo", "text" ou nada (usar o padrão).
    pub const TITLE_LOGO: &str = "DesktopTitleLogo";
    pub const RESTORE_ENVIRONMENT: &str = "RestoreEnvironmentAfterGame";
    pub const DEVELOPER: &str = "DeveloperOptions";
    pub const GAME_SCREEN_ENABLED: &str = "GameScreenEnabled";
    pub const GAME_SCREEN_ID: &str = "GameScreenId";
    pub const GAME_SCREEN_NAME: &str = "GameScreenName";
    pub const GAME_SCREEN_HIDE_OTHERS: &str = "GameScreenHideOthers";
    pub const GAME_SCREEN_MODE: &str = "GameScreenMode";
    pub const GAME_SCREEN_HDR: &str = "GameScreenHdr";
    pub const GAME_SCREEN_VRR: &str = "GameScreenVrr";
    /// Resolução, HDR e VRR de cada monitor (PERF-002), em JSON.
    pub const GAME_SCREEN_PRESETS: &str = "GameScreenPresets";
    pub const GAME_SCREEN_CONFIRMED: &str = "GameScreenConfirmedSetup";
    pub const GAME_AUDIO_ID: &str = "GameAudioDeviceId";
    pub const GAME_AUDIO_NAME: &str = "GameAudioDeviceName";
    pub const DISABLED_STORES: &str = "DisabledStores";
    pub const CONTROL_CENTER_CHORD: &str = tako_core::input::profiles::keys::CHORD;
    pub const CONTROL_CENTER_KEY: &str = "ControlCenterKey";
    pub const LANGUAGE: &str = "Language";
    pub const HIDE_CURSOR: &str = "HideCursorDuringGame";
    pub const CONTROLLER_MOUSE: &str = "ControllerMouseDuringGame";
    pub const SAFE_MODE: &str = "SafeMode";
    pub const DISCORD_PRESENCE: &str = "DiscordRichPresence";
    pub const SUNSHINE_SYNC: &str = "SunshineSyncOnRescan";
    pub const FIRST_RUN_DONE: &str = "FirstRunCompleted";
    pub const SMART_COLLECTIONS: &str = "SmartCollections";
    // Marcados para exclusão no CLOUD-001 (PR #137):
    // LocalApiToken nunca deve entrar em backup nem export (segredo local da máquina).
    // LocalApiEnabled não pode ser importado como consentimento (opt-in estrito por máquina).
    pub const API_ENABLED: &str = tako_core::api::API_ENABLED;
    pub const API_PORT: &str = tako_core::api::API_PORT;
    pub const API_TOKEN: &str = tako_core::api::API_TOKEN;
}

/// O que o "Desfazer" de um aviso faz.
pub type Undo = std::rc::Rc<dyn Fn(&mut Ctx)>;

pub struct Ctx {
    pub paths: DataPaths,
    pub library: GameLibrary,
    pub collections: CollectionLibrary,
    pub settings: Settings,
    pub game_settings: Option<GameSettingsStore>,
    pub choices: Option<GameChoicesStore>,
    pub platform: Platform,
    pub version: String,
    pub pad: PadFamily,
    pub pad_name: String,
    /// GUID do SDL do controle em uso (escolhe o perfil de botões).
    pub pad_guid: String,
    /// Controles conectados agora: `(guid, nome)`.
    pub pads: Vec<(String, String)>,
    /// Perfis de botões (INPUT-001), lidos das configurações; mudam só por [`Ctx::save_profiles`].
    pub profiles: Profiles,
    pub pads_connected: usize,
    /// O controle em uso caiu e nenhum outro ficou: o aviso "Controle desconectado" fica na tela.
    pub pad_lost: bool,
    /// Fila de revisão de capas do desktop (FilaDeCapas).
    pub art_review: super::art_review::Queue,
    /// Bateria (%) do controle sem fio em uso; `None` = com fio, desconhecida ou sem controle.
    pub pad_battery: Option<i32>,
    pub input: InputKind,
    pub toast: Option<(String, Instant)>,
    pub toast_kind: ToastKind,
    /// "Desfazer" do aviso atual (ações em lote do desktop); some junto com o aviso.
    pub toast_undo: Option<Undo>,
    /// O aviso atual é o "Bem-vindo de volta": no desktop leva o polvo no lugar do ✓.
    pub toast_mascot: bool,
    /// Ponteiro sobre o aviso (pílula, "Desfazer"): o tempo fica parado.
    pub toast_hover: [bool; 2],
    pub effects: Vec<Effect>,
    pub running: Option<RunningGame>,
    pub sources: Vec<SourceReport>,
    /// Texto da última verificação de atualização (Configurações › Geral).
    pub update_status: String,
    pub update_ready: Option<String>,
    /// Espaço livre no disco padrão das lojas, para o destaque de jogo não instalado (SRC-005).
    pub free_space: Option<tako_core::devices::FreeSpace>,
    pub app_update: crate::app_updates::State,
    /// AtividadeNaBarra: o que roda em segundo plano (UX-006).
    pub activity: super::activities::State,
    pub safe_mode: bool,
    pub scanning: bool,
    /// Restauração de backup em andamento (bloqueia jogar, varrer, restaurar e salvar configurações).
    pub restoring: bool,
    pub pending_link: Option<tako_core::links::Link>,
    /// Colunas da grade da biblioteca (o tema calcula pela largura da janela).
    pub grid_columns: usize,
    /// Camada que uma página pediu para abrir junto com ela.
    pub pending_layer: Option<Box<dyn super::Layer>>,
    /// Abrir os detalhes deste jogo (pedido de um diálogo).
    pub open_details: Option<String>,
    /// Pedido de uma camada para abrir uma rota (ex.: Configurações pelo menu da marca no desktop).
    pub open_route: Option<String>,
    /// Sair dos detalhes (o jogo foi removido).
    pub leave_details: bool,
    /// Jogo a abrir assim que o atual terminar ("Encerrar e jogar").
    pub queued_launch: Option<String>,
    /// Jogos achados na última leitura das lojas (a biblioteca lê daqui no `refresh`).
    pub found: std::sync::Arc<std::sync::Mutex<Vec<tako_core::library::Game>>>,
    /// Cache de imagens em disco (com limite).
    pub art: std::sync::Arc<tako_core::metadata::art_cache::ArtCache>,
    /// Mover o ponteiro com o analógico durante a sessão (o supervisor executa).
    pub controller_mouse: bool,
    /// O jogo terminou: foco nele e "Bem-vindo de volta" no próximo desenho (id, tempo jogado).
    pub welcome_back: Option<(String, std::time::Duration, tako_core::session::GameEnd)>,
    /// Jogos mandados instalar pela loja e até quando esperar por eles (SRC-005).
    pub installing: Vec<(String, Instant)>,
    /// Loja escolhida em "Instalar pela" por jogo; sem escolha vale a loja do próprio jogo (SRC-005).
    pub install_pick: HashMap<String, tako_core::library::GameStore>,
    /// Pedidos de instalação à espera da hora de sair (`details::run_install_queue`).
    pub install_queue: Vec<(Instant, tako_core::library::InstallOffer)>,
    /// Desenhar de novo neste instante mesmo sem entrada (camadas que mudam com o tempo).
    pub redraw_at: Option<Instant>,
    /// Tema em prévia (THEME-002): vale na tela, mas não está salvo; vence o tema escolhido até confirmar ou voltar.
    pub theme_preview: Option<super::theme_preview::ThemePreview>,
    /// Rascunho de personalização de tema enquanto o usuário edita (THEME-003).
    pub theme_draft: Option<tako_themes::overrides::ThemeTokensOverride>,
    /// Indica se o tema atual implementa a API padrão do global `Ui` (THEME-003).
    pub theme_supports_ui: bool,
    /// O tema usa os tokens de fonte e fundo da API padrão.
    pub theme_supports_font: bool,
    pub theme_supports_background: bool,
    /// O tema selecionado falhou ou foi ignorado no modo seguro.
    pub theme_loaded_from_fallback: bool,
    /// Destaque personalizado persistido em disco para o tema atual.
    pub persisted_theme_accent: Option<tako_core::presentation::Rgb>,
    /// Destaque padrão do tema ativo (lido de tokens.toml do tema).
    pub base_theme_accent: Option<tako_core::presentation::Rgb>,
    /// "Abrindo…" está na tela (o aviso de que a janela do jogo apareceu só vale com ela).
    pub launching: bool,
    /// Desde quando o jogo está abrindo: depois de `LAUNCH_SLOW` o botão vira "Ainda abrindo…" com "Cancelar".
    pub launch_since: Option<Instant>,
    /// Saindo para o jogo: a interface some aos poucos antes de fechar.
    pub leaving: bool,
    /// Entrada do app (BRAND-002) na tela: cobre a primeira tela, que já está pronta por baixo. Acaba no tempo
    /// (`INTRO_HOLD`) ou na primeira entrada do usuário, que só pula e não age.
    pub intro: bool,
    /// Até quando a janela fica por cima de tudo em "Abrindo…" (esconde as janelas de inicialização do jogo).
    pub launch_on_top_until: Option<Instant>,
    /// A janela está por cima de tudo agora (o que foi aplicado no SO).
    pub window_on_top: bool,
    /// Levar a janela para estes limites no próximo desenho (teste da tela de jogo).
    pub move_window: Option<tako_core::devices::ScreenBounds>,
    /// Algo mudou e a tela precisa ser redesenhada.
    pub dirty: bool,
    /// O último foco veio do mouse passando por cima: o tema destaca, mas não rola (só teclado/controle revelam).
    pub pointer: bool,
    /// Botões segurados agora (`GamepadButtons`), lidos pelo app a cada 16 ms.
    pub pressed_buttons: u32,
    /// Analógico esquerdo agora (-1..1, Y para baixo positivo).
    pub pad_stick: (f32, f32),
    /// Uma camada que mostra o controle ao vivo pede outro desenho no próximo ciclo (marca a cada desenho).
    pub live_pad: std::cell::Cell<bool>,
    /// Configurações por jogo já lidas (e as do `--simulate`, que não tem banco).
    pub game_overrides: HashMap<String, tako_core::session::settings::GameSettingsOverride>,
    /// Ids fixados em "Fontes e IDs" por jogo (cópia de `game_source_ids`; vazio = "nenhum").
    pub pinned_ids: HashMap<String, tako_core::metadata::agent::PinnedIds>,
    /// Imagens fixadas por jogo (cópia de `game_art`); a biblioteca as reaplica a cada leitura.
    pub art_pins: std::sync::Arc<std::sync::Mutex<HashMap<String, ArtPins>>>,
    /// Fontes de metadados ligadas e a fila; `None` até `metadata::start_metadata` (e no `--simulate`).
    pub metadata: Option<std::rc::Rc<crate::metadata::Metadata>>,
    /// Versão da biblioteca da última verificação de jogos quebrados (LIB-004).
    pub broken_checked: Option<u64>,
    /// A interface está no modo desktop (ADR-0027); `false` = tela cheia.
    pub desktop: bool,
    /// Altura (px físicos) em que a arte de fundo é decodificada: a da janela, em degraus (`tiles::art_height`).
    pub art_height: u32,
    /// Primeira linha visível da lista do desktop (o tema avisa), para pedir só as miniaturas perto dela.
    pub list_first: usize,
    /// O último foco veio do teclado ou do controle: o desktop mostra o anel de foco (o mouse esconde).
    pub focus_visible: bool,
    /// Downloads por torrent (DL-001): fontes, fila e motor; vazio com a função desligada.
    pub downloads: super::downloads::State,
}

pub type ArtPins = std::collections::BTreeMap<tako_core::library::ArtKind, String>;

impl Ctx {
    /// Recarrega banco, configurações, biblioteca e coleções após restaurar backup.
    pub fn reload_data(&mut self) {
        if self.simulated() {
            return;
        }
        let db = TakoDatabase::new(self.paths.database());
        self.settings = Settings::new(Some(SettingsStore::new(db.clone())));
        self.game_settings = Some(GameSettingsStore::new(db.clone()));
        self.choices = Some(GameChoicesStore::new(db));
        self.library.load();
        let _ = self.collections.load();
        self.sync_collections();
        self.dirty = true;
    }

    /// Sem banco nem lojas (`--simulate`).
    pub fn simulated(&self) -> bool {
        self.game_settings.is_none()
    }

    pub fn lang(&self) -> Lang {
        i18n::lang()
    }

    pub fn t(&self, key: &str) -> String {
        i18n::t(key)
    }

    pub fn tf(&self, key: &str, args: &[(&str, &str)]) -> String {
        i18n::tf(key, args)
    }

    pub fn tn(&self, key: &str, n: i64, args: &[(&str, &str)]) -> String {
        i18n::tn(key, n, args)
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.app_update.toast_label.clear();
        self.toast = Some((text.into(), Instant::now()));
        self.toast_kind = ToastKind::Info;
        self.toast_undo = None;
        self.toast_mascot = false;
        self.toast_hover = [false; 2];
        self.dirty = true;
    }

    pub fn toast_success(&mut self, text: impl Into<String>) {
        self.toast(text);
        self.toast_kind = ToastKind::Success;
    }

    pub fn toast_warning(&mut self, text: impl Into<String>) {
        self.toast(text);
        self.toast_kind = ToastKind::Warning;
    }

    pub fn toast_error(&mut self, text: impl Into<String>) {
        self.toast(text);
        self.toast_kind = ToastKind::Error;
    }

    /// "Bem-vindo de volta" ao fechar o jogo (BRAND-001).
    pub fn toast_welcome_back(&mut self, text: impl Into<String>) {
        self.toast(text);
        self.toast_mascot = true;
    }

    /// Aviso com "Desfazer": `undo` volta o que a ação mudou.
    pub fn toast_with_undo(&mut self, text: impl Into<String>, undo: impl Fn(&mut Ctx) + 'static) {
        self.toast(text);
        self.toast_undo = Some(std::rc::Rc::new(undo));
    }

    /// Abrindo há tempo demais (JogarEAbrir): "Ainda abrindo…" e a ação de cancelar.
    pub fn launch_slow(&self, now: Instant) -> bool {
        self.launching
            && self
                .launch_since
                .is_some_and(|t| now.duration_since(t) >= LAUNCH_SLOW)
    }

    /// O aviso atual já passou do tempo (o de controle desconectado fica até um controle voltar).
    pub fn toast_expired(&self, now: Instant) -> bool {
        !(self.pad_lost && self.input == InputKind::Gamepad)
            && !self.toast_hover.contains(&true)
            && self
                .toast
                .as_ref()
                .is_some_and(|(_, at)| now.duration_since(*at) >= self.toast_time())
    }

    /// Ponteiro entrou ou saiu do aviso (`zone` 0/1 = pílula, 2/3 = "Desfazer"). Ao sair, o tempo recomeça.
    pub fn toast_hovered(&mut self, zone: usize, now: Instant) {
        self.toast_hover[zone / 2] = zone % 2 == 1;
        if zone.is_multiple_of(2)
            && let Some((_, at)) = &mut self.toast
        {
            *at = now;
        }
    }

    /// Quanto o aviso fica. Desktop: 4 s, e 8 s com "Desfazer" (dá tempo de clicar); tela cheia: `TOAST_TIME`.
    pub fn toast_time(&self) -> std::time::Duration {
        if self.desktop && !self.app_update.toast_label.is_empty() {
            return std::time::Duration::from_secs(4);
        }
        match (self.desktop, self.toast_undo.is_some()) {
            (true, true) => std::time::Duration::from_secs(8),
            (true, false) => std::time::Duration::from_secs(4),
            _ => super::TOAST_TIME,
        }
    }

    /// Controle entrou ou saiu. Fora do jogo avisa, sem mexer no foco nem nas camadas; durante o jogo o
    /// fluxo da sobreposição segue como está.
    pub fn pad_event(&mut self, ev: PadEvent, count: usize) {
        match ev {
            PadEvent::Connected { name, family, guid } => {
                self.pad = family;
                self.pad_name = name;
                self.pad_guid = guid;
                self.input = InputKind::Gamepad;
                if std::mem::take(&mut self.pad_lost) {
                    self.toast(self.t("status.pad_connected"));
                }
            }
            PadEvent::Disconnected { remaining } => {
                if self.running.is_none() && self.input == InputKind::Gamepad {
                    self.pad_lost = remaining == 0;
                    self.toast(self.t("status.pad_disconnected"));
                }
            }
        }
        self.pads_connected = count;
        self.dirty = true;
    }

    /// Descarta o rascunho de personalização de tema e limpa imagens temporárias (`background.draft.*`).
    pub fn discard_theme_draft(&mut self) {
        if let Some(ref d) = self.theme_draft
            && let Some(ref dir) = d.draft_dir
        {
            let _ = tako_themes::overrides::clean_background_draft_dir(dir);
        }
        let theme_id = tako_themes::selected(&self.paths.themes());
        let _ = tako_themes::overrides::clean_background_draft(
            &self.paths.themes(),
            theme_id.as_deref(),
        );
        self.theme_draft = None;
    }

    /// Atualiza o cache dos destaques do tema lidos do disco.
    pub fn reload_theme_cache(&mut self) {
        let selected_id = tako_themes::selected(&self.paths.themes());
        let theme_id = selected_id.as_deref();
        self.persisted_theme_accent =
            tako_themes::overrides::load_override(&self.paths.themes(), theme_id).and_then(
                |text| {
                    let ov = tako_themes::overrides::ThemeTokensOverride::from_toml_str(&text);
                    ov.accent
                        .as_deref()
                        .and_then(tako_core::presentation::Rgb::parse)
                },
            );
        self.base_theme_accent = if let Some(id) = theme_id {
            let dir = self.paths.themes().join(id);
            std::fs::read_to_string(dir.join("tokens.toml"))
                .ok()
                .and_then(|text| {
                    let ov = tako_themes::overrides::ThemeTokensOverride::from_toml_str(&text);
                    ov.accent
                        .as_deref()
                        .and_then(tako_core::presentation::Rgb::parse)
                })
        } else {
            None
        };
    }

    /// Depois de mudar coleções: os jogos passam a saber em quais estão.
    pub fn sync_collections(&mut self) {
        let Ctx {
            collections,
            library,
            ..
        } = self;
        collections.apply_to(library.games_mut());
        self.dirty = true;
    }

    /// A biblioteca mudou: verifica fora da UI se o arquivo ou a pasta dos jogos adicionados sumiu (LIB-004).
    /// Jogos que abrem por URI de loja ficam de fora.
    pub fn check_broken(&mut self) {
        let version = self.library.version();
        if self.broken_checked == Some(version) {
            return;
        }
        self.broken_checked = Some(version);
        let targets: Vec<(String, String, Option<String>)> = self
            .library
            .games()
            .iter()
            .filter(|g| g.store() == tako_core::library::GameStore::Manual)
            .filter_map(|g| {
                let target = g.launch_target.clone()?;
                Some((g.id.clone(), target, g.working_directory.clone()))
            })
            .collect();
        crate::tasks::run(
            self,
            move || {
                targets
                    .into_iter()
                    .filter(|(_, t, d)| tako_core::library::manual::is_broken(t, d.as_deref()))
                    .map(|(id, ..)| id)
                    .collect::<std::collections::HashSet<_>>()
            },
            |ctx, broken| {
                for g in ctx.library.games_mut() {
                    g.is_broken = broken.contains(&g.id);
                }
                ctx.dirty = true;
            },
        );
    }

    pub fn effect(&mut self, e: Effect) {
        if self.restoring
            && matches!(
                e,
                Effect::Launch(_)
                    | Effect::ResumeGame
                    | Effect::Rescan
                    | Effect::UserRescan
                    | Effect::ReloadLibrary
            )
        {
            self.toast(self.t("settings.system.restore_busy"));
            return;
        }
        self.effects.push(e);
    }

    /// Quanto tempo esperar uma instalação pela loja antes de parar de reler a biblioteca.
    pub const INSTALL_WAIT: Duration = Duration::from_secs(2 * 60 * 60);

    /// Avisa os jogos que terminaram de instalar e larga os que passaram do prazo; `true` = ainda há o que esperar.
    pub fn check_installs(&mut self) -> bool {
        let now = Instant::now();
        let mut done = Vec::new();
        let library = &self.library;
        self.installing
            .retain(|(id, until)| match library.find(id) {
                Some(g) if g.is_installed => {
                    done.push(g.title.clone());
                    false
                }
                Some(_) => now < *until,
                None => false,
            });
        for title in done {
            let text = self.tf("game.installed_now", &[("game", &title)]);
            self.toast(text);
        }
        !self.installing.is_empty()
    }

    /// Glifo do botão para a entrada em uso (controle Xbox, PlayStation, Nintendo ou teclado).
    pub fn glyph(&self, cmd: Cmd) -> &'static str {
        if self.input == InputKind::Keyboard {
            return match cmd {
                Cmd::Accept => "Enter",
                Cmd::Back => "Esc",
                Cmd::Favorite => "F",
                Cmd::Search => "/",
                Cmd::PrevSection => "Q",
                Cmd::NextSection => "E",
                Cmd::Menu => "M",
                Cmd::Guide => "Ctrl+⇧+Tab",
                _ => "",
            };
        }
        self.pad_glyph(cmd)
    }

    /// Glifo do comando no controle, mesmo com o teclado em uso (textos que falam do controle).
    pub fn pad_glyph(&self, cmd: Cmd) -> &'static str {
        let command = match cmd {
            Cmd::Accept => AppCommand::Accept,
            Cmd::Back => AppCommand::Back,
            Cmd::Favorite => AppCommand::Favorite,
            Cmd::Search => AppCommand::Search,
            Cmd::PrevSection => AppCommand::PreviousSection,
            Cmd::NextSection => AppCommand::NextSection,
            Cmd::Menu => AppCommand::Menu,
            Cmd::Guide => return button_glyph(self.pad, GamepadButtons::GUIDE),
            _ => return "",
        };
        // Os botões do perfil do controle em uso (INPUT-001).
        self.profile()
            .buttons
            .button(command)
            .map_or("", |b| button_glyph(self.pad, b))
    }

    /// Perfil de botões do controle em uso (sem controle, o padrão).
    pub fn profile(&self) -> &Profile {
        self.profiles.of(&self.pad_guid)
    }

    pub fn save_profiles(&mut self, profiles: Profiles) {
        for (key, value) in profiles.save() {
            self.settings.set(key, &value);
        }
        self.profiles = profiles;
        self.dirty = true;
    }

    /// A prévia em curso é de um tema instalado (não do padrão): se ele não carregar, a prévia falhou.
    pub fn theme_preview_wants_installed(&self) -> bool {
        self.theme_preview.as_ref().is_some_and(|p| p.id.is_some())
    }

    pub fn hint(&self, cmd: Cmd, label: &str) -> Hint {
        Hint {
            glyph: self.glyph(cmd).into(),
            label: label.into(),
        }
    }

    pub fn hint_raw(&self, glyph: &str, label: &str) -> Hint {
        Hint {
            glyph: glyph.into(),
            label: label.into(),
        }
    }
}
