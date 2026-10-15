# Changelog
Formato [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), versionamento [SemVer](https://semver.org/lang/pt-BR/).

## [Não lançado]
### Adicionado
- Biblioteca real: jogos instalados pela Steam encontrados automaticamente (todas as pastas de biblioteca, tempo jogado e última vez da conta) e jogos adicionados pelo usuário (executável, atalho ou URI), com favoritos e histórico em `%LOCALAPPDATA%\Tako\library.json`. "Jogar" abre o jogo de verdade (`steam://rungameid/…` ou o executável). A biblioteca de exemplo saiu do app e ficou só nos testes.
- Agente de metadados (como no Plex): descrição em português, gêneros, lançamento, desenvolvedora, nota do Metacritic e artes 2x pela API pública da Steam Store, com correspondência por título para jogos adicionados, cache em disco, atualização em segundo plano e "Corrigir correspondência". Opções do jogo: abrir na Steam, trocar capa, abrir pasta, remover.
- Launcher WPF do Marco 0 a partir do protótipo: Início, Biblioteca, Detalhes, Configurações, sessão de jogo simulada, Central de Controle, diálogos e tela de recuperação; navegação por controle (XInput), teclado e mouse; tema padrão carregado pelo carregador XAML restrito; teste de fumaça da UI com capturas no CI. Relatório do Marco 0 em `docs/marco-0/relatorio.md`.
- `DESIGN.md` (tokens do tema padrão) e protótipo navegável por controle em `design/prototype/index.html` (MVP-013, MVP-023, MVP-028).
- Spike MVP-007: pesquisa do FSE / Xbox mode (`docs/spikes/MVP-007-fse.md`), ferramenta `Launcher.FseProbe` e pacote de desenvolvimento em `packaging/fse-dev/`.
- Spike MVP-016: validador e carregador XAML restrito para temas (`Launcher.Themes.Xaml`), ferramenta `Launcher.ThemeCheck` e relatório em `docs/spikes/`.
- Nome do produto: **Tako**. Executável `Tako.exe` com o ícone do polvo, marca na barra lateral e textos da interface; logos em `logo/`. O repositório e os namespaces continuam `ultralauncher`/`Launcher.*`.
- Controles PlayStation (DualSense, DualSense Edge, DualShock 4; USB e Bluetooth) lidos direto por HID, com dicas ✕ ○ □ △ e botão PS abrindo a Central de Controle.
- Interface responsiva: o palco acompanha a proporção da tela (21:9, 16:10…), DPI por monitor e artes decodificadas na resolução física (capas e fundos 2x da Steam), nítidas em 2K/4K.
- Releases com instalador (`Tako-…-Setup-x64.exe`, Inno Setup, por usuário sem administrador, com desinstalador) e versão portable (`Tako-…-Portable-x64.zip`, dados em `Data\` ao lado do executável via `portable.txt`).
- Workflow de release: cada tag `v*` publica o launcher (self-contained, win-x64) e o kit do experimento FSE na página da release.
- Estrutura inicial da solução, CI e regras de desenvolvimento.

### Alterado
- Central de Controle no teclado: Shift+Tab (como na Steam) no lugar de G. Títulos em Source Sans 3 leve (estilo PS5) ocupando a largura da tela; barra lateral com o logo horizontal do Tako e itens que cabem na altura.
- `Console.Runtime` renomeado para `ConsoleMode.Runtime` (evita conflito com `System.Console`) e compilado para `net8.0` e `net10.0` (ADR-0011).
- `Launcher.Themes` passa a usar `net10.0-windows` com WPF.
- Versões de pacotes centralizadas em `Directory.Packages.props`; testes migrados para xUnit v3 4.x sobre Microsoft.Testing.Platform.
- Lock files de pacotes (`packages.lock.json`); o CI restaura em `--locked-mode`.
- Regra de aprovação: 1 aprovação de code owner enquanto houver dois mantenedores; `nextestudios` adicionado ao CODEOWNERS.
- Temas: `Popup` aceito só como lista de lista suspensa (ComboBox/ComboBoxEx), sem abrir nem se mover sozinho; o adaptador P10 fornece os booleanos e conversores que o Playnite dá aos temas. Tema Fullscreen padrão do Playnite 10: 31 de 33 arquivos carregados (MVP-016, MVP-018).

### Corrigido
- Build do CI falhava com CA1707 nos nomes de teste.
