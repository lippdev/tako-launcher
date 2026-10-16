# Tako

<img src="logo/Logo%20TAKO%20com%20fundo%20transparente.png" alt="Tako" width="220" />

Launcher de jogos para Windows 11 com experiência de console: gamepad-first, temas Fullscreen do Playnite 10 e integração com o Windows Full Screen Experience (FSE). Status: **planejamento / Marco 0**.

- Produto e backlog: [`docs/PRD.md`](docs/PRD.md)
- Por onde começar: [`docs/ROADMAP.md`](docs/ROADMAP.md)
- Biblioteca, metadados e artes: [`docs/metadados.md`](docs/metadados.md)
- Como contribuir (Git Flow, commits, PRs, DoD, arquitetura): [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Decisões de arquitetura: [`docs/adr/`](docs/adr/)

## Baixar

Na [página de releases](https://github.com/lippdev/ultralauncher/releases), no mesmo formato do Console Mode:
- **Instalador:** `Tako-Setup-x64.exe` — instala para o seu usuário (sem administrador), com atalho no Menu Iniciar; dados em `%LOCALAPPDATA%\Tako`.
- **Portátil:** `Tako-Portable-x64.exe` — um único executável; guarda os dados na pasta `Tako_Data` ao lado do arquivo.

Nenhum dos dois exige o .NET instalado. Depois de instalado, o Tako se atualiza sozinho: ao abrir, baixa a versão nova em segundo plano (conferida pelo SHA-256 do GitHub) e instala ao reiniciar ou ao sair. Para gerar localmente: `.\build\Publish-Tako.ps1 -Version 0.1.0` (o instalador precisa do Inno Setup 6).

## Estrutura

```
src/
  Launcher.Host/                 supervisão, modo FSE/Fullscreen, recuperação
  Launcher.UI.Wpf/               telas Fullscreen (WPF, MVVM), tema padrão, navegação por controle
  Launcher.Core/                 biblioteca, catálogo, persistência (SQLite)
  Launcher.Themes/               descoberta/validação/aplicação de temas
  Launcher.PlayniteCompat.P10/   adaptador de temas Playnite 10
  ConsoleMode.Runtime/           contratos de sessão/monitores/áudio/IPC (net8.0 + net10.0)
tools/
  Launcher.ThemeCheck/           valida uma pasta de tema com o carregador XAML restrito (spike MVP-016)
  Launcher.FseProbe/             coleta sinais do Xbox mode / FSE e testa o ciclo de vida de home app (spike MVP-007)
packaging/
  fse-dev/                       pacote de desenvolvimento da FseProbe como gaming home app (só Modo de Desenvolvedor)
build/                           Publish-Tako.ps1 (portátil + instalador) e Tako.iss (Inno Setup)
tests/                           xUnit
```

## Build

Requer Windows 11 e [.NET 10 SDK](https://dotnet.microsoft.com/download) (ver `global.json`).

Rodar o launcher (biblioteca real: jogos da Steam + adicionados):

```
dotnet run --project src/Launcher.UI.Wpf -c Release
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

Dados do usuário e caches ficam em `%LOCALAPPDATA%\Tako` na versão instalada, ou em `Tako_Data` ao lado do executável na portátil (e ao rodar pelo `dotnet run`); nenhuma arte de jogo vai para o repositório.

```
dotnet restore
dotnet build
dotnet test --filter-not-trait "Category=Hardware" --ignore-exit-code 8
```

Versões de pacotes ficam centralizadas em `Directory.Packages.props`, com lock files (`packages.lock.json`). Ao mudar uma versão, rode `dotnet restore --force-evaluate` e commite os lock files. Os testes usam xUnit v3 sobre Microsoft.Testing.Platform (ver `global.json`).

Fora do Windows (macOS/Linux) é possível compilar com `-p:EnableWindowsTargeting=true`, mas os testes dos projetos `-windows` só rodam no Windows.

## Licença

[AGPL-3.0-only](LICENSE), a mesma do [Console Mode](https://github.com/lippdev/consolemode), de onde vêm partes do código. Fontes sob a SIL OFL 1.1 e o tema padrão do Playnite sob MIT, cada um com o aviso na própria pasta.
