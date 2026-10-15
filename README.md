# Tako

<img src="logo/Logo%20TAKO%20com%20fundo%20transparente.png" alt="Tako" width="220" />

Launcher de jogos para Windows 11 com experiência de console: gamepad-first, temas Fullscreen do Playnite 10 e integração com o Windows Full Screen Experience (FSE). Status: **planejamento / Marco 0**.

- Produto e backlog: [`docs/PRD.md`](docs/PRD.md)
- Por onde começar: [`docs/ROADMAP.md`](docs/ROADMAP.md)
- Biblioteca, metadados e artes: [`docs/metadados.md`](docs/metadados.md)
- Como contribuir (Git Flow, commits, PRs, DoD, arquitetura): [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Decisões de arquitetura: [`docs/adr/`](docs/adr/)

## Baixar

Na [página de releases](https://github.com/lippdev/ultralauncher/releases): **instalador** (`Tako-…-Setup-x64.exe`, sem precisar de administrador) ou **portable** (`Tako-…-Portable-x64.zip`, roda de qualquer pasta e guarda os dados em `Data\` ao lado do `Tako.exe`). Nenhum dos dois exige o .NET instalado.

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
  installer/                     instalador (Inno Setup) usado pelo workflow de release
  release/                       notas, LEIA-ME e marcador portable.txt das releases
tests/                           xUnit
```

## Build

Requer Windows 11 e [.NET 10 SDK](https://dotnet.microsoft.com/download) (ver `global.json`).

Rodar o launcher (biblioteca real: jogos da Steam + adicionados):

```
dotnet run --project src/Launcher.UI.Wpf -c Release
```

Controle Xbox (XInput), controle PlayStation (DualSense, DualSense Edge ou DualShock 4, por USB ou Bluetooth, lido direto por HID), teclado ou mouse. Teclado: setas, Enter, Esc, F (favoritar), Q/E (seções), M (configurações), Shift+Tab (Central de Controle, como na Steam; no controle, Guia ou PS). Dados do usuário e caches ficam em `%LOCALAPPDATA%\Tako` (ou em `Data\` no modo portable); nenhuma arte de jogo vai para o repositório.

```
dotnet restore
dotnet build
dotnet test --filter-not-trait "Category=Hardware" --ignore-exit-code 8
```

Versões de pacotes ficam centralizadas em `Directory.Packages.props`, com lock files (`packages.lock.json`). Ao mudar uma versão, rode `dotnet restore --force-evaluate` e commite os lock files. Os testes usam xUnit v3 sobre Microsoft.Testing.Platform (ver `global.json`).

Fora do Windows (macOS/Linux) é possível compilar com `-p:EnableWindowsTargeting=true`, mas os testes dos projetos `-windows` só rodam no Windows.
