# Changelog
Formato [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), versionamento [SemVer](https://semver.org/lang/pt-BR/).

## [Não lançado]
### Adicionado
- Estrutura inicial da solução, CI e regras de desenvolvimento.

### Alterado
- `Console.Runtime` renomeado para `ConsoleMode.Runtime` (evita conflito com `System.Console`) e compilado para `net8.0` e `net10.0` (ADR-0011).
- `Launcher.Themes` passa a usar `net10.0-windows` com WPF.
- Versões de pacotes centralizadas em `Directory.Packages.props`; testes migrados para xUnit v3 4.x sobre Microsoft.Testing.Platform.

### Corrigido
- Build do CI falhava com CA1707 nos nomes de teste.
