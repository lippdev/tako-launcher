//! Textos da interface em português do Brasil e inglês (EUA) (PRD §6).
//!
//! Catálogos em `locales/<idioma>/<módulo>.toml`, chaves planas (`"library.count" = "{n} jogos"`).
//! Plural: `chave.one` e `chave.other`, escolhidos por [`tn`]. Chave ausente no idioma cai no pt-BR
//! e, se faltar lá também, a própria chave aparece (o teste `catalogs_match` impede isso no CI).

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    PtBr,
    EnUs,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::PtBr => "pt-BR",
            Lang::EnUs => "en-US",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let c = code.trim().to_ascii_lowercase().replace('_', "-");
        if c.starts_with("pt") {
            Some(Lang::PtBr)
        } else if c.starts_with("en") {
            Some(Lang::EnUs)
        } else {
            None
        }
    }

    /// Idioma do sistema (variáveis `LC_ALL`, `LC_MESSAGES`, `LANG`; no Windows e no macOS a UI passa o do SO).
    /// Sem pista, português.
    pub fn from_env() -> Lang {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .find_map(|v| Lang::from_code(&v))
            .unwrap_or(Lang::PtBr)
    }
}

type Catalog = HashMap<String, String>;

fn parse(files: &[(&str, &str)]) -> Catalog {
    let mut out = Catalog::new();
    for (name, src) in files {
        let table: toml::Table =
            toml::from_str(src).unwrap_or_else(|e| panic!("catálogo inválido {name}: {e}"));
        flatten("", &table, &mut out);
    }
    out
}

fn flatten(prefix: &str, t: &toml::Table, out: &mut Catalog) {
    for (k, v) in t {
        let key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            toml::Value::String(s) => {
                out.insert(key, s.clone());
            }
            toml::Value::Table(t) => flatten(&key, t, out),
            other => panic!("valor não textual em {key}: {other}"),
        }
    }
}

fn catalogs() -> &'static (Catalog, Catalog) {
    static C: OnceLock<(Catalog, Catalog)> = OnceLock::new();
    C.get_or_init(|| {
        (
            parse(include!(concat!(env!("OUT_DIR"), "/pt-BR.rs"))),
            parse(include!(concat!(env!("OUT_DIR"), "/en-US.rs"))),
        )
    })
}

fn current() -> &'static RwLock<Lang> {
    static L: OnceLock<RwLock<Lang>> = OnceLock::new();
    L.get_or_init(|| RwLock::new(Lang::PtBr))
}

pub fn set_lang(lang: Lang) {
    *current().write().unwrap_or_else(|e| e.into_inner()) = lang;
}

pub fn lang() -> Lang {
    *current().read().unwrap_or_else(|e| e.into_inner())
}

/// Texto da chave no idioma atual.
pub fn t(key: &str) -> String {
    t_in(lang(), key)
}

pub fn t_in(lang: Lang, key: &str) -> String {
    let (pt, en) = catalogs();
    let primary = if lang == Lang::EnUs { en } else { pt };
    primary
        .get(key)
        .or_else(|| pt.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

/// Texto com parâmetros: `{nome}` é trocado pelo valor.
pub fn tf(key: &str, args: &[(&str, &str)]) -> String {
    fill(t(key), args)
}

/// Plural: usa `chave.one` quando `n == 1`, senão `chave.other`; `{n}` é preenchido.
pub fn tn(key: &str, n: i64, args: &[(&str, &str)]) -> String {
    let form = if n == 1 { "one" } else { "other" };
    let n_text = n.to_string();
    let mut all = vec![("n", n_text.as_str())];
    all.extend_from_slice(args);
    fill(t(&format!("{key}.{form}")), &all)
}

fn fill(mut s: String, args: &[(&str, &str)]) -> String {
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// Todas as chaves (para a ponte com o tema e para os testes).
pub fn keys() -> impl Iterator<Item = &'static String> {
    catalogs().0.keys()
}

/// O catálogo inteiro de um idioma, `chave completa → texto`, como está nos `.toml`: `{nome}` nos parâmetros e
/// `chave.one`/`chave.other` nos plurais. Sem o recuo ao pt-BR de [`t_in`] (o `catalogs_match` garante que os dois
/// idiomas têm as mesmas chaves). É o que a interface web recebe pela ponte.
pub fn messages(lang: Lang) -> &'static HashMap<String, String> {
    let (pt, en) = catalogs();
    if lang == Lang::EnUs { en } else { pt }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_match() {
        let (pt, en) = catalogs();
        let mut missing: Vec<_> = pt.keys().filter(|k| !en.contains_key(*k)).collect();
        missing.extend(en.keys().filter(|k| !pt.contains_key(*k)));
        missing.sort();
        assert!(missing.is_empty(), "chaves sem tradução: {missing:?}");
    }

    #[test]
    fn messages_expose_the_whole_catalog_of_each_language() {
        let (pt, en) = (messages(Lang::PtBr), messages(Lang::EnUs));
        assert_eq!(pt.len(), en.len());
        assert_eq!(pt["settings.general.title"], "Geral");
        assert_eq!(en["settings.general.title"], "General");
        assert_eq!(pt["common.games.one"], "{n} jogo");
        assert_eq!(en["common.games.other"], "{n} games");
    }

    #[test]
    fn falls_back_and_fills() {
        assert_eq!(t_in(Lang::EnUs, "não.existe"), "não.existe");
        assert_eq!(fill("{a} e {b}".into(), &[("a", "1"), ("b", "2")]), "1 e 2");
        assert_eq!(Lang::from_code("en_US.UTF-8"), Some(Lang::EnUs));
        assert_eq!(Lang::from_code("pt_BR"), Some(Lang::PtBr));
        assert_eq!(Lang::from_code("C"), None);
    }

    #[test]
    fn step_instructions_are_multiline_and_numbered() {
        for lang in [Lang::PtBr, Lang::EnUs] {
            for key in [
                "settings.metadata.igdb_steps",
                "settings.metadata.ss_steps",
                "settings.metadata.sgdb_steps",
            ] {
                let val = t_in(lang, key);
                let lines: Vec<&str> = val.lines().collect();
                assert!(
                    lines.len() >= 3,
                    "{key} in {lang:?} should have >=3 lines, got {val:?}"
                );
                for (i, line) in lines.iter().enumerate() {
                    let expected_prefix = format!("{}.", i + 1);
                    assert!(
                        line.starts_with(&expected_prefix),
                        "{key} in {lang:?} line {} should start with '{expected_prefix}': {line}",
                        i + 1
                    );
                }
            }
        }
    }

    #[test]
    fn plural_forms() {
        set_lang(Lang::PtBr);
        assert_eq!(tn("common.games", 1, &[]), "1 jogo");
        assert_eq!(tn("common.games", 3, &[]), "3 jogos");
        assert_eq!(
            t_in(Lang::EnUs, "common.games.other").replace("{n}", "3"),
            "3 games"
        );
    }
}
