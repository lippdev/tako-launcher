# ADR-0024: Matriz de plataforma (telas, áudio, cursor, processos)

- Status: Proposta
- Data: 2026-10-16

## Contexto

O MIG-002 pede a matriz real das funções de D-03 (PRD §5.3) em cada SO, antes de virar promessa ao usuário. O risco principal (PRD §11) é o Wayland: não há API comum para esconder o cursor nem para trocar a resolução, e cada compositor expõe a sua. No macOS, mover o cursor exige a permissão de Acessibilidade.

O comportamento esperado vem do C# do Windows (`ConsoleMode.Runtime.Windows`): capturar o estado antes do jogo, aplicar o pedido, restaurar depois, e restaurar duas vezes não pode desfazer nada a mais.

## Decisão

1. **Um modelo comum de telas para Linux e macOS** (`tako-platform/src/unix/outputs.rs`). Cada backend só lê as saídas e aplica um estado completo; achar o modo pela taxa mais próxima, ligar e desligar, reposicionar, capturar e restaurar ficam numa lógica única, testada sem SO. Proteções: nunca aplicar um estado sem nenhuma tela ligada; um registro de outra mesa (nenhuma tela conhecida) é recusado em vez de desligar tudo.
2. **Backend de telas escolhido pela sessão** (`XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `DISPLAY`, `XDG_CURRENT_DESKTOP`, `GAMESCOPE_WAYLAND_DISPLAY`):
   - X11: XRandR pelo `x11rb` (Rust puro, sem libxcb), como o `xrandr`: desliga os CRTCs que mudam, ajusta o tamanho da tela e liga de novo, com o servidor travado.
   - KDE Wayland: `kscreen-doctor` (`-j` para ler; `output.<nome>.mode/position/rotation/enable/disable/primary` para aplicar, numa chamada só). HDR por `output.<nome>.hdr.enable` quando o JSON informa `hdr`.
   - GNOME Wayland: Mutter `org.gnome.Mutter.DisplayConfig` pelo `zbus` (`GetCurrentState`, `ApplyMonitorsConfig` com método 1 = temporário, sem gravar no `monitors.xml`). A escala atual é mantida se o novo modo a aceitar.
   - wlroots (Sway, Hyprland, river...): `wlr-randr --json` e `wlr-randr --output ...`. Não existe tela principal.
   - gamescope: só leitura pelo Xwayland da sessão; `is_managed_by_session() == true` e os setters devolvem `Unsupported`.
   - Outro compositor Wayland ou sem sessão: tudo `Unsupported`, sem pânico.
3. **Áudio no Linux** por `pactl -f json` (ids = nomes dos sinks, estáveis); sem `pactl` (ou `pactl` antigo, sem JSON), `wpctl`. Ferramenta de linha de comando em vez de `libpulse`: sem dependência C e a mesma coisa funciona em PulseAudio e PipeWire.
4. **Cursor**: X11 pelo XFixes numa conexão que vive enquanto o cursor está escondido (se o Tako cair, o servidor X mostra o cursor sozinho). Wayland: não há API global; `set_hidden` responde Ok, a UI esconde sobre a própria janela e `limitation()` explica ao usuário. macOS: `CGDisplayHideCursor`. Em todos, o marcador `<dados>/cursor-hidden` registra o estado para a recuperação.
5. **Cursor pelo controle**: Linux por um mouse virtual no `/dev/uinput` (crate `evdev`), que funciona em X11, Wayland e gamescope; o pacote instala `packaging/linux/60-tako-uinput.rules` (`uaccess`). macOS por `CGEventPost` no `kCGHIDEventTap`; `check_access` usa `CGPreflightPostEventAccess` (a mesma permissão de Acessibilidade do `AXIsProcessTrusted`, sem ligar o ApplicationServices) e orienta o usuário.
6. **Processos**: Linux por `/proc` (pai, executável, início por `starttime` + `btime`), executáveis e scripts direto, `.desktop` por `gio launch` (sem ele, `xdg-open`), URIs por `xdg-open`. macOS por `libproc` (`proc_listallpids`, `proc_pidinfo`, `proc_pidpath`) e `open` para `.app`, URIs e arquivos. Fechar = SIGTERM. Steam: `RunningAppID` do `registry.vdf`. Um processo aberto pelo Tako é esperado numa thread, para não virar zumbi e parecer rodando.
7. **Abrir com o sistema**: `~/.config/autostart/tako.desktop` (num AppImage, o caminho de `$APPIMAGE`) e `~/Library/LaunchAgents/dev.lipp.tako.plist`.

### Matriz de suporte

Legenda: **Sim** = implementado e testado na lógica; **a validar** = implementado, falta prova em hardware (checklists em `docs/testing/`); **Não** = o SO ou o compositor não oferece.

| Função | Windows | X11 | KDE Wayland | GNOME Wayland | wlroots | gamescope | macOS |
|---|---|---|---|---|---|---|---|
| Listar telas e modos | Sim (CCD, C#) | a validar | a validar | a validar | a validar (`wlr-randr` com `--json`) | a validar (só a da sessão) | a validar |
| Trocar resolução e taxa | Sim | a validar | a validar | a validar | a validar | Não (da sessão) | a validar |
| Ligar e desligar telas | Sim | a validar | a validar | a validar | a validar | Não | Não (só ligar o que já está ligado) |
| Tela principal | Sim | a validar | a validar | a validar | Não existe | Não | a validar (a da origem) |
| Rotação na restauração | Sim | a validar | a validar | a validar | a validar | Não | Não |
| HDR | Sim | Não | a validar (Plasma 6) | Não | Não | Não | Não |
| VRR (PERF-002) | Não: sem API pública | Não: é do driver (`VariableRefresh`) | a validar (`vrrpolicy`) | Não (ainda) | a validar (`--adaptive-sync`) | Não | Não: sem API pública |
| Capturar e restaurar telas | Sim | a validar | a validar | a validar | a validar | Não precisa | a validar |
| Saída padrão, volume e mudo | Sim (`IPolicyConfig`) | a validar (`pactl`/`wpctl`) | igual ao X11 | igual ao X11 | igual ao X11 | a validar | a validar (CoreAudio) |
| Esconder o cursor sobre o jogo | Sim | a validar (XFixes) | Não: só sobre a janela do Tako | Não: idem | Não: idem | a validar (XFixes no Xwayland) | a validar com o jogo na frente |
| Cursor pelo controle | Sim (`SendInput`) | a validar (`uinput` + regra udev) | idem | idem | idem | idem | a validar (Acessibilidade) |
| Abrir jogos e acompanhar | Sim | a validar | idem | idem | idem | idem | a validar |
| Abrir com o sistema | Sim | a validar (XDG autostart) | idem | idem | idem | Não se aplica | a validar (LaunchAgent) |

### Limitações mostradas ao usuário

- Wayland (KDE, GNOME, wlroots): "No Wayland, o Tako só consegue esconder o cursor sobre a própria janela. Sobre o jogo, depende do jogo." (`platform.unix.cursor_wayland`).
- macOS: "No macOS, o cursor pode voltar a aparecer quando outro app fica na frente." (`platform.unix.cursor_macos`).
- Sem acesso ao `/dev/uinput`: orientação para instalar a regra udev e entrar de novo na sessão (`platform.unix.uinput_denied`).
- macOS sem Acessibilidade: caminho em Ajustes do Sistema (`platform.unix.accessibility_denied`).
- gamescope: a resolução é da sessão (`is_managed_by_session`); a UI só informa.
- Compositor Wayland desconhecido: telas `Unsupported`; a UI esconde as opções de tela.

## Consequências

- Os parsers (`kscreen-doctor`, `wlr-randr`, `pactl`, `wpctl`, `/proc/<pid>/stat`, `registry.vdf`) e a lógica de telas são puros e testados em qualquer SO; o código que fala com o SO só foi compilado (Linux) ou checado (`cargo clippy --target aarch64-apple-darwin`). Nada disso foi provado em hardware: a matriz acima fica "a validar" até os checklists rodarem.
- Atalhos assumidos: reposicionar ao ligar telas coloca todas lado a lado (pilhas verticais viram horizontais); no GNOME, espelhamento vira telas separadas; com o `wpctl` os ids de áudio mudam entre sessões (a saída por jogo só é estável com o `pactl`); no macOS o id da tela é fabricante-modelo-série (duas telas iguais sem série colidem).
- `kscreen-doctor`, `wlr-randr`, `pactl`/`wpctl` e `gio`/`xdg-open` são dependências de execução, não de compilação: se faltarem, a função responde `Unsupported` com o nome da ferramenta.
- O pacote Linux precisa instalar a regra udev; sem ela, o cursor pelo controle pede a instalação.
