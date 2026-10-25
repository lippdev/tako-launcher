//! Regras de editar um jogo adicionado à mão (MVP-024): nome, argumentos, pasta de trabalho e fim do jogo. O alvo
//! (executável, atalho ou URI) não muda: trocar o arquivo é adicionar outro jogo. Usada pela interface Slint e
//! pela ponte da interface web.

use super::{GameTrackingStrategy, ManualGameRecord, manual};
use std::path::Path;

/// O que o usuário preenche ao editar. Texto vazio = sem valor; a pasta vazia volta à pasta do executável.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDraft {
    pub title: String,
    pub arguments: String,
    pub folder: String,
    /// `None` = automático (processo iniciado).
    pub track_by: Option<GameTrackingStrategy>,
}

impl EditDraft {
    /// O rascunho que mostra o que o jogo já tem.
    pub fn of(record: &ManualGameRecord) -> EditDraft {
        EditDraft {
            title: record.title.clone(),
            arguments: record.arguments.clone().unwrap_or_default(),
            folder: record.working_directory.clone().unwrap_or_default(),
            track_by: record.track_by,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("o jogo precisa de um nome")]
    NameRequired,
    /// A pasta de trabalho digitada não existe (o texto é a pasta, já sem espaços nas pontas).
    #[error("pasta não encontrada: {0}")]
    FolderMissing(String),
}

/// A pasta que não existe não é aceita; vazio = a pasta do executável.
pub fn folder_missing(folder: &str) -> bool {
    let f = folder.trim();
    !f.is_empty() && !Path::new(f).is_dir()
}

/// A pasta do executável, que vale quando a de trabalho está vazia (URIs ficam sem pasta).
pub fn default_folder(path: &str) -> Option<String> {
    (!manual::is_uri(path))
        .then(|| Path::new(path).parent())
        .flatten()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty())
}

impl ManualGameRecord {
    /// O registro com o rascunho aplicado; o alvo, o id e as imagens ficam como estão. Valida o nome e a pasta.
    pub fn edited(&self, draft: &EditDraft) -> Result<ManualGameRecord, EditError> {
        let title = draft.title.trim();
        let folder = draft.folder.trim();
        if title.is_empty() {
            return Err(EditError::NameRequired);
        }
        if folder_missing(folder) {
            return Err(EditError::FolderMissing(folder.to_owned()));
        }
        Ok(ManualGameRecord {
            title: title.to_owned(),
            arguments: Some(draft.arguments.trim().to_owned()).filter(|a| !a.is_empty()),
            working_directory: if folder.is_empty() {
                default_folder(&self.path)
            } else {
                Some(folder.to_owned())
            },
            track_by: draft.track_by,
            ..self.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(path: &str) -> ManualGameRecord {
        ManualGameRecord::create("Jogo", path, "2026-10-24T12:00:00Z".parse().unwrap())
    }

    fn draft(title: &str, folder: &str) -> EditDraft {
        EditDraft {
            title: title.into(),
            arguments: String::new(),
            folder: folder.into(),
            track_by: None,
        }
    }

    #[test]
    fn edit_applies_the_draft_and_keeps_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("jogo.exe");
        let r = record(&exe.to_string_lossy());
        let d = EditDraft {
            arguments: "  -windowed  ".into(),
            track_by: Some(GameTrackingStrategy::Process),
            ..draft("  Meu Jogo ", "")
        };
        let out = r.edited(&d).unwrap();
        assert_eq!(out.title, "Meu Jogo");
        assert_eq!(out.arguments.as_deref(), Some("-windowed"));
        assert_eq!(out.track_by, Some(GameTrackingStrategy::Process));
        assert_eq!(out.path, r.path);
        assert_eq!(out.id, r.id);
        // Pasta vazia: a do executável.
        assert_eq!(
            out.working_directory.as_deref(),
            Some(&*dir.path().to_string_lossy())
        );
        assert_eq!(EditDraft::of(&out).arguments, "-windowed");
    }

    #[test]
    fn edit_rejects_a_blank_name_and_a_missing_folder() {
        let r = record("/jogos/a.sh");
        assert_eq!(
            r.edited(&draft("  ", "")).unwrap_err(),
            EditError::NameRequired
        );
        assert_eq!(
            r.edited(&draft("X", " /nao/existe ")).unwrap_err(),
            EditError::FolderMissing("/nao/existe".into())
        );
        // O nome é conferido antes da pasta, como no Slint.
        assert_eq!(
            r.edited(&draft("", "/nao/existe")).unwrap_err(),
            EditError::NameRequired
        );
    }

    #[test]
    fn uris_have_no_default_folder() {
        assert_eq!(default_folder("steam://rungameid/570"), None);
        let out = record("steam://rungameid/570")
            .edited(&draft("Steam", ""))
            .unwrap();
        assert_eq!(out.working_directory, None);
    }
}
