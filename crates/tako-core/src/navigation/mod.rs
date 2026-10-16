//! Navegação pelo controle (port de `Launcher.Core/Navigation`, MIG-013): vizinho espacial e rolagem que segue o foco.

pub mod scroll;
pub mod spatial;

pub use scroll::{row_reveal_offset, vertical_scroll};
pub use spatial::{NavCandidate, NavDirection, NavRect, find_next};
