//! Como aplicar a atualização baixada, como dados: quem executa é a UI (o processo sai logo depois).
//!
//! Um executável em uso não pode ser trocado com segurança; por isso, fora o instalador do Windows,
//! um script espera o Tako (UI e supervisor) sair, troca os arquivos e, se pedido, abre de novo.

use std::path::{Path, PathBuf};

use super::feed::Package;

/// Processo a iniciar (desanexado) antes de o Tako sair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateApplyPlan {
    /// Script a gravar antes de rodar `program` (`None` para o instalador).
    pub script: Option<(PathBuf, String)>,
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl UpdateApplyPlan {
    /// `target` depende do pacote: o `.exe` (Windows portátil), o arquivo `.AppImage`, a pasta do
    /// `.tar.gz` ou o bundle `.app`. `None` para `.deb` (gerenciador de pacotes).
    pub fn build(
        package: Package,
        downloaded: &Path,
        target: &Path,
        wait_for: &[u32],
        relaunch: bool,
    ) -> Option<UpdateApplyPlan> {
        let sh = |body: String| {
            let script = downloaded.with_file_name("swap.sh");
            UpdateApplyPlan {
                program: PathBuf::from("/bin/sh"),
                args: vec![script.to_string_lossy().into_owned()],
                script: Some((script, unix_swap_script(wait_for, &body))),
            }
        };
        let (new, dst) = (q(downloaded), q(target));
        match package {
            Package::WindowsSetup => Some(UpdateApplyPlan {
                script: None,
                program: downloaded.to_path_buf(),
                args: installer_args(relaunch),
            }),
            Package::WindowsPortable => {
                let script = downloaded.with_file_name("swap.cmd");
                Some(UpdateApplyPlan {
                    program: PathBuf::from("cmd.exe"),
                    args: vec!["/c".into(), script.to_string_lossy().into_owned()],
                    script: Some((
                        script,
                        portable_swap_script(
                            &downloaded.to_string_lossy(),
                            &target.to_string_lossy(),
                            wait_for,
                            relaunch,
                        ),
                    )),
                })
            }
            Package::LinuxAppImage => {
                let backup = q(&target.with_extension("update-old"));
                let failed = q(&downloaded.with_file_name("failed"));
                Some(sh(format!(
                    "chmod +x {new} && cp -p {dst} {backup} && mv -f {new} {dst}\nif [ $? -ne 0 ]; then [ ! -f {backup} ] || mv -f {backup} {dst}; touch {failed}; else rm -f {backup}; fi\n{}",
                    relaunch_line(relaunch, &format!("{dst} >/dev/null 2>&1 &"))
                )))
            }
            // A pasta do .tar.gz guarda o `Tako_Data` do portátil: copia por cima em vez de trocar a pasta.
            Package::LinuxTarGz => Some(sh(format!(
                "staging=\"$(mktemp -d)\"\ntar -xzf {new} -C \"$staging\" || exit 1\n\
                 set -- \"$staging\"/*\nif [ $# -eq 1 ] && [ -d \"$1\" ]; then src=\"$1\"; else src=\"$staging\"; fi\n\
                 cp -Rf \"$src\"/. {dst}/ && rm -rf \"$staging\" {new}\n{}",
                relaunch_line(relaunch, &format!("{dst}/tako >/dev/null 2>&1 &"))
            ))),
            // O zip traz `Tako.app`; o antigo só sai depois que o novo está no lugar.
            Package::MacApp => {
                let old = q(&target.with_extension("app.old"));
                let failed = q(&downloaded.with_file_name("failed"));
                Some(sh(format!(
                    "staging=\"$(mktemp -d)\"\nditto -x -k {new} \"$staging\" || exit 1\n\
                     set -- \"$staging\"/*.app\n[ -d \"$1\" ] || exit 1\n\
                     rm -rf {old}\nmv {dst} {old} && mv \"$1\" {dst} && rm -rf {old} \"$staging\" {new} \\\n\
                     || {{ [ -d {dst} ] || mv {old} {dst}; touch {failed}; }}\n{}",
                    relaunch_line(relaunch, &format!("open {dst}"))
                )))
            }
            Package::LinuxDeb => None,
        }
    }
}

/// Inno Setup: fecha o Tako pelo Restart Manager e o reabre ao terminar (menos com `/NOLAUNCH=1`).
pub fn installer_args(relaunch: bool) -> Vec<String> {
    let mut args: Vec<String> = [
        "/SILENT",
        "/SUPPRESSMSGBOXES",
        "/NORESTART",
        "/CLOSEAPPLICATIONS",
        "/FORCECLOSEAPPLICATIONS",
    ]
    .map(String::from)
    .into();
    if !relaunch {
        args.push("/NOLAUNCH=1".into());
    }
    args
}

/// Port de `PortableSwapScript.Build`: `.cmd` que espera os PIDs, move o novo `.exe` por cima e reabre.
pub fn portable_swap_script(
    downloaded: &str,
    executable: &str,
    wait_for: &[u32],
    relaunch: bool,
) -> String {
    let mut s = String::from("@echo off\r\n");
    for (label, pid) in dedup(wait_for).iter().enumerate() {
        // ping como espera: timeout.exe falha sem console interativo.
        s.push_str(&format!(
            ":wait{label}\r\ntasklist /fi \"PID eq {pid}\" | find \"{pid}\" >nul && (ping -n 2 127.0.0.1 >nul & goto wait{label})\r\n"
        ));
    }
    s.push_str(&format!(
        "copy /y \"{executable}\" \"{executable}.update-old\" >nul\r\nif errorlevel 1 goto failed\r\nmove /y \"{downloaded}\" \"{executable}\" >nul\r\nif errorlevel 1 (copy /y \"{executable}.update-old\" \"{executable}\" >nul & goto failed)\r\ndel \"{executable}.update-old\" >nul 2>&1\r\ngoto done\r\n:failed\r\ntype nul > \"%~dp0failed\"\r\n:done\r\n"
    ));
    if relaunch {
        s.push_str(&format!("start \"\" \"{executable}\"\r\n"));
    }
    s
}

fn unix_swap_script(wait_for: &[u32], body: &str) -> String {
    let mut s = String::from("#!/bin/sh\n");
    for pid in dedup(wait_for) {
        s.push_str(&format!(
            "while kill -0 {pid} 2>/dev/null; do sleep 0.5; done\n"
        ));
    }
    s.push_str(body);
    s
}

fn relaunch_line(relaunch: bool, cmd: &str) -> String {
    if relaunch {
        format!("{cmd}\n")
    } else {
        String::new()
    }
}

fn dedup(pids: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for p in pids {
        if !out.contains(p) {
            out.push(*p);
        }
    }
    out
}

/// Aspas simples do shell: o caminho vira um único argumento literal.
fn q(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', r"'\''"))
}

/// Bundle `.app` que contém o executável (`X.app/Contents/MacOS/tako`).
pub fn mac_bundle_of(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_swap_waits_for_every_process_then_moves_and_relaunches() {
        let script = portable_swap_script(
            r"C:\Temp\TakoUpdate\Tako-Portable-x64.exe",
            r"D:\Jogos\Tako.exe",
            &[4242, 77, 4242],
            true,
        );
        let lines: Vec<_> = script.split("\r\n").filter(|l| !l.is_empty()).collect();
        assert_eq!(lines[0], "@echo off");
        assert!(lines[2].contains("PID eq 4242"));
        assert!(lines[2].contains("goto wait0"));
        assert!(lines[4].contains("PID eq 77"));
        assert!(lines[4].contains("goto wait1"));
        // Copia a versão atual antes de trocar; se a troca falha, restaura e deixa `failed` para o Tako avisar.
        assert!(
            lines[5].starts_with(r#"copy /y "D:\Jogos\Tako.exe" "D:\Jogos\Tako.exe.update-old""#)
        );
        assert_eq!(
            lines[7],
            r#"move /y "C:\Temp\TakoUpdate\Tako-Portable-x64.exe" "D:\Jogos\Tako.exe" >nul"#
        );
        assert!(lines[8].contains(r#"copy /y "D:\Jogos\Tako.exe.update-old" "D:\Jogos\Tako.exe""#));
        assert!(script.contains("type nul > \"%~dp0failed\""));
        assert_eq!(lines.last().unwrap(), &r#"start "" "D:\Jogos\Tako.exe""#);
        assert!(!portable_swap_script("a.exe", "b.exe", &[1], false).contains("start"));
    }

    #[test]
    fn installer_gets_silent_args() {
        let p = UpdateApplyPlan::build(
            Package::WindowsSetup,
            Path::new("T/Tako-Setup-x64.exe"),
            Path::new("ignored"),
            &[1],
            false,
        )
        .unwrap();
        assert_eq!(p.program, Path::new("T/Tako-Setup-x64.exe"));
        assert!(p.script.is_none());
        assert_eq!(
            p.args.join(" "),
            "/SILENT /SUPPRESSMSGBOXES /NORESTART /CLOSEAPPLICATIONS /FORCECLOSEAPPLICATIONS /NOLAUNCH=1"
        );
        assert!(!installer_args(true).contains(&"/NOLAUNCH=1".to_owned()));
    }

    #[test]
    fn windows_portable_runs_the_cmd_script() {
        let p = UpdateApplyPlan::build(
            Package::WindowsPortable,
            Path::new("T/Tako-Portable-x64.exe"),
            Path::new("D/Tako.exe"),
            &[5, 6],
            true,
        )
        .unwrap();
        let (path, body) = p.script.unwrap();
        assert_eq!(path, Path::new("T/swap.cmd"));
        assert_eq!(p.program, Path::new("cmd.exe"));
        assert_eq!(p.args, ["/c", &*path.to_string_lossy()]);
        assert!(body.contains("PID eq 6") && body.contains("start"));
    }

    #[test]
    fn appimage_swap_waits_quotes_and_replaces() {
        let p = UpdateApplyPlan::build(
            Package::LinuxAppImage,
            Path::new("/tmp/TakoUpdate/Tako-Linux-x64.AppImage"),
            Path::new("/home/a/Apps/Tako's.AppImage"),
            &[10, 10, 11],
            true,
        )
        .unwrap();
        let (path, body) = p.script.unwrap();
        assert_eq!(path, Path::new("/tmp/TakoUpdate/swap.sh"));
        assert_eq!(p.program, Path::new("/bin/sh"));
        assert_eq!(body.matches("kill -0 10 ").count(), 1);
        assert!(body.contains("kill -0 11 "));
        assert!(body.contains(
            "mv -f '/tmp/TakoUpdate/Tako-Linux-x64.AppImage' '/home/a/Apps/Tako'\\''s.AppImage'"
        ));
        assert!(body.trim_end().ends_with("&"));
    }

    #[test]
    fn tar_gz_copies_over_keeping_data_and_mac_swaps_the_bundle() {
        let tar = UpdateApplyPlan::build(
            Package::LinuxTarGz,
            Path::new("/tmp/u/Tako-Linux-x64.tar.gz"),
            Path::new("/opt/tako"),
            &[1],
            false,
        )
        .unwrap();
        let body = tar.script.unwrap().1;
        assert!(body.contains("tar -xzf '/tmp/u/Tako-Linux-x64.tar.gz'"));
        assert!(body.contains("cp -Rf \"$src\"/. '/opt/tako'/"));
        assert!(!body.contains("rm -rf '/opt/tako'"));
        assert!(!body.contains("/opt/tako'/tako"));

        let mac = UpdateApplyPlan::build(
            Package::MacApp,
            Path::new("/tmp/u/Tako-macOS-arm64.zip"),
            Path::new("/Applications/Tako.app"),
            &[1],
            true,
        )
        .unwrap();
        let body = mac.script.unwrap().1;
        assert!(body.contains("ditto -x -k"));
        assert!(body.contains("mv '/Applications/Tako.app' '/Applications/Tako.app.old'"));
        assert!(body.contains("open '/Applications/Tako.app'"));
    }

    #[test]
    fn deb_has_no_plan_and_bundle_is_found() {
        assert!(
            UpdateApplyPlan::build(Package::LinuxDeb, Path::new("a"), Path::new("b"), &[], true)
                .is_none()
        );
        assert_eq!(
            mac_bundle_of(Path::new("/Applications/Tako.app/Contents/MacOS/tako")),
            Some(PathBuf::from("/Applications/Tako.app"))
        );
        assert_eq!(mac_bundle_of(Path::new("/usr/bin/tako")), None);
    }
}
