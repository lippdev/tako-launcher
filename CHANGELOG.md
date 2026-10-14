# Changelog
Formato [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), versionamento [SemVer](https://semver.org/lang/pt-BR/).

## [Não lançado]
### Adicionado
- Spike MVP-016: validador e carregador XAML restrito para temas (`Launcher.Themes.Xaml`), ferramenta `Launcher.ThemeCheck` e relatório em `docs/spikes/`.
- Estrutura inicial da solução, CI e regras de desenvolvimento.

### Alterado
- `Console.Runtime` renomeado para `ConsoleMode.Runtime` (evita conflito com `System.Console`) e compilado para `net8.0` e `net10.0` (ADR-0011).
- `Launcher.Themes` passa a usar `net10.0-windows` com WPF.
- Versões de pacotes centralizadas em `Directory.Packages.props`; testes migrados para xUnit v3 4.x sobre Microsoft.Testing.Platform.
- Lock files de pacotes (`packages.lock.json`); o CI restaura em `--locked-mode`.
- Regra de aprovação: 1 aprovação de code owner enquanto houver dois mantenedores; `nextestudios` adicionado ao CODEOWNERS.

### Corrigido
- Build do CI falhava com CA1707 nos nomes de teste.
