# MVP-022: homologação de temas Fullscreen reais do Playnite

- Data: 2026-10-15
- Escopo: MVP-014 (importar `.pthm`), MVP-015 (`theme.yaml`), MVP-017 (herança do tema padrão) e MVP-022 (homologação)
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

| | PS5ish 5.0.3 | Toggle 1.10.1 | Revisited 2.1.0 (parcial) | Playnite padrão (referência) |
|---|---|---|---|---|
| Importação `.pthm` (validação e extração) | ✅ 66 arquivos | ✅ 619 arquivos | ✅ 10 arquivos | — (embutido) |
| `theme.yaml` | ✅ 2.3.0 | ✅ 2.9.0 | ✅ 2.9.0 | ✅ 2.0.0 |
| Arquivos do tema carregados | **22 de 26** | **27 de 33** | **3 de 6** | 31 de 33 |
| Herdados do padrão | 9 | 4 | 28 | — |
| Ignorados (fora do padrão) | 0 | 0 | 1 (`Views/ActionSelection.xaml`) | — |
| Grade de jogos na prévia | ✅ visual do tema | ✅ visual do tema (foco laranja, barra azul) | ✅ estilo do tema + resto do padrão | ✅ |

### Matriz de recursos

✅ suportado · ⚠️ parcial ou com fallback para o padrão · ❌ não suportado (recusado pelo carregador)

| Recurso do tema | PS5ish | Toggle | Revisited | Situação no Tako |
|---|---|---|---|---|
| Cores, pincéis e fontes (`Constants.xaml`) | ⚠️ recusado por `SplineThicknessKeyFrame`; usa o do padrão | ✅ | ✅ | Suportado. `SplineThicknessKeyFrame` fora da lista de permissões (candidato a liberar: animação inofensiva) |
| Mídia (`Media.xaml`, imagens via `ThemeFile`) | ⚠️ herdado | ⚠️ `BitmapImage` falha na carga; usa o do padrão | ⚠️ herdado | `ThemeFile` suportado; imagem que falha recusa o arquivo inteiro |
| Controles padrão (Button, CheckBox, ComboBox, Slider, TextBox…) | ⚠️ 6 de 7 (Button usa `{SettingsBinding}`; usa o do padrão) | ✅ 9 de 9 | ✅ Button do tema + resto herdado | Suportado |
| Estilos derivados (`ListGameItemStyle`/`Template`, menus) | ✅ | ⚠️ `{PluginSettings}`; usa o do padrão | ✅ `ListGameItemStyle` do tema | Suportado. `{PluginSettings}` (extensões do Playnite) não |
| Controles próprios (filtros, `FilterPresetSelector`) | ✅ | ⚠️ `{ThemeFileBinding}` | ⚠️ herdado | `ThemeFileBinding` ainda não implementado no adaptador |
| Views de menu, filtros, mensagens, notificações, configurações | ✅ | ✅ | ⚠️ herdado | Suportado (renderização completa das views: MVP-018/019) |
| `Views/Main.xaml` | ❌ vídeo de fundo fora da pasta do tema (`\ExtraMetadata\...`) | ❌ `{PluginSettings}` | ❌ `{Api}` | Não suportado: API do Playnite, extensões e arquivos fora do tema |
| `Views/GameDetails.xaml` | ❌ `{PluginSettings}`, `HtmlTextView` | ❌ `FocusBahaviors` (Playnite.Behaviors) | ❌ `{PluginSettings}`, `{Api}` | Não suportado: `HtmlTextView` (segurança), behaviors e API |
| `Views/GameStatus.xaml` | ✅ | ✅ | ❌ `{Api}`, caminho fora do tema | Parcial |
| Extensões de terceiros (SuccessStory, ExtraMetadataLoader, ThemeOptions…) | ❌ | ❌ | ❌ | Não suportado: não há plugins do Playnite no Tako |
| Arquivos fora da pasta do tema (`\ExtraMetadata`, vídeos, perfil) | ❌ | — | ❌ | Bloqueado por segurança (`PathEscapesTheme`) |
| Bindings para dados do Playnite (`Platforms`, `Library`, `ElementName` de PART_) | ⚠️ ficam vazios | ⚠️ ficam vazios | ⚠️ ficam vazios | Dados e PART_ ainda não fornecidos (MVP-018/019). Os erros de binding de temas de terceiros vão para o relatório, não para a verificação do launcher |

### O que bloqueia mais arquivos (por frequência)

1. `{PluginSettings}`/`{PluginStatus}`: configurações de extensões do Playnite. O adaptador poderia expor um objeto vazio, para os valores ficarem nulos em vez de recusar o arquivo (MVP-018).
2. `{Api …}`: API do Playnite (notificações, caminhos). Fica no escopo do MVP-018.
3. Caminhos `\ExtraMetadata\...`: vídeos e logos que o ExtraMetadataLoader grava fora do tema. Continuam bloqueados.
4. `{SettingsBinding}` e `{ThemeFileBinding}`: extensões do Playnite que ainda não existem no adaptador. São candidatas diretas, porque não executam código.
5. `SplineThicknessKeyFrame` e `Playnite.Behaviors`: tipo e behavior fora da lista. Cada um só entra individualmente e com justificativa (spike MVP-016, questão 3).

Nenhum item da política de segurança foi afrouxado nesta homologação.

## Como reproduzir

- **No app**: Configurações › Temas › "Importar tema do Playnite (.pthm)", escolha um dos pacotes acima e depois "Diagnóstico" no tema.
- **Pela linha de comando (Windows)**: extraia o `.pthm` e rode
  `dotnet run --project tools/Launcher.ThemeCheck -- <pasta> --p10 --load --base src/Launcher.UI.Wpf/Themes/Playnite-Default`.
- **No CI**: o artefato `ui-screenshots` traz `theme-homologation.txt`, com OK/RECUSA/HERDADO/IGNORADO por arquivo e os erros de binding, e as capturas `30-` a `35-`.
