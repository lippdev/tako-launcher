//! O Windows já fornece PowerShell 5.1; o teste usa somente fakes, sem UAC ou registro real.
#[cfg(windows)]
#[test]
fn fse_state_restoration_with_fake_windows() {
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fse-state.ps1"))
        .output()
        .expect("PowerShell fornecido pelo Windows");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
