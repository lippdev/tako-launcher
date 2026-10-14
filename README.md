# UltraLauncher

Launcher de jogos para Windows 11 com experiência de console: gamepad-first, temas Fullscreen do Playnite 10 e integração com o Windows Full Screen Experience (FSE). Status: **planejamento / Marco 0**.

- Produto e backlog: [`docs/PRD.md`](docs/PRD.md)
- Por onde começar: [`docs/ROADMAP.md`](docs/ROADMAP.md)
- Como contribuir (Git Flow, commits, PRs, DoD, arquitetura): [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Decisões de arquitetura: [`docs/adr/`](docs/adr/)

## Estrutura

```
src/
  Launcher.Host/                 supervisão, modo FSE/Fullscreen, recuperação
  Launcher.UI.Wpf/               telas Fullscreen (WPF, MVVM)
  Launcher.Core/                 biblioteca, catálogo, persistência (SQLite)
  Launcher.Themes/               descoberta/validação/aplicação de temas
  Launcher.PlayniteCompat.P10/   adaptador de temas Playnite 10
  Console.Runtime/               contratos de sessão/monitores/áudio/IPC
tests/                           xUnit
```

## Build

Requer Windows 11 e [.NET 10 SDK](https://dotnet.microsoft.com/download) (ver `global.json`).

```
dotnet restore
dotnet build
dotnet test --filter "Category!=Hardware"
```

> O esqueleto foi gerado sem o SDK 10 instalado localmente e ainda não foi compilado; o primeiro build/CI pode exigir pequenos ajustes.
