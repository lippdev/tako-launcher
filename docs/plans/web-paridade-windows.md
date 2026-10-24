# Paridade Windows: interface web × Slint

Critério de pronto da migração nesta fase (PRD D-15): **a versão web no Windows faz tudo o que a interface Slint
fazia no Windows.** Linux e macOS ficam para depois. Cada linha é uma função da `tako-ui` (Slint) e o que a
versão web tem dela.

Estados: ✅ pronto · 🟡 parcial · ⬜ falta · — não se aplica.

## Núcleo e plataforma

| Função (Slint) | Web | Tarefa |
|---|---|---|
| Boot do núcleo, biblioteca, snapshot | ✅ | WEB-01a |
| Leitura das lojas e emuladores (`sources`, Playnite) | 🟡 lojas e emuladores; falta importar do Playnite | WEB-01c · **WEB-31** |
| Metadados e arte (`metadata.rs`, `art.rs`): fontes, cache, IGDB, ScreenScraper, SteamGridDB | 🟡 cache lido pelo hospedeiro; falta arte pelo núcleo e a busca de metadados | **WEB-01d**, **WEB-32** |
| Sessão de jogo (`session.rs`, `hosts.rs`): abrir, tela e áudio, acompanhar, restaurar, recuperação | ⬜ em andamento | **WEB-20** |
| "Abrindo…" (`launch_splash`) e "Fechando o jogo…" (`closing_game`) | ⬜ | **WEB-21** |
| Central rápida durante o jogo (`control_center`) e o atalho global | ⬜ | **WEB-22** |
| Controles pelo SDL3 (`gamepad.rs`): navegação, bateria, reconexão, isolar do jogo (HidHide) | ⬜ teclado pronto | **WEB-05b** |
| Teclado virtual (`keyboard`) para busca e campos no controle | ⬜ | **WEB-23** |
| Atualizações do app (`app_updates.rs`) | 🟡 componente pronto; falta ligar | **WEB-33** |
| Discord Rich Presence (`discord.rs`) | ⬜ chave existe, ninguém lê | **WEB-34** |
| API local (`local_api.rs`, EXT-002) | ⬜ | **WEB-34** |
| Links `tako://` (`links`) | ⬜ | **WEB-34** |
| Splash de abertura e primeira leitura (`splash.rs`) | ✅ | WEB-35 |
| Interface descartável (núcleo e interface separados) | ✅ | WEB-02b |
| Bandeja do sistema | ⬜ | **WEB-36** |
| Desempenho do Tako (`performance`, PERF-001) | ⬜ | **WEB-37** |
| Modo seguro e recuperação de tema | ⬜ | **WEB-37** |
| Sandbox do CEF | ⬜ | **WEB-02c** |

## Telas do desktop

| Função (Slint) | Web | Tarefa |
|---|---|---|
| Janela, barra de cima, menu do Tako (`bar`, `desktop`) | ✅ | WEB-10a |
| Lista da biblioteca: abas, busca, filtros, ordenação, coleções, seleção múltipla (`desktop`, `library`) | ✅ | WEB-11a |
| Modos Grade e Categorias da lista | 🟡 componentes prontos; falta ligar | WEB-04j · **WEB-11c** |
| Visão geral (`overview`, `home`) | ✅ (falta "Voltar a jogar") | WEB-10b |
| Página do jogo (`details`) | ✅ (falta "jogando", depende do WEB-20) | WEB-12a |
| Configurações do jogo (`game_settings`) | 🟡 pronto; comandos antes e depois só leitura até a aprovação | WEB-12b · **WEB-20** |
| Editar jogo (`edit_game`) e Fontes e IDs (`source_ids`) | ⬜ | **WEB-12c** |
| Adicionar jogo (`add_game`) | ⬜ | **WEB-12c** |
| Seletor de imagens (`art`) e Revisar capas (`art_review`) | 🟡 componente do seletor pronto | **WEB-12d** |
| Editor de etiquetas (`tags_editor`) | ✅ | WEB-12b |
| Coleções automáticas (`smart`): salvar filtros | ✅ (falta a lista em Configurações › Biblioteca, WEB-13c) | WEB-11b |
| Busca geral Ctrl+K (`global_search`) | ✅ | WEB-14a |
| Downloads (`downloads`): painel, ligar com aviso legal, fontes | 🟡 pronto; falta gerenciar fontes e abrir a pasta (núcleo) | WEB-14b · **WEB-14e** |
| Atividades na barra (`activities`) | ✅ (downloads) | WEB-14b |
| Perfil (`profile`) e perfis locais (`profiles`, UX-004) | 🟡 pronto; falta reabrir com o perfil trocado e "Jogando…" | WEB-14c · **WEB-20** |
| Diálogos do jogo e da biblioteca (`dialogs`): remover, erro do jogo, confirmar | 🟡 erro do jogo pronto | **WEB-14d** |
| Primeira execução (`first_run`, UX-003) | ⬜ | **WEB-16** |

## Configurações

| Categoria (Slint) | Web | Tarefa |
|---|---|---|
| Geral, Aparência, Bibliotecas | ✅ | WEB-13a |
| Tema, Som, Tela e áudio, Integrações, Sistema, Sobre | ✅ (lacunas no quadro) | WEB-13b |
| Aparência do desktop: cor de destaque, opções do Liquid Glass (`settings_desk_theme`) | ⬜ | **WEB-13c** |
| Capas e metadados: chaves no cofre, capas (`settings_metadata`) | ⬜ | **WEB-13c** |
| Controles: atalhos, mapeamento, HidHide (`settings_controls`) | ⬜ | **WEB-13c** (depois do WEB-05b) |
| Emuladores e ROMs (`settings_emulators`) | ⬜ | **WEB-13c** |
| Sunshine com prévia e confirmação (`settings_sunshine`) | 🟡 só a chave | **WEB-13c** |
| Downloads (ligar, aviso legal, limite) | ✅ | WEB-14b |
| Perfis | ✅ (em Geral) | WEB-14c |
| Backup e restauração (CLOUD-001) | ⬜ | **WEB-13c** |

## Tela cheia (Big Picture)

| Função (Slint) | Web | Tarefa |
|---|---|---|
| Tema padrão da tela cheia, temas alternativos (Fita), prévia de tema (`theme_preview`, `tiles`) | ⬜ | **WEB-15** (espera o design system da tela cheia) |
| Troca desktop ↔ tela cheia (F11, controle conectado) | ⬜ | **WEB-15** |

## Textos

| Função | Web | Tarefa |
|---|---|---|
| Idiomas pt-BR e en-US em todas as telas | 🟡 infraestrutura pronta | **WEB-06b** |

## Fechamento (só Windows)

| Item | Tarefa |
|---|---|
| Atualização do Chromium (processo e CI que avisa) | WEB-41 |
| Medição contra a §8.5 no release, e comparação CEF × WebView2 | WEB-60 |
| Instalador por componentes e onboarding | INST-01 a INST-04 |
| Corte: a web vira o padrão no Windows, sai o Slint | WEB-50 |
