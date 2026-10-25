# Tako

<img src="logo/Logo%20TAKO%20com%20fundo%20transparente.png" alt="Tako" width="220" />

Launcher de jogos para Windows 11 com experiência de console: gamepad-first, temas Fullscreen do Playnite 10 e integração com o Windows Full Screen Experience (FSE). Status: **planejamento / Marco 0**.

- Produto e backlog: [`docs/PRD.md`](docs/PRD.md)
- Por onde começar: [`docs/ROADMAP.md`](docs/ROADMAP.md)
- Biblioteca, metadados e artes: [`docs/metadados.md`](docs/metadados.md)
- Criar temas (API de temas e `tako theme check`): [`docs/themes/`](docs/themes/README.md)
- Como contribuir (Git Flow, commits, PRs, DoD, arquitetura): [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Decisões de arquitetura: [`docs/adr/`](docs/adr/)

## Baixar

Na [página de releases](https://github.com/lippdev/ultralauncher/releases), no mesmo formato do Console Mode:
- **Instalador:** `Tako-Setup-x64.exe` — instala para o seu usuário (sem administrador), com atalho no Menu Iniciar; dados em `%LOCALAPPDATA%\Tako`.
- **Portátil:** `Tako-Portable-x64.exe` — um único executável; guarda os dados na pasta `Tako_Data` ao lado do arquivo.

Depois de instalado, o Tako se atualiza sozinho: ao abrir, baixa a versão nova em segundo plano (conferida pelo SHA-256 do GitHub) e instala ao reiniciar ou ao sair. Para gerar localmente os pacotes: `.\build\rust\Package-Tako.ps1` (o instalador do Windows precisa do Inno Setup 6).

## Estrutura

```
crates/
  tako/                          executável principal e ponto de entrada
  tako-app/                      composição da aplicação e ciclo de vida
  tako-core/                     biblioteca, catálogo, persistência e i18n
  tako-platform/                 integrações com SO (janela, áudio, monitores, FSE)
  tako-sources/                  leitores de lojas (Steam, Epic, Playnite, ROMs)
  tako-themes/                   motor de temas
  tako-ui/                       interface nativa (Slint)
  tako-web/                      hospedeiro web (Chromium/CEF)
web/                             interface web em React + TypeScript
design/                          design system e componentes desktop
build/
  rust/                          scripts de empacotamento (Package-Tako.ps1, package.sh, instaladores)
packaging/                       arquivos de empacotamento e manifesto FSE
```

## Build

Requer Rust (edição 2024, ver `rust-toolchain.toml`), Node 24 e npm (para a interface web). No Windows, o CMake do Visual Studio Build Tools pode ser necessário para compilações com SDL3.

Rodar o launcher (biblioteca real: jogos da Steam + adicionados):

```powershell
cargo run -p tako
```

Controle Xbox (XInput), controle PlayStation (DualSense, DualSense Edge ou DualShock 4, por USB ou Bluetooth, lido direto por HID), teclado ou mouse.

### Controles

| Ação | Controle Xbox | PlayStation | Teclado |
|---|---|---|---|
| Navegar | Direcional ou analógico esquerdo | Direcional ou analógico esquerdo | Setas |
| Selecionar / voltar | A / B | ✕ / ○ | Enter / Esc |
| Favoritar · buscar | X · Y | □ · △ | F · / |
| Trocar de seção | LB / RB | L1 / R1 | Q / E |
| Configurações (ou ações da capa em foco) | Menu (☰) | Options | M |
| **Central de Controle** (abrir e fechar, durante o jogo) | **segurar View + Y** por meio segundo | **segurar Create + △** (Share + △ no DualShock 4) | **Ctrl+Shift+Tab** |

O atalho da Central muda em Configurações › "Abrir a Central de Controle no controle" e "… no teclado" (há "Restaurar atalhos padrão"). O Guia/PS e o Shift+Tab não abrem mais a Central: o Guia abre a Barra de Jogo do Xbox e a Steam, e o Shift+Tab é a sobreposição da Steam. Com um jogo em janela ou em tela cheia sem bordas, a Central abre por cima do jogo, que continua rodando atrás; em tela cheia exclusiva, o Tako vem para a frente com a arte do jogo (detalhes no [DESIGN.md](DESIGN.md), Menu rápido).

Dados do usuário e caches ficam em `%LOCALAPPDATA%\Tako` na versão instalada, ou em `Tako_Data` ao lado do executável na portátil (e ao rodar em desenvolvimento); nenhuma arte de jogo vai para o repositório.

Comandos de validação e testes:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Interface web:

```powershell
cd web
npm install
npm run typecheck
npm run build
```

## Licença

[AGPL-3.0-only](LICENSE), a mesma do [Console Mode](https://github.com/lippdev/consolemode), de onde vêm partes do código. Fontes sob a SIL OFL 1.1 e o tema padrão do Playnite sob MIT, cada um com o aviso na própria pasta.
