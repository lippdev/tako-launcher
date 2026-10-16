//! Structs da API de tema v1, iguais às exportadas por `theme/main.slint`.

use crate::theme_struct;
use slint::{Color, Image};

theme_struct! {
    pub struct Hint { pub glyph: String, pub label: String }
}

theme_struct! {
    /// Linha de painel, configuração ou diálogo. `kind`: toggle, choice, action, info, header, radio, check,
    /// slider, text, button. `style`: "", "primary" ou "danger".
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
    pub fn danger(mut self) -> Self {
        self.style = "danger".into();
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
        self.kind != "header"
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
    /// Camada sobre a página. `kind`: dialog, panel, keyboard, art, launch, progress, recovery.
    pub struct Overlay {
        pub kind: String,
        pub title: String,
        pub overline: String,
        pub text: String,
        pub rows: Vec<PanelRow>,
        pub sel: i32,
        pub wide: bool,
        pub image: Image,
        pub keys: Vec<String>,
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
