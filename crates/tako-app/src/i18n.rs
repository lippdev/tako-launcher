//! Textos e idiomas para a interface web: o catálogo do idioma atual sai daqui, nunca de dentro dos componentes.

use crate::{Core, protocol::*};

/// O idioma atual do núcleo e o catálogo inteiro dele, no formato de `tako_core::i18n`.
pub(crate) fn catalog(core: &Core) -> I18nCatalog {
    let lang = core.language();
    I18nCatalog {
        lang: lang.code().to_owned(),
        messages: tako_core::i18n::messages(lang)
            .iter()
            .map(|(key, text)| (key.clone(), text.clone()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppService;
    use std::sync::mpsc;
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    fn service(dir: &tempfile::TempDir) -> AppService {
        let core = Core::open_with_sources(
            DataPaths::at(dir.path()),
            Platform::simulated(),
            true,
            Vec::new(),
        )
        .unwrap();
        AppService::with_core("test", core)
    }

    fn catalog_of(app: &mut AppService) -> I18nCatalog {
        match app.handle(Call::I18nCatalog).unwrap() {
            Reply::I18nCatalog(catalog) => catalog,
            other => panic!("resposta inesperada: {other:?}"),
        }
    }

    fn set_language(app: &mut AppService, value: &str) -> Result<Reply, BridgeError> {
        app.handle(Call::SettingsSet(Setting {
            key: "Language".into(),
            value: value.into(),
        }))
    }

    #[test]
    fn catalog_is_flat_and_keeps_placeholders_and_plurals() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = service(&dir);
        set_language(&mut app, "pt-BR").unwrap();
        let pt = catalog_of(&mut app);
        assert_eq!(pt.lang, "pt-BR");
        assert_eq!(pt.messages["settings.general.title"], "Geral");
        assert_eq!(pt.messages["common.games.one"], "{n} jogo");
        assert_eq!(pt.messages["common.games.other"], "{n} jogos");
        set_language(&mut app, "en-US").unwrap();
        let en = catalog_of(&mut app);
        assert_eq!(en.lang, "en-US");
        assert_eq!(en.messages["settings.general.title"], "General");
        assert_eq!(pt.messages.len(), en.messages.len());
    }

    #[test]
    fn catalog_fits_the_bridge_limit_with_room_to_spare() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = service(&dir);
        for code in ["pt-BR", "en-US"] {
            set_language(&mut app, code).unwrap();
            let reply = Response::ok(1, Reply::I18nCatalog(catalog_of(&mut app)));
            let bytes = serde_json::to_string(&reply).unwrap().len();
            println!("i18n.catalog {code}: {bytes} bytes");
            // Folga de pelo menos 4x: o catálogo cresce com as telas migradas.
            assert!(bytes < MAX_MESSAGE_BYTES / 4, "{code}: {bytes} bytes");
        }
    }

    #[test]
    fn wire_format_of_call_reply_and_event() {
        let request = parse_request(r#"{"v":1,"id":3,"name":"i18n.catalog"}"#).unwrap();
        assert_eq!(request.call, Call::I18nCatalog);
        let with_payload = parse_request(r#"{"v":1,"id":3,"name":"i18n.catalog","payload":{}}"#);
        assert_eq!(with_payload.unwrap_err().1.code, ErrorCode::BadRequest);

        let reply = Response::ok(
            3,
            Reply::I18nCatalog(I18nCatalog {
                lang: "pt-BR".into(),
                messages: [("a.b".to_owned(), "Olá {nome}".to_owned())].into(),
            }),
        );
        assert_eq!(
            serde_json::to_string(&reply).unwrap(),
            r#"{"v":1,"id":3,"ok":true,"reply":{"name":"i18n.catalog","data":{"lang":"pt-BR","messages":{"a.b":"Olá {nome}"}}}}"#
        );
        let event = EventEnvelope::from(Event::I18nChanged(I18nLang {
            lang: "en-US".into(),
        }));
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"v":1,"event":{"name":"i18n.changed","payload":{"lang":"en-US"}}}"#
        );
    }

    #[test]
    fn changing_language_emits_the_event_once_and_only_when_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = service(&dir);
        set_language(&mut app, "pt-BR").unwrap();
        let (tx, rx) = mpsc::channel();
        app.set_event_sink(tx);

        set_language(&mut app, "pt-BR").unwrap();
        assert!(rx.try_recv().is_err(), "mesmo idioma não avisa");

        set_language(&mut app, "en_US").unwrap();
        assert_eq!(
            rx.try_recv().unwrap(),
            Event::I18nChanged(I18nLang {
                lang: "en-US".into()
            })
        );
        assert!(rx.try_recv().is_err(), "um aviso só");
        assert_eq!(catalog_of(&mut app).lang, "en-US");

        assert!(set_language(&mut app, "klingon").is_err());
        app.handle(Call::SettingsSet(Setting {
            key: "ReducedMotion".into(),
            value: "true".into(),
        }))
        .unwrap();
        assert!(rx.try_recv().is_err(), "falha e outra chave não avisam");
    }

    #[test]
    fn catalog_without_core_is_an_internal_error() {
        let mut app = AppService::new("1");
        assert_eq!(
            app.handle(Call::I18nCatalog).unwrap_err().code,
            ErrorCode::Internal
        );
    }
}
