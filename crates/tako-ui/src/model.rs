//! Structs da API de tema v1, iguais às exportadas por `theme/main.slint`.

use crate::theme_struct;
use slint::{Color, Image};

theme_struct! {
    pub struct Hint { pub glyph: String, pub label: String }
}

theme_struct! {
    /// Linha de painel, configuração ou diálogo. `kind`: toggle, choice, action, info, header, radio, check,
    /// slider, text, button. `style`: "", "primary", "danger" ou "link".
    pub struct PanelRow {
        pub key: String,
        pub kind: String,
        pub title: String,
        pub description: String,
        pub value: String,
        pub on: bool,
        pub enabled: bool,
        pub emphasized: bool,
        pub icon: String,
        pub style: String,
        pub image: Image,
    }
}

impl PanelRow {
    pub fn new(key: impl Into<String>, kind: &str, title: impl Into<String>) -> Self {
        PanelRow {
            key: key.into(),
            kind: kind.into(),
            title: title.into(),
            enabled: true,
            ..Default::default()
        }
    }
    pub fn action(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self::new(key, "action", title)
    }
    pub fn button(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self::new(key, "button", title)
    }
    pub fn header(title: impl Into<String>) -> Self {
        Self::new("", "header", title)
    }
    pub fn toggle(key: impl Into<String>, title: impl Into<String>, on: bool) -> Self {
        PanelRow {
            on,
            ..Self::new(key, "toggle", title)
        }
    }
    pub fn choice(
        key: impl Into<String>,
        title: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        PanelRow {
            value: value.into(),
            ..Self::new(key, "choice", title)
        }
    }
    pub fn info(
        key: impl Into<String>,
        title: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        PanelRow {
            value: value.into(),
            ..Self::new(key, "info", title)
        }
    }
    pub fn radio(key: impl Into<String>, title: impl Into<String>, on: bool) -> Self {
        PanelRow {
            on,
            ..Self::new(key, "radio", title)
        }
    }
    pub fn check(key: impl Into<String>, title: impl Into<String>, on: bool) -> Self {
        PanelRow {
            on,
            ..Self::new(key, "check", title)
        }
    }
    pub fn desc(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = v.into();
        self
    }
    pub fn icon(mut self, i: &str) -> Self {
        self.icon = i.into();
        self
    }
    pub fn primary(mut self) -> Self {
        self.style = "primary".into();
        self
    }
    pub fn style(mut self, s: impl Into<String>) -> Self {
        self.style = s.into();
        self
    }
    pub fn danger(mut self) -> Self {
        self.style = "danger".into();
        self
    }
    /// Ação de texto à esquerda do rodapé do popup do desktop ("Agora não", "Salvar relatório"); a tela cheia ignora.
    pub fn link(mut self) -> Self {
        self.style = "link".into();
        self
    }
    pub fn enabled(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }
    pub fn emphasized(mut self, e: bool) -> Self {
        self.emphasized = e;
        self
    }
    pub fn is_focusable(&self) -> bool {
        self.kind != "header" && self.kind != "separator"
    }
    /// Divisória entre grupos de um menu.
    pub fn separator() -> Self {
        PanelRow::new("", "separator", "")
    }
}

theme_struct! {
    pub struct Tile {
        pub id: String,
        pub title: String,
        pub meta: String,
        pub cover: Image,
        pub accent: Color,
        pub installed: bool,
        pub favorite: bool,
        pub running: bool,
    }
}

theme_struct! {
    pub struct HomeRow { pub title: String, pub count: String, pub tiles: Vec<Tile>, pub see_all: bool }
}

theme_struct! {
    pub struct NavItem {
        pub id: String,
        pub label: String,
        pub icon: String,
        pub count: String,
        pub section: String,
        pub selected: bool,
    }
}

theme_struct! {
    pub struct Hero {
        pub has_game: bool,
        pub title: String,
        pub overline: String,
        pub meta: String,
        pub description: String,
        pub art: Image,
        pub logo: Image,
        pub play_label: String,
        pub running: bool,
    }
}

theme_struct! {
    pub struct Stat { pub label: String, pub value: String }
}

theme_struct! {
    pub struct Details {
        pub title: String,
        pub overline: String,
        pub meta: String,
        pub description: String,
        pub art: Image,
        pub cover: Image,
        pub logo: Image,
        pub hint: String,
    }
}

theme_struct! {
    pub struct Key { pub label: String, pub row: i32, pub col: i32, pub span: i32 }
}

theme_struct! {
    /// Camada sobre a página. `kind`: dialog, panel, keyboard, art, launch, progress, recovery.
    pub struct Overlay {
        pub kind: String,
        pub title: String,
        pub overline: String,
        pub text: String,
        pub rows: Vec<PanelRow>,
        pub sel: i32,
        pub wide: bool,
        // Desktop: as ações vão para o rodapé do popup (link à esquerda, principal por último).
        pub footer: bool,
        // Desktop: popup só com título, texto e ações (tamanho S, 440).
        pub compact: bool,
        pub image: Image,
        pub keys: Vec<Key>,
        pub key_cols: i32,
        pub input: String,
        pub input_hint: String,
        pub secret: bool,
        pub tabs: Vec<String>,
        pub tab: i32,
        pub tabs2: Vec<String>,
        pub tab2: i32,
        pub thumbs: Vec<Tile>,
        pub thumb_cols: i32,
        pub preview: Image,
        pub preview_text: String,
    }
}

theme_struct! {
    /// Linha da lista de jogos do modo desktop (`theme/desktop.slint`, fora da API de tema).
    pub struct DeskGame {
        pub id: String,
        pub title: String,
        pub thumb: Image,
        // Capa 2:3 maior, só na grade e nas categorias, perto da área visível.
        pub cover: Image,
        pub accent: Color,
        pub favorite: bool,
        pub running: bool,
        pub installed: bool,
        // Loja de origem (ícone e nome, para a dica e o rótulo acessível).
        pub store_icon: String,
        pub store_name: String,
        // Marcado na seleção múltipla.
        pub checked: bool,
        // Posição na lista que o tema avisa nos cliques (nas categorias, na lista achatada das fileiras).
        pub index: i32,
        // Rótulo acessível inteiro (título, loja e, se for o caso, "não instalado" e o progresso).
        pub label: String,
        // "" = jogo; "section" = NÃO INSTALADOS; "group" = cabeçalho de uma loja (`index` = ordem do grupo).
        pub kind: String,
        // Cabeçalhos: quantos jogos; o grupo aberto.
        pub count: String,
        pub open: bool,
        // Instalação pela loja em andamento: fração de 0 a 1 (negativa = a loja não informa), "42%" e os rótulos
        // do botão "Instalar <jogo>" e do anel.
        pub installing: bool,
        pub progress: f32,
        pub percent: String,
        pub install_label: String,
    }
}

theme_struct! {
    /// Item com ícone do modo desktop: faixa de estado do destaque, linhas de Detalhes e de Sessões.
    pub struct DeskInfo {
        pub icon: String,
        pub label: String,
        pub value: String,
        // Texto de apoio (horário da sessão).
        pub detail: String,
        // Ponto verde (jogando, instalado).
        pub dot: bool,
        // Etiquetas do usuário (LIB-002), como chips.
        pub chips: Vec<String>,
        // Nota (Metacritic) na faixa de estado.
        pub score: i32,
    }
}

theme_struct! {
    /// Linha da fila de capas (FilaDeCapas), em paralelo às linhas das Configurações (só o desktop).
    pub struct CoverRow {
        pub current: Image,
        pub has_current: bool,
        pub proposed: Image,
        pub has_proposed: bool,
        // Texto no lugar da capa atual ("Sem capa").
        pub current_label: String,
        // Barrinhas preenchidas: 3 alta, 2 média, 1 baixa, 0 sem proposta.
        pub level: i32,
        // "Confiança média · 71%" e o motivo, em português.
        pub confidence: String,
        pub reason: String,
        pub accept: String,
        pub other: String,
    }
}

theme_struct! {
    /// DestaqueFases: a fase 2 do destaque (o que o Tako sabe do jogo além da primeira tela).
    pub struct DeskMore {
        // A fase 2 está aberta.
        pub on: bool,
        // Mais sobre o jogo, Voltar, Sobre, Detalhes, Sessões, Imagens, Trailer.
        pub labels: Vec<String>,
        // A descrição inteira, sem corte.
        pub description: String,
        pub details: Vec<DeskInfo>,
        pub sessions_summary: String,
        pub sessions: Vec<DeskInfo>,
        // Capturas de tela da fonte de metadados (só com a fase aberta).
        pub shots: Vec<Image>,
    }
}

theme_struct! {
    /// Trecho da grade com a seção de não instalados: capas soltas, a divisória NÃO INSTALADOS ou uma loja com as
    /// capas dela. `*_before` dizem quantas linhas de capas e cabeçalhos vêm antes (o tema soma as alturas).
    pub struct DeskBlock {
        // "covers" | "section" | "group"
        pub kind: String,
        pub title: String,
        pub count: String,
        pub label: String,
        pub icon: String,
        pub open: bool,
        pub games: Vec<DeskGame>,
        // Índice (na lista de jogos) da primeira capa, para o `scrolled`; e a ordem do grupo, para o clique.
        pub first: i32,
        pub index: i32,
        pub rows_before: i32,
        pub sections_before: i32,
        pub groups_before: i32,
    }
}

theme_struct! {
    /// Fileira da BibliotecaCategorias: título, quantos jogos, ícone e as capas.
    pub struct DeskCategory {
        pub title: String,
        pub count: String,
        pub icon: String,
        pub games: Vec<DeskGame>,
    }
}

theme_struct! {
    /// Destaque do jogo selecionado no modo desktop.
    pub struct DeskGameView {
        pub has_game: bool,
        pub title: String,
        pub overline: String,
        pub description: String,
        pub art: Image,
        // Cópia de vidro da arte (desfocada e saturada) e a luminância dela em `images::GLASS_GRID`.
        pub art_blur: Image,
        pub art_lum: Vec<f32>,
        pub logo: Image,
        pub accent: Color,
        pub hint: String,
        // Jogo não instalado: espaço livre e por onde a instalação abre, ou o aviso de falta de espaço (`warn`).
        pub install_note: String,
        pub install_warn: bool,
        // Jogando agora: a etiqueta ganha o ponto verde.
        pub running: bool,
        // A descrição foi cortada: o cartão mostra "Mais".
        pub about_more: bool,
        // Até 4 gêneros (texto, não chips) e o "+N" do resto.
        pub genres: Vec<String>,
        pub genres_more: String,
        // Tem trailer (a mídia `video` do jogo).
        pub trailer: bool,
        // A arte (ou a cópia de vidro) ainda carrega: a janela segura a do jogo anterior para não piscar.
        pub art_loading: bool,
        pub store_icon: String,
        pub store_name: String,
        pub play_status: String,
        pub play_status_label: String,
        // TituloDoJogo: o título é o logo (senão, o nome em texto).
        pub show_logo: bool,
        // O logo ainda carrega: o título fica vazio (nem o nome em texto) até ele chegar.
        pub title_pending: bool,
    }
}

theme_struct! {
    /// TelaPerfil: uma barra do gráfico da semana (segunda a domingo).
    pub struct DeskBar {
        pub label: String,
        // Só hoje tem o valor escrito.
        pub value: String,
        // Altura da barra, de 0 a 1 (o dia com mais tempo = 1).
        pub fraction: f32,
        pub today: bool,
        // Dia que ainda não chegou (tracejado).
        pub future: bool,
    }
}

theme_struct! {
    /// TelaPerfil: linha com capa (mais jogados, sessões, capas das coleções) ou cabeçalho de dia (`header`).
    pub struct DeskEntry {
        pub id: String,
        pub title: String,
        pub detail: String,
        // Texto à direita antes do valor ("42 h no jogo").
        pub note: String,
        pub value: String,
        pub thumb: Image,
        pub accent: Color,
        // Trilho proporcional (mais jogados), de 0 a 1.
        pub fraction: f32,
        // Ponto verde (jogando agora).
        pub dot: bool,
        pub header: bool,
        // Guia Sessões: o índice da linha sem os cabeçalhos de dia (o que o tema manda no clique).
        pub row: i32,
    }
}

theme_struct! {
    /// PerfilColecoes: um cartão de coleção (do usuário ou automática).
    pub struct DeskCard {
        pub title: String,
        pub count: String,
        pub auto: bool,
        pub covers: Vec<DeskEntry>,
    }
}

theme_struct! {
    /// TelaPerfil inteira (Visão geral, Sessões e Coleções).
    pub struct DeskProfile {
        pub name: String,
        pub initial: String,
        // Linha de estado: "Perfil principal", "Jogando … · 42 min" (com `dot`), "Jogou por último hoje".
        pub line: Vec<DeskInfo>,
        pub playing: bool,
        pub tabs: Vec<String>,
        pub tab: i32,
        // Ações em ícone: ícone e nome (o nome aparece no hover e no foco).
        pub actions: Vec<DeskInfo>,
        // Blocos: rótulo e valor (Tempo jogado, Mais jogado, Jogos na biblioteca).
        pub tiles: Vec<DeskInfo>,
        pub week_label: String,
        pub week_total: String,
        pub week: Vec<DeskBar>,
        // Leitura do gráfico para o leitor de tela ("segunda 40 min, …").
        pub week_text: String,
        pub top: Vec<DeskEntry>,
        pub fan: Vec<DeskEntry>,
        pub recent_title: String,
        pub see_all: String,
        pub recent: Vec<DeskEntry>,
        pub empty: String,
        pub periods: Vec<String>,
        pub period: i32,
        pub summary: String,
        pub sessions: Vec<DeskEntry>,
        pub cards_summary: String,
        pub cards: Vec<DeskCard>,
        pub auto_label: String,
        pub new_label: String,
        pub new_text: String,
        pub zone: String,
        pub index: i32,
    }
}
