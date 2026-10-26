# MVP-022: homologação de temas Fullscreen reais do Playnite

- Data: 2026-10-15
- Escopo: MVP-014 (importar `.pthm`), MVP-015 (`theme.yaml`), MVP-017 (herança do tema padrão), MVP-018/019 (adaptadores de dados e controles `PART_`, interface inteira) e MVP-022 (homologação)
- Medição: teste de fumaça da UI no CI Windows. Cada tema é importado pelo fluxo real (`ShellViewModel.ImportThemeFrom` → `ThemePackage.Install`) e aberto na prévia (`P10Theme.Load` sobre o Fullscreen padrão). O relatório completo fica em `theme-homologation.txt` e as capturas `34-tema-*.png` ficam no artefato `ui-screenshots`.

## Temas escolhidos

Os três temas têm licença MIT. **Nenhum foi versionado**: o teste baixa o `.pthm` da release fixada, confere o SHA-256 (`tests/Launcher.UI.Wpf.Tests/ThemeFixtures.cs`) e só então importa. Sem rede, a homologação aparece como "não executado" e o resto do teste continua.

| Tema | Tipo | Licença | Origem (release) | ThemeApiVersion | XAML no pacote |
|---|---|---|---|---|---|
| **PS5ish 5.0.3** | completo, popular (93★) | MIT, © 2021 davidkgriggs | `davidkgriggs/PS5ish` v5.0.3 | 2.3.0 | 34 (mais App.xaml e GlobalResources.xaml de projeto Blend) |
| **Toggle 1.10.1** | completo, recente, usa extensões | MIT, © 2025 Jonathan Sellier | `jonosellier/toggle-theme-playnite` v1.10.1 | 2.9.0 | 37 |
| **Revisited 2.1.0** | **parcial**: altera só 7 arquivos | MIT, © 2022 ld892012 | `ld892012/PlayniteRevisitedSkin` 2.1.0 | 2.9.0 | 7 |

Ficaram de fora:

- Temas **sem licença**: Aniki-Lite, ReMiX, ReskinPS5, os temas do Kosnix, DashX e outros. Sem licença, todos os direitos ficam reservados.
- Temas **copyleft**: Penumbra Blur (AGPL-3.0) e AllyNite (GPL-3.0).
- **Aniki-ReMake** (MIT, o mais popular): o pacote tem 380 MB, grande demais para o CI.

## API de temas e regra de compatibilidade

- O Playnite 10.62 expõe a API Fullscreen **2.10.0** (`source/Playnite/Themes.cs`). A regra do Playnite é: versão maior igual, e o tema não pode ser mais novo que a API (`ThemeManifest.IsCompatible`).
- O Tako aceita **2.x** (`ThemeManifestValidator.SupportedRange`). Acima de 2.10.0 o tema é aceito com aviso; 1.x e 3.x são recusados. Também são recusados os temas com `Mode: Desktop` e os manifestos sem Id, Name, Mode ou ThemeApiVersion.
- **Composição (MVP-017)**: como no `Themes.ApplyTheme` do Playnite, só valem os arquivos que existem no tema padrão, carregados na ordem padrão → tema. Um tema parcial herda o que não altera, e um arquivo recusado volta para a versão do padrão. Os demais XAML do pacote ficam como "ignorados", sem contar como recusa: arquivos de projeto (`App.xaml`, `GlobalResources.xaml`), galerias e views que o padrão não tem.

## Resultado (CI Windows, carregador XAML restrito + adaptador P10)

"Antes" é a medição da frente C (PR #19), só com a prévia. "Depois" inclui os adaptadores de dados e os controles `PART_` (MVP-018/019, PR #24). Os temas passam a ser aplicados à **interface inteira**: a tela inicial do Tako vira o `Views/Main.xaml` do tema.

| | PS5ish 5.0.3 | Toggle 1.10.1 | Revisited 2.1.0 (parcial) | Playnite padrão (referência) |
|---|---|---|---|---|
| Importação `.pthm` (validação e extração) | ✅ 66 arquivos | ✅ 619 arquivos | ✅ 10 arquivos | — (embutido) |
| `theme.yaml` | ✅ 2.3.0 | ✅ 2.9.0 | ✅ 2.9.0 | ✅ 2.0.0 |
| Arquivos do tema carregados: antes → **depois** | 22 de 26 → **25 de 26** | 27 de 33 → **33 de 33** | 3 de 6 → **4 de 6** | 31 de 33 → **33 de 33** |
| Herdados do padrão: antes → depois | 9 → 8 | 4 → 0 | 28 → 29 (inclui 2 recusados) | — |
| Ignorados (fora do padrão) | 0 | 0 | 1 (`Views/ActionSelection.xaml`) | — |
| `Views/Main.xaml` | ⚠️ herdado do padrão (vídeos de `\ExtraMetadata`, bloqueado) | ✅ do tema | ✅ do tema | ✅ |
| `Views/GameDetails.xaml` | ✅ do tema | ✅ do tema | ⚠️ herdado (`\ExtraMetadata`) | ✅ |
| Interface inteira (tela inicial, capas, detalhes, Jogar, voltar) | ✅ | ✅ | ✅ | ✅ |

Arquivos por tema, depois:

- **PS5ish**: `Constants.xaml` (liberado `SplineThicknessKeyFrame`), `DefaultControls/Button.xaml` (`{SettingsBinding}`, `{PluginSettings}`) e `Views/GameDetails.xaml` (`HtmlTextView` seguro, `{PluginSettings}`, `DesignWidth`) passaram a carregar. Só o `Views/Main.xaml` continua recusado: monta vídeos e a foto de perfil a partir de `\ExtraMetadata`, fora da pasta do tema. A interface usa o `Main.xaml` padrão com o restante do visual do PS5ish.
- **Toggle**: todos os 33 carregam: `{ThemeFileBinding}`, `{PluginSettings}`/`{PluginStatus}`, `{Api}`, `pbeh:FocusBahaviors` e `pin:GameControllerInputBinding` por nome, e a imagem `custom_cover_background.png` que falta no pacote vem do tema padrão, como no Playnite.
- **Revisited**: `Views/Main.xaml` passou a carregar (`{Api Notifications.Count}`). `GameDetails.xaml` e `GameStatus.xaml` continuam recusados por `\ExtraMetadata`.
- **Playnite padrão**: `Views/Main.xaml` (`{Api}`) e `Views/GameDetails.xaml` (`HtmlTextView` seguro) passaram a carregar.

**Frente H (caminhos montados por dados):** a validação estática ganhou regras para `sys:String`, formatos e literais de binding, e uma guarda de mídia passou a barrar em tempo de execução as origens fora do tema. Os números acima não mudam: o teste de homologação confere, para cada tema, o mesmo número de arquivos carregados (`ThemeFixtures.Homologated`). Os temas que montam caminhos (PS5ish e Revisited, para `\ExtraMetadata`) já usavam o `Main.xaml`/`GameDetails.xaml` do padrão por causa do literal bloqueado.

O teste de homologação do CI agora verifica que o `Main.xaml` carrega (`P10ThemeReport.HasMainView`) e se ele veio do próprio tema (Toggle, Revisited) ou do padrão (PS5ish). O bloco "Tema Playnite na interface (frente F)" do teste de fumaça aplica os três temas à interface inteira e abre os detalhes pelo controle (capturas `60-` a `66-` e relatório `theme-whole-ui.txt` no artefato `ui-screenshots`).

### Matriz de recursos

✅ suportado · ⚠️ parcial ou com fallback para o padrão · ❌ não suportado (recusado pelo carregador)

| Recurso do tema | PS5ish | Toggle | Revisited | Situação no Tako |
|---|---|---|---|---|
| Cores, pincéis e fontes (`Constants.xaml`) | ✅ | ✅ | ✅ | Suportado (`SplineThicknessKeyFrame` liberado) |
| Mídia (`Media.xaml`, imagens via `ThemeFile`) | ⚠️ herdado | ✅ | ⚠️ herdado | `{ThemeFile}` procura no tema e depois no padrão, sempre dentro das pastas dos temas |
| Controles padrão (Button, CheckBox, ComboBox, Slider, TextBox…) | ✅ 7 de 7 | ✅ 9 de 9 | ✅ Button do tema + resto herdado | Suportado |
| Estilos derivados (`ListGameItemStyle`/`Template`, menus) | ✅ | ✅ | ✅ `ListGameItemStyle` do tema | Suportado |
| Controles próprios (filtros, `FilterPresetSelector`) | ✅ | ✅ | ⚠️ herdado | Filtros rápidos (Todos, Instalados, Favoritos, Jogados recentemente) e **painel de filtros** (frente E): `Filters` com Instalados, Não instalados, Favoritos, Jogados recentemente, Fonte, Gênero, Ordenar e Limpar, desenhado com os estilos do tema (`FilterPanelCheckBoxEx`, `FilterPanelNagivationButton`, `FilterPanelButtonEx`, `FilterPanelText`); Fonte e Gênero abrem o `FilterDbItemtSelection` do tema em `PART_ContentFilterItems`. `FiltersAdditional` fica oculto |
| Tela principal (`Views/Main.xaml`) e `PART_` | ⚠️ herdada | ✅ | ✅ | **Suportado**: grade de jogos e seleção, Jogar/Instalar, Detalhes, Opções, Buscar (teclado virtual do Tako; o tema mostra `PART_ElemSearchActive` com `AppSettings.Fullscreen.FilterSettings.Name`), Filtrar (abre o painel; `PART_ElemExtraFilterActive` com filtro ligado), menu principal (→ Configurações do Tako), relógio, fundo do jogo |
| Detalhes (`Views/GameDetails.xaml`) | ✅ | ✅ | ⚠️ herdado | Suportado: capa, fundo, dados do jogo, ação principal e opções; a descrição é texto puro (`HtmlTextView` seguro) |
| `Views/GameStatus.xaml` | ✅ (oculto) | ✅ (oculto) | ❌ `\ExtraMetadata` | O status do jogo usa a tela de sessão do Tako |
| Dados do Playnite (`{Api}`, `{Settings}`, `{SettingsBinding}`) | ✅ | ✅ | ✅ | Subconjunto só-leitura; configurações com os valores do Tako |
| Extensões de terceiros (SuccessStory, ExtraMetadataLoader, ThemeOptions…) | ⚠️ | ⚠️ | ⚠️ | O arquivo carrega, e a extensão aparece como "não instalada" (`{PluginSettings}`/`{PluginStatus}`). O Toggle mostra o próprio aviso de que falta o ThemeOptions |
| Arquivos fora da pasta do tema (`\ExtraMetadata`, vídeos, perfil) | ❌ | — | ❌ | Bloqueado por segurança (`PathEscapesTheme`) |
| Caminhos montados por dados (`MultiBinding`/`StringFormat`/`sys:String` → imagem ou mídia) | ⚠️ | — | ⚠️ | Só carregam da pasta do tema, do tema padrão, do cache de artes ou de `pack://application`; UNC, URLs e arquivos de fora são bloqueados em tempo de execução, sem aviso na tela e com registro no log (frente H, [questão 8](../spikes/MVP-016-xaml-restrito.md#questão-8-fechada-na-frente-h-caminhos-montados-por-dados)) |
| Behaviors e atalhos de controle do Playnite | — | ⚠️ | — | `FocusBahaviors` funciona; `GameControllerInputBinding` fica inerte (a entrada é do Tako) |
| Bindings a `Window.DataContext` do Playnite | ⚠️ | ⚠️ | — | Ficam vazios (no Tako a janela é a do launcher); vão para o relatório de erros de binding |

### O que ainda fica de fora

1. Caminhos `\ExtraMetadata\...` (vídeos, logos e fundos que o ExtraMetadataLoader grava fora do tema): continuam bloqueados. Por isso o `Main.xaml` do PS5ish e o `GameDetails.xaml`/`GameStatus.xaml` do Revisited usam os do padrão.
2. Extensões do Playnite: não há plugins no Tako, e o tema as vê como não instaladas.
3. Bateria, progresso e filtros adicionais do Playnite (`FiltersAdditional`, listas de plataforma, categoria, série etc.): fora do subset. A busca e o painel de filtros passaram a funcionar na frente E (UX-001/002/004), com as mesmas regras da biblioteca do Tako (`LibraryCriteria`): sem acento, sem diferenciar maiúsculas, ordenação por nome, último jogado, tempo de jogo, nota e data de adição. O `Views/TextInput.xaml` dos temas não é usado: a busca abre o teclado virtual do Tako por cima do tema.

**Frente E (filtros e busca):** o teste de fumaça abre o painel de filtros, marca Instalados, abre a lista de fontes, volta com B e busca "God" em cada tema homologado e no padrão (capturas `67-` a `69-` por tema no artefato `ui-screenshots`; resultado em `theme-whole-ui.txt`). Os números de arquivos carregados da tabela acima não mudam: nenhuma liberação nova no carregador XAML.

Nenhuma liberação foi genérica: cada tipo ou extensão entra por nome e é implementado no adaptador. As justificativas estão em [`docs/spikes/MVP-016-xaml-restrito.md`](../spikes/MVP-016-xaml-restrito.md#liberações-para-a-interface-inteira-mvp-018019).

## Como reproduzir

- **No app**: Configurações › Temas › "Importar tema do Playnite (.pthm)", escolha um dos pacotes acima e depois "Diagnóstico" no tema. "Usar este tema" aplica o tema à tela inicial; Menu (☰ / M) volta às Configurações do Tako.
- **Pela linha de comando (Windows)**: extraia o `.pthm` e rode
  `dotnet run --project tools/Launcher.ThemeCheck -- <pasta> --p10 --load --base src/Launcher.UI.Wpf/Themes/Playnite-Default`.
- **No CI**: o artefato `ui-screenshots` traz `theme-homologation.txt`, com OK/RECUSA/HERDADO/IGNORADO por arquivo e os erros de binding, e as capturas `30-` a `35-`; a interface inteira está em `theme-whole-ui.txt` e nas capturas `60-` a `66-`.
