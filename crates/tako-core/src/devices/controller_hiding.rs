//! Esconder o controle do jogo enquanto a Central está aberta (INPUT-005). Jogos que leem o controle sem estar
//! em foco (XInput, ex.: Resident Evil 2) andariam junto com a Central; com o driver opcional (HidHide no
//! Windows), o supervisor esconde os controles dos outros programas e devolve ao voltar ao jogo.
//!
//! Sem o driver, nada muda.

use super::DeviceResult;

/// Página oficial do HidHide (o Tako não baixa nem executa instaladores: só abre a página).
pub const HIDHIDE_URL: &str = "https://github.com/nefarius/HidHide/releases";

/// Configuração "esconder o controle do jogo com a Central aberta" (padrão ligado; só vale com o driver).
pub const SETTING: &str = "HideControllerFromGame";

pub trait ControllerHiding: Send + Sync {
    /// O driver está instalado e respondendo.
    fn available(&self) -> bool;
    /// O SO tem um driver para isso (só o Windows); nos outros, a opção nem aparece.
    fn supported(&self) -> bool {
        false
    }
    /// Esconde dos outros programas os controles `(vendor, product)` do USB; o Tako continua vendo. Deve
    /// sobreviver a uma queda: a implementação grava o que mudou e [`ControllerHiding::reveal`] desfaz depois.
    fn hide(&self, pads: &[(u16, u16)]) -> DeviceResult<()>;
    /// Desfaz o que [`ControllerHiding::hide`] mudou (só isso; o resto da configuração do driver fica).
    /// Sem nada escondido, não faz nada.
    fn reveal(&self) -> DeviceResult<()>;
    fn is_hidden(&self) -> bool;
}

/// Sem driver (Linux, macOS): nada a esconder.
#[derive(Debug, Default)]
pub struct NoHiding;

impl ControllerHiding for NoHiding {
    fn available(&self) -> bool {
        false
    }
    fn hide(&self, _pads: &[(u16, u16)]) -> DeviceResult<()> {
        Ok(())
    }
    fn reveal(&self) -> DeviceResult<()> {
        Ok(())
    }
    fn is_hidden(&self) -> bool {
        false
    }
}
