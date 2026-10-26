//! "Abrir ao iniciar o Windows": valor `Tako` em `HKCU\...\CurrentVersion\Run` (port de
//! `Services/StartupRegistration.cs`). O registro é a fonte da verdade; a opção na tela só o reflete.

use crate::Autostart;
use crate::windows_startup::{command_for, points_to};
use tako_core::devices::{DeviceError, DeviceResult};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Tako";

pub struct WindowsAutostart;

fn current_exe() -> DeviceResult<String> {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| DeviceError::Failed(format!("caminho do executável: {e}")))
}

impl Autostart for WindowsAutostart {
    /// Ligado só quando o valor aponta para este executável.
    fn is_enabled(&self) -> bool {
        let Ok(exe) = current_exe() else { return false };
        let command = windows_registry::CURRENT_USER
            .open(RUN_KEY)
            .and_then(|k| k.get_string(VALUE_NAME))
            .ok();
        points_to(command.as_deref(), &exe)
    }

    fn set_enabled(&self, enabled: bool) -> DeviceResult<()> {
        let key = windows_registry::CURRENT_USER
            .create(RUN_KEY)
            .map_err(|e| DeviceError::PermissionDenied(format!("{RUN_KEY}: {e}")))?;
        let result = if enabled {
            key.set_string(VALUE_NAME, command_for(&current_exe()?))
        } else {
            match key.remove_value(VALUE_NAME) {
                // ERROR_FILE_NOT_FOUND: já estava desligado.
                Err(e) if e.code() == windows::core::HRESULT::from_win32(2) => Ok(()),
                r => r,
            }
        };
        result.map_err(|e| DeviceError::PermissionDenied(format!("{RUN_KEY}\\{VALUE_NAME}: {e}")))
    }
}
