# Implementação

Como levar este sistema para o código (`crates/tako-ui`, branch `develop`), em etapas que podem ir para o ar uma de cada vez.

## Rebranding (out/2026) — o que muda no app

O sistema foi refeito pelo protótipo do mantenedor (`launcher.html`). O que já estava no app até a rodada 71ad29a (vidro flutuante sobre a arte) passa a ser substituído por estas etapas, nesta ordem:

1. **Tokens e temas.** `tokens.json` novo: paleta do tema TAKO (`canvas` #0b0b0e, `surface`, `tile`, `line`…), raios (7/9/10/11/16/18), movimento (`curva-entrada` = `cubic-bezier(0.22, 1, 0.36, 1)`, tempos do protótipo). Os nomes antigos (`glass-*`, `on-accent*`, `settings-*`) ficam como apelidos para nada quebrar no meio. Temas: TAKO (padrão), Escuro, Meia-noite, Preto OLED, Liquid Glass, em Configurações › Tema, com a cor de destaque.
2. **Marca.** `tako-mark.png` (polvo), `tako-wordmark.png` (TAKO) e `tako-kana.png` (タコ) em `assets/Logos`; a barra de cima monta as três (50×40 + 68×14 + 46×8). Ícone do app: o polvo.
3. **Cores da arte do jogo (TAKO).** Amostra da arte (64×36), matiz mais forte, destaque com saturação 62–92% e luz 50–58%; só o destaque e dois brilhos de canto mudam, em 1 s. Barra de cima fixa nas cores do logo. Ver Temas.
4. **Estrutura.** BarraSuperior de 52 (marca, LOJA · BIBLIOTECA, downloads, perfil, tela cheia, janela); ListaDeJogos fixa de 292 (recolhe para 76 com Ctrl B; grade de capas; some na loja e nas configurações).
5. **Peças.** GrupoDeBlocos, BotaoDesktop (primário branco com chip), barra de dados (CartaoSessoes), chips, menus, seletores, chave, deslizante, linha de configuração.
6. **Telas.** TelaBiblioteca (Visão geral), TelaJogo/TelaJogoRolada (uma página para loja e biblioteca), TelaLoja, TelaLojaExplorar, TelaListaDeDesejos, TelaConfiguracoes e TelaTema; depois perfil, onboarding e camadas.
7. **Movimento.** Troca de vista, troca de jogo em escada, lista, arte, menus e camadas com os tempos de MovimentoBase; Reduzir animações zera tudo.

A AtividadeNaBarra que entrou no app (pílula antes do perfil) vira o botão **Downloads** com contador e o popover do protótipo; a fila de atividades por trás continua a mesma.

Saem: DestaqueFases (a página do jogo agora rola) e BarraLateralUnida.
**Pronto quando:** cada tela do app bate com a prévia do mesmo nome a 1600×900, nos cinco temas, com Reduzir animações e com Efeitos de vidro desligado.

---

## Antes do rebranding (histórico)

As etapas abaixo descrevem o sistema anterior (vidro flutuante); ficam como registro do que já entrou no app.

## Etapa 1 — Tokens no `Ui`

- Acrescentar ao global `Ui` (tokens.slint), ao lado dos atuais, os tokens deste sistema que o desktop usa: `glass-*` (panel, panel-dense, highlight, specular, edge, control, hover, selected, pressed, edge-strong, solid, item-highlight, item-edge), `focus-*`, `primary-*`, `art-scrim-left/bottom`, `settings-*`, os raios (`radius-thumb/control/row/card`), `tempo-hover`, `medal-*` (taças de Conquistas) e `art-scrim-strong` (Perfil, Grade, Categorias).
- As durações que já existem (`focus-time`, `page-time`, `color-time`, `leave-time`) ficam; as curvas viram `easing: cubic-bezier(…)` nas animações do Slint.
- Nova propriedade `reduce-transparency` (como `reduce-motion`): quando ligada, os `glass-*` assumem os valores do tema `solido`.
- **Fontes:** títulos em **Poppins** (400–800, `fonts/Poppins-*.ttf`) e todo o resto em **Plus Jakarta Sans** (variável 200–800, `fonts/PlusJakartaSans-Variable.ttf`), as duas OFL. Copiar para `crates/tako-ui/fonts/`, registrar com `import "../fonts/…ttf";` no `tokens.slint` e expor `Ui.font-heading` / `Ui.font-text`. Títulos (`display`, `headline`, `title`, títulos de cartão, tela, bloco e capa) usam `font-heading`; botões, rótulos, abas, números e texto usam `font-text`. A tela cheia mantém as fontes dela.
- **Pronto quando:** o desktop compila usando só `Ui.*`, sem cor ou medida solta nos componentes.

## Etapa 2 — Material (Liquid Glass sem desfoque em tempo real)

- `images.rs`: ao decodificar a arte de fundo de um jogo, gerar também uma cópia desfocada (`image::imageops::blur`, raio equivalente a `glass-blur` na escala da janela) e saturada (`glass-saturate`), em thread de fundo, guardada no mesmo cache. Medir a luminância média da cópia nas regiões dos painéis para escolher `glass-panel` ou `glass-panel-dense` (limite 0,18, o mesmo do AccentPalette).
- `desktop.slint`: um componente `Vidro` (nome a decidir) que desenha, dentro de um `Rectangle` com `clip: true` e `border-radius`, a cópia desfocada posicionada em `-self.absolute-position` (o pedaço que fica atrás dele), a tinta, o brilho de cima (150 px fixos), a luz ambiente do `accent`, o contorno de 1px e a sombra `shadow-float`.
- Sem cópia pronta, o `Vidro` usa `glass-solid` e troca com cross-fade quando ela chega.
- **Pronto quando:** barra, painel e um cartão mostram a arte desfocada atrás deles; a rolagem da lista não redesenha o desfoque; Reduzir transparência deixa tudo opaco.

## Etapa 3 — Fundo e estrutura da biblioteca

- Capa no padrão Steam **600 × 900** (`library_600x900`, 2:3; aceita 1200 × 1800; CapaDoJogo): recorte central se a proporção for outra, decodificada no tamanho em que aparece.
- Arte de fundo no padrão Steam **3840 × 1240** (`library_hero`; FundoDoJogo): cobre a altura da janela, centro da arte a 62% da largura, sem sobra; acima de 31:10 cobre a largura e ancora a 30% do topo. Decodificada no tamanho da janela.
- `DeskLibrary` / `Desktop`: a arte nítida na janela inteira (hoje fica só no Hero), posicionada para a direita, com `art-scrim-left` e `art-scrim-bottom`; as bordas fundidas com a cópia desfocada + 45% do `accent`. Sem arte: degradê do `accent` (EstadoSemArte).
- Margens de 36, barra a 12 do topo, painel de 344, 28 entre painel e conteúdo, 20 entre cartões.
- **Pronto quando:** a TelaBiblioteca bate com a prévia a 1600×900, e a JanelaPequena a 960×600.

## Etapa 4 — Barra superior

- **BarraDaJanela** (unificada): `Window { no-frame: true; }`; o `DeskTitleBar` separado sai e o `TopBar` ocupa o topo inteiro, de borda a borda (sem margem, raio ou sombra; linha de 1 embaixo), com Minimizar · Maximizar/Restaurar · Fechar na ponta direita (`window-control-width` 46, altura toda da barra; X com `window-close-hover`). Hit-test `HTCAPTION` no vazio, `HTMAXBUTTON` no Maximizar, `HTCLOSE` no X. Some na tela cheia.

- `TopBar`: marca com ⌄ que abre o MenuDoTako (Configurações, Trocar perfil…, Ir para a tela cheia, Procurar atualizações…, Sair do Tako); guias de texto em caixa alta (Biblioteca agora; Loja e Perfil só quando existirem); `SearchBox` com o keycap Ctrl + K (Ctrl + F continua); Tela cheia no canto, com o raio da busca.
- Configurações deixa de ser item de navegação.
- **Pronto quando:** BarraSuperior e MenuDoTako batem com as prévias; Tab percorre marca → guias → busca → Tela cheia.

## Etapa 5 — Lista de jogos

- `GameList` vira o painel de vidro: `Tabs` como seletor segmentado (trilho + opção escolhida na segunda camada de vidro); linha de contagem com os ícones de inverter ordem e de filtrar (com a quantidade de filtros); `GameRow` com miniatura quadrada 40×40, seleção na segunda camada de vidro + ▶ redondo, hover só clareando, ponto de "jogando", "Baixando · n%" quando houver, e o ícone da loja de origem (`store-*` de icons.slint) à direita do nome.
- Estados: ListaVazia (busca/filtro sem resultado, biblioteca e coleção vazias) e os blocos de espera do EstadoCarregando.
- Aba Coleções: o `Chips` vira o SeletorDeColecao (`DeskSelect` da largura do painel, 40 de altura, raio `radius-control`, alinhado ao seletor de abas).
- Seleção múltipla (SelecaoMultipla): Ctrl/Shift + clique, caixa redonda na linha, barra de ações no pé do painel; ListaVazia com o polvo.
- Modos de exibição: botão na linha de contagem (Lista · Grade · Categorias); BibliotecaGrade reaproveita o `GameCard` da tela cheia na escala de mesa; BibliotecaCategorias empilha fileiras virtualizadas.
- **Pronto quando:** ListaDeJogos, Abas, ListaVazia, ListaTexto125 SelecaoMultipla, BibliotecaGrade e BibliotecaCategorias batem com as prévias; a lista continua virtualizada.

## Etapa 6 — Destaque, ações e cartões

- `Hero`: etiqueta de estado (20 acima do título), título como **logo** (TituloDoJogo: Steam `library_logo`, caixa 520 × 180, contraste ≥ 3:1 senão texto; Configurações › Aparência › Título dos jogos e exceção por jogo) ou `display` (até duas linhas), faixa de estado (Nuvem, Última sessão, Tempo de jogo, Conquistas) em cápsula de vidro; AcoesDoJogo (pílula Jogar + favoritar, ⋯, engrenagem em vidro redondo).
- Cartões de baixo: CartaoSobre, CartaoTrailer (miniatura parada, toca só ao clicar), CartaoDetalhes (ícone por linha, sem repetir o tempo de jogo; coluna de rótulos com largura mínima de 104 — corrige o "Desenvolv…"). Limites de texto e espaçamento de CartaoSobre e CartaoDetalhes conforme a tabela "Limites" dos READMEs deles (descrição em até 4 linhas com "Mais", valor em uma linha de 24 caracteres com dica).
- Estados: EstadoJogando (Voltar ao jogo + Encerrar jogo), EstadoNaoInstalado (Instalar na loja), EstadoSemArte.
- CartaoSessoes na fileira de baixo (depende do SESSION-002, que já existe) e EtiquetasDoJogo (LIB-002).
- Ícones novos em `icons.slint`: `clock`, `calendar`, `user`, `tag`, `layers`, `chevron-down`, `pencil`, `trophy`, `cloud`, `list`, `tick`, `updown` (desenhos na pasta Ícones).
- **Pronto quando:** os quatro estados do destaque batem com as prévias.

## Etapa 7 — Camadas

- `DeskOverlay`: Dialogo, PainelLateral, MenuDeContexto e Aviso no vidro de camada (`glass-panel-dense`, `glass-edge-strong`, `shadow-layer`); linhas de painel agrupadas (`dk-group`), nunca caixas soltas.
- Primeira abertura: TelaCarregamento (janela do atualizador, UPD-001; o polvo em camadas — anel, corpo e 7 tentáculos — animado pelo progresso), depois TelaOnboarding → OnboardingEmuladores (SRC-001/002) → OnboardingCapas (IGDB ligada ou não) → OnboardingModo (grava "Abrir em": janela ou tela cheia) → OnboardingPronto, envolvendo o `first_run.rs`.
- SeletorDeImagens (ART-002): popup grande no lugar da tela de imagens atual — X sempre visível, Capa/Fundo/Logo/Ícone em seletor segmentado, Fonte e Estilo em `DeskSelect`, grade que preenche a largura (`minmax(132, 1fr)`), prévia fixa de 380 com Usar / Voltar à original / Cancelar.
- Telas que já existem no código e mudam só de visual: ConfiguracoesDoJogo (`game_settings.rs`), AssistenteInicial com indicador de passos (`first_run.rs`), ErroDoJogo (aviso de fim anormal, `closing_game.rs` / `session`).
- **Pronto quando:** as quatro camadas e as três telas batem com as prévias; Esc fecha a camada de cima; o diálogo escurece o fundo e o painel lateral não.

## Etapa 8 — Configurações

- `DeskSettings`: a mesma estrutura da biblioteca sobre `settings-bg`: painel de categorias (ícones sem fundo, seleção na segunda camada de vidro), cabeçalho da categoria, `DeskRow` em grupos com divisórias e cabeçalhos em `overline`; transição arte → cinza em `color-time`.
- `DeskSelect` (CampoDeSelecao): pílula + `PopupWindow` com a lista. Toda escolha de uma opção entre várias usa ele — inclusive Tela de jogo (tela, resolução e taxa) e os filtros/ordenação do PainelLateral. Tirar das descrições os textos "← → trocam…" e o ‹ › do painel; com a lista fechada, ↑ ↓ não mudam o valor. Mudança de tela ou resolução confirma com "Manter esta configuração? Volta em 15 s".
- `DeskSlider` (ControleDeslizante) no lugar do ‹ barra › de Sons: arrastar, clique na trilha, roda do mouse, ← → 5% (Shift 10%), alto-falantes que silenciam e vão a 100%, valor escrito, som de amostra ao soltar.
- Controles com bateria (TelaControles). Capas e metadados ganha a FilaDeCapas (ART-001/002/003). "Reduzir transparência" entra em Acessibilidade.
- **Pronto quando:** TelaConfiguracoes e TelaControles batem com as prévias.

## Etapa 9 — Acessibilidade, foco e movimento
- Transições conforme os storyboards do grupo Movimento: troca de jogo (texto 120/200; arte nova **por cima** da antiga em 700 assentando 1,04 → 1 com desfoque saindo, a antiga opaca embaixo até o fim — nunca pisca; só começa com a imagem decodificada; debounce de 150 ms no teclado), guias (280, 16 px na direção da guia), abas internas (pílula 160), modo de exibição (duas levas), popups (280/200, 0,98 → 1), painel lateral (280 pela direita), aviso (280 · 4 s · 450), menus (160 do ponto), dicas (500 + 120), Jogar (pointer-down 0,97), passos do onboarding (280, 24 px). Toda transição interrompível a partir do estado atual.

- Foco suave (`focus-fill` + contorno `focus-ring` + brilho `focus-glow`) em todo elemento focável, sem mudar a geometria; hover só clareia.
- Durações, curvas e escalas pelos tokens de Tempo, Curva e Escala; Reduzir animações zera tudo menos o hover; nada anima com a janela minimizada.
- "Tamanho do texto" 110/125% sem quebrar a lista e o destaque.
- **Pronto quando:** a navegação só por teclado e só por controle passa por todas as telas; o contraste medido continua ≥ 4,5:1 (ver abaixo).

## Já no app (registro do relatório de mudanças)
- ListaDeJogos: linha de 44 (`game-row-height`), passo de 50, sem ícone da loja, vaga fixa de 36 para o ▶, botão + (Adicionar jogo) na contagem; "Adicionar jogo…" no MenuDoTako.
- PilulaDaLoja (StorePill) no canto superior direito do destaque.
- BarraDaJanela sem logo; BarraSuperior com `tako-logo-horizontal` 2× e hover de escala/opacidade.
- EntradaDoApp (passos de `intro-delay` 70 ms, `intro-rise` 12 px, `tempo-pagina` 280 na `curva-entrada`).
- TrocaDeJogo: vidro, luminância e accent em crossfade junto com a arte (700, `curva-estado`). **Falta:** troca entre jogo com arte e sem arte ainda é instantânea.
- DestaqueFases (fase 2 `GameMore`), tokens `tempo-fase` 460 e `tempo-fase-sai` 200, ícone `chevron-up`.
- **Rodada 71ad29a (app 0.2.0-alpha.14):**
  - BarraDaJanela unificada: `TopBar` de borda a borda com Minimizar · Maximizar · Fechar (`window-control-width` 46; `titlebar-height` saiu), hit-test `HTCAPTION`/`HTMAXBUTTON`/`HTCLOSE` no Windows; perfil como na Steam (menu Ver perfil · Trocar perfil… · Editar perfil) e Tela cheia só com o ícone de controle.
  - CampoDeBusca fora da barra: no painel da lista (40), centralizada em Grade/Categorias (420 / 280), no topo do painel das Configurações; Ctrl + K flutuante em Loja e Perfil.
  - BarraLateralUnida em Aparência › Layout (`layout.sidebar_unida`, Normal 380 / Estreita 336), um vidro só em L.
  - Contagem com padding 9/8 e filtros ativos em chips fixos; StatusDoJogo no canto do destaque, no PainelLateral, no menu de contexto e na SelecaoMultipla; primeira partida marca Jogando.
  - Fase 1 Sobre · Detalhes · Sessões e Metacritic na faixa; fase 2 sem repetir a fase 1, com Trailer, Detalhes completos e Imagens, vidro denso fixo.
  - AtualizacaoDoApp: item do menu com Procurando / Em dia / Erro, aviso com o menu fechado, diálogo Baixando/Pronta e volta à versão anterior se a instalação falhar.
  - `.dk-stack` no SeletorDeImagens; logos oficiais de todos os emuladores (também nas Configurações).
  - **Falta (sem fonte no app):** HowLongToBeat, Conquistas Steam, Preço (IsThereAnyDeal), Requisitos x seu PC e Guia de platina na fase 2 — os blocos não aparecem.
- **Fundo e vidro do desktop (lippdev/tako-launcher#166):**
  - O escurecimento da arte segue a regra do FundoDoJogo numa camada só, a mesma do fundo e de dentro do vidro; a elipse atrás do texto chega a 0 antes da borda, sem aresta dura.
  - O vidro desfoca a arte já escurecida, como o `backdrop-filter` do protótipo, e usa a tinta sólida (`glass-solid`) até a cópia desfocada chegar, trocando com cross-fade.
- **AtividadeNaBarra (lippdev/tako-launcher#175 e #176, UX-006):**
  - Pílula antes do perfil com a atividade mais importante e o "+N" (erro → download → biblioteca → atualização do Tako → o resto) e a lista Atividades (400, `glass-panel-dense`, rodapé Ver downloads / Limpar concluídas); crossfade de 160 ms (`focus-time`); giro e anel param com Reduzir animações; janela pequena só com o ícone e o "+N"; o leitor de tela ouve início, conclusão e erro (`accessible-live-region: polite`, sem a %).
  - Alimentam a fila: atualizar a biblioteca ("Steam · 3 de 5 lojas"), capas ("12 de 24 jogos"), downloads e instalações das lojas, procurar atualização com o menu fechado e "Atualização pronta"; o "↓ 42%" e a pílula "Atualização pronta" antigos saíram. Só no desktop: a tela cheia não tem a barra e continua com os avisos, inclusive "terminou de baixar".
  - Atualização do Tako com o menu fechado: "Procurando…", o resultado, a versão encontrada e o erro com "Tentar de novo" vão para a pílula; com o menu aberto, quem responde é o item do menu.
  - **Falta:** sincronizar a nuvem (o tipo existe na fila; não há nuvem no app); erro por loja da atualização da biblioteca e "Tentar de novo" dela (depende do lippdev/tako-launcher#174); erro de capas (a fila de metadados não informa falha).
  - Erro com ação: a ação fica na pílula e Aceitar (teclado e controle) ou o clique na linha da lista refaz o trabalho; fechar o erro é um gesto à parte (✕ da linha, → no teclado). A instalação de uma loja só vira "instalado" se o jogo ficou instalado. Clicar na pílula com a lista aberta fecha a lista.
- **Rebranding · Marca (lippdev/tako-launcher#180, UX-007 etapa 2):**
  - Barra de cima do desktop com o polvo 50×40 (brilho lilás embutido no PNG, porque o Slint só tem sombra de caixa), a palavra 68×14 (sombra embutida no PNG) e a kana 46×8 a 85%, mais o ⌄ do MenuDoTako. O resto da barra continua o de antes até a etapa 3.
  - Ícone do app (exe, .icns, Linux/Flatpak) e da janela do desktop: o polvo sobre uma placa escura arredondada (`canvas`, raio 22%), para não sumir em barra de tarefas e Dock claros.
  - Ícones do desktop (`desk-icons.slint`): 33 desenhos de `assets/Icones`, traço 1,7; os sem desenho novo (power, speaker, tv, layers, tag…) seguem os antigos. `check` mantém o círculo (ação) e `tick` é o ✓ simples (opção escolhida); ▶ só preenchido; alerta e erro usam o mesmo `badge-alert`, separados por cor e texto.
  - **Ficou de fora:** a tela cheia (splash, barra lateral e tema Fita) mantém o polvo antigo, porque o rebranding é só do desktop; ícones em 15 px nos menus ficam para a etapa de camadas; o ícone ainda não foi conferido em Windows, macOS e Linux reais.
- **Rebranding · Tokens, temas e cores da arte (lippdev/tako-launcher#178 e #179, UX-007 etapa 1):**
  - `Ui` com a paleta do TAKO, raios 7·9·10·11·16·18·999, `curva-entrada`, tempos do MovimentoBase (zeram com Reduzir animações, inclusive o hover) e sombras; cada cor tem a coluna dos cinco temas e a sólida do Liquid Glass (Reduzir transparência). Os apelidos `glass-*`, `settings-*` e `primary-*` levam os valores novos; a tela cheia lê os nomes dela (`on-accent*`, `warning`, `error`, `canvas`) e não muda, por isso o desktop ganhou `desk-text`, `desk-on-muted`, `desk-warning`, `desk-error`…
  - Configurações › Aparência (só no desktop): Tema do desktop (TAKO, Escuro, Meia-noite, Preto OLED, Liquid Glass) e, fora do TAKO, Cor de destaque (as nove + cor própria `#RRGGBB`); no Liquid Glass, arte como luz ambiente, opacidade e controles em pílula.
  - Cores da arte do jogo (TAKO, chave própria `DesktopArtColors`, ligada): amostra 64×36 na thread do vidro, matiz mais forte em faixas de 20°, saturação 62–92% e luz 50–58%; desliza em 1 s; desligado ou sem arte, pedra. Texto escuro `#14110f` sobre destaque claro pela luma perceptual (0,299/0,587/0,114), como nas prévias: pela luminância WCAG a pedra ficaria com texto branco a 1,7:1.
  - O destaque pinta a chave ligada, o progresso e o item escolhido.
  - **Ficou de fora:** o deslizante de desfoque do Liquid Glass (a cópia desfocada em cache tem raio fixo; volta com a barra e as camadas); a pílula só vale nos controles com `radius-control`, os raios fixos viram pílula conforme cada tela for refeita; tamanhos (`topbar-height` 52, `primary-height` 46, linhas de 64…) entram nas etapas de estrutura; TelaTema com cartões de prévia e bolinhas de cor fica para a etapa das Configurações (hoje são seletores).
- **Rebranding · Barra de cima e Downloads (lippdev/tako-launcher#181 e #182, UX-007 etapa 3):**
  - BarraSuperior de 52 de borda a borda: marca, aba BIBLIOTECA (Poppins 12/400, 0,32 em) com o sublinhado em tentáculo, Downloads, perfil como avatar com anel na cor de destaque e ponto verde, Tela cheia, divisória (8 depois) e Minimizar · Maximizar · Fechar de 40 com ícones de 15; linha de baixo mantida. Hit-test do Windows em 52/40. No TAKO, tinta lilás sobre a cópia desfocada em cache e cores do logo fixas (não segue a arte); sem desfoque com Reduzir transparência; nos outros temas, `#070707` opaco com linha.
  - Downloads no lugar da pílula da AtividadeNaBarra: contador na cor de destaque que pula (1,35 em 400 ms, zera com Reduzir animações) e conta só o que está em andamento; selo vermelho "!" no erro e ponto verde com atualização pronta (o nome acessível diz "Downloads, 1 ativo, 1 erro"); Aceitar e clique sempre abrem o popover de 340, onde a linha do erro refaz. O resultado (concluída / erro) vai para o Aviso, uma vez por evento. A fila de atividades por trás é a mesma.
  - **Ficou de fora:** a aba LOJA (o app não tem a Loja); abas inativas a 55% em vez de 45% (contraste ≥ 4,5:1); brilho do texto e sombra do tentáculo (o Slint não tem sombra de texto); capa, tamanho em GB e Pausar/Continuar nas linhas do popover (a fila não carrega esses dados; pausar fica no painel completo); o Aviso usa os 4 s do app em vez de 2,3 s, porque mudaria todos os avisos do desktop; "Efeitos de vidro" é o Reduzir transparência que já existe.
- **Rebranding · Lista da biblioteca (lippdev/tako-launcher#183 e #184, UX-007 etapa 4):**
  - ListaDeJogos fixa de 292 sob a barra (TAKO sem fundo nem divisória; outros temas com superfície e linha; Liquid Glass em vidro), conteúdo a 40 dela. Abas Todos · Favoritos · Instalados (largura pelo texto + 8, para caber a 125%), busca de 38 com "Ordenar, ver e filtrar" (contagem, ordem, modo, filtros, coleções, Adicionar jogo, Atualizar biblioteca); a coleção ativa vira um chip no lugar da Visão geral. Topo da lista é uma zona de foco: teclado e controle chegam ao menu, ao recolher e ao chip (Enter abre o seletor, Esc no chip sai da coleção).
  - Linha de 42 com ícone de 28 (loja → executável → inicial, raio 7/28), ▶ com nome e dica. Recolher com Ctrl + B ou botão: 76 com ícones de 46 (96 px no cache), 500 ms, salvo por perfil; Ctrl + K e a lupa expandem e focam a busca. Grade de três capas 2:3 dentro da lista. Não instalados com ícone escurecido e nome apagado. ListaVazia com o polvo.
  - Saíram as opções "barra lateral unida" e "largura da barra lateral" (BarraLateralUnida); a linha de baixo da barra fica sempre visível, também sobre a lista.
  - **Ficou de fora:** Categorias seguem em tela larga; "Com atualização" e o ponto de atualização na linha (o app não tem o dado); ordenar por Nota/Data/Tamanho só no painel Filtrar e ordenar; janela pequena não recolhe sozinha; busca geral em painel de 560 (Ctrl + K só expande a lista); ícones recolhidos instalados a 55% em vez de cinza (o Slint não filtra cinza ao vivo); escada ao filtrar e o pisca do jogo novo.
- **Rebranding · Visão geral (lippdev/tako-launcher#190 e #192, UX-007 etapa 5):**
  - O desktop abre na TelaBiblioteca nova (a linha "Visão geral" da lista volta para ela; abrir um jogo, links `tako://`, a volta do jogo e a troca vinda da tela cheia continuam indo para o jogo). Título com contagem; Destaque de 400 com o último jogo (dados embaixo à esquerda; situação, logo ou nome, descrição e o primário embaixo à direita, como dizem os READMEs) e Voltar a jogar (300) com os quatro seguintes.
  - Peças novas reutilizáveis: BotaoDesktop, GrupoDeBlocos/Bloco, barra de dados e CapaDoJogo, só com tokens `Ui.*`.
  - Sua semana só das sessões gravadas desta semana (some sem sessões); Precisa de atenção com erros de segundo plano ("Tentar de novo"), jogo sem arquivos e instalação em andamento, mais uma linha agregada "N jogos sem capa" (some sem nada); fileira de Instalados com ‹ › que só rolam, capas 2:3 numa textura só de 400×600.
  - Abaixo de 760 de largura útil o Voltar a jogar some e sai também do caminho do teclado.
  - **Ficou de fora:** Conquistas, Capturas, "Atualizar" e erro de capas/nuvem (sem fonte no app); a caixa tracejada da ListaVazia (o Slint não tem traço tracejado: contorno `line-strong`); sombra de texto; rótulos abaixo de 12 da prévia sobem para 12. **Para o design corrigir:** a prévia da TelaBiblioteca põe a situação e o logo à esquerda, mas os READMEs pedem identidade e ações à direita (o app segue os READMEs); o CapaDoJogo diz "600×900 (3:4)", que é 2:3.
- **Rebranding · Configurações (lippdev/tako-launcher#185, #186, #189 e #191, UX-007 etapa 7):**
  - Categorias fixas de 292 (ícone num quadrado de 28 que fica branco no item atual), conteúdo de até 900 com cabeçalho de ícone 52 + título 28/700 e grupos de linhas de 64 com contorno; busca no estilo da lista. A rota `settings:<categoria>/<linha>` abre a categoria e faz a linha piscar na cor de destaque (1,6 s; não pisca com Reduzir animações); "Gerenciar fontes" cai em Downloads › Adicionar fonte.
  - CampoDeSelecao com o título da linha na lista e ✓ na atual; ControleDeslizante de trilha fina com o valor escrito (←/→ 5%, Shift 10%, Home/End); chave de 46×28 na cor de destaque.
  - Categoria Tema (só no desktop): cartões dos cinco temas, bolinhas de 34 com dica do nome + cor própria e código (fora do TAKO), opções do tema e a Prévia (fora da busca e do foco). Em Aparência, a linha do tema da tela cheia virou "Tema da tela cheia".
  - FilaDeCapas com botões contornados; TelaControles com a bateria corrigida (a barrinha ficava vazia e aparecia sem controle).
  - **Ficou de fora:** preço na Prévia (o app não tem loja); traço tracejado de "Sem capa" (o Slint não tem); bateria dos controles que não estão em uso (o app só lê a do controle ativo); os alto-falantes de silenciar/volume máximo do deslizante saíram como na spec (teclado: Home/End).
- **Rebranding · Perfil e onboarding (lippdev/tako-launcher#187 e #188, UX-007 etapa 10):**
  - TelaPerfil: avatar de 88 com anel na cor de destaque à esquerda, nome em Poppins e linha de status, Trocar perfil e Editar perfil à direita; abas Visão geral · Sessões · Coleções. Visão geral em GrupoDeBlocos (tempo jogado com a semana, mais jogado, jogos na biblioteca) e Sessões recentes; Sessões com o seletor de período; Coleções em grade com mosaicos 2×2. Teclado: barra → ações → abas.
  - Onboarding do desktop no visual de TelaOnboarding: mural de capas inclinado, marca nova, PassosDoOnboarding ("n de 5" + traços), BotaoDesktop primário, os cinco passos que o app já tem (lojas, emuladores, capas, modo, pronto) com os números reais da primeira leitura; a barra de cima some durante o assistente (os botões da janela ficam). Cabe em 960×600 com texto a 125%.
  - A tela de abertura do desktop usa o canvas do TAKO e a barra de progresso na pedra; a entrada da tela cheia continua preta.
  - **Ficou de fora:** PerfilFamilia (o app não tem perfis de família) e Conquistas (sem fonte); nenhum passo novo no onboarding.
- **Rebranding · Camadas (lippdev/tako-launcher#196, #197 e #198, UX-007 etapa 8):**
  - Menus com mínimo de 220 (crescem com o conteúdo), raio 12, `surface-raised`, `line-strong`, `shadow-float`, ícones de 15, abrindo da âncora (150/200 ms). Dica com espera de 500 ms (mantida com Reduzir animações) e transição de 150. Aviso embaixo no centro, claro com texto escuro (tokens `toast-*`): 2,3 s na confirmação curta, 8 s quando tem ação ("Tentar de novo", "Desfazer"); a tela cheia continua com os dela (2,6 s).
  - PainelLateral de 400 (480 nas ConfiguracoesDoJogo) entrando de fora em 450 ms, com véu de 50% (25% nas configurações do jogo); Diálogo de 480 (S 440 / M 540 / L 680), raio 18, `shadow-layer`, véu de 60%, sobe 12 e 0,98 → 1. Diálogo com imagem (ErroDoJogo) em 540, SeletorDeImagens em 1180 com prévia de 380, CampoDeTexto e AtualizacaoDoApp no padrão novo.
  - Busca geral (CampoDeBusca) de 560 com Ctrl + K nas telas sem busca própria (na biblioteca o Ctrl + K continua abrindo a lista e a busca dela; nas Configurações, a busca delas): procura na biblioteca inteira, mostra capa (ou inicial), título, loja · situação e a etiqueta Biblioteca; ↑ ↓, Enter abre o jogo, Esc limpa e fecha; também pelo controle.
  - **Ficou de fora:** resultados da Loja na busca (o app não tem loja); AssistenteInicial e ConfiguracoesDoJogo só herdaram as medidas novas do painel e do diálogo.
- **Rebranding · Loja (UX-007 etapa 9):** não entrou nada, porque o app não tem a área da Loja nem fonte de dados para ela (preços, ofertas, lista de desejos, cupons). TelaLoja, TelaLojaExplorar, TelaListaDeDesejos, TelaJogoNaLoja, PopupOferta e PopupCupom ficam para quando existir a fonte (DEALS-001, Etapa 10 — Áreas novas); a aba LOJA da barra, a etiqueta Loja na busca geral e o preço na Prévia do Tema só aparecem a partir daí. A página do jogo já é a mesma para loja e biblioteca, faltando só o conjunto de ações da loja.
- **Rebranding · Página do jogo (lippdev/tako-launcher#195, que reúne #193 e #194, UX-007 etapa 6):**
  - Saiu a DestaqueFases: a página rola. Destaque de 560 a 700 com a arte de borda a borda sumindo na lista e no canvas; dados embaixo à esquerda (última sessão, tempo de jogo, Metacritic só com nota); situação, logo ou título, descrição e AcoesDoJogo (BotaoDesktop) embaixo à direita, com a coluna nunca mais estreita que as ações (cabe em 960×600 e a 125%). Barra fina de 62 que ganha fundo, capa e o primário ao rolar.
  - Corpo: Sobre (entrelinha 1,65), trailer parado que toca ao clicar, captura, Recursos, Sessões, Detalhes (com título; Situação por último) e Parecidos (mesmo gênero ou desenvolvedora na biblioteca, guardados por jogo). Cada bloco só com dado.
  - Teclado e controle: ↓ nas ações entra no corpo e rola, os itens do corpo (trailer, Ver todas, Parecidos) recebem foco e a página rola até eles; focar uma ação volta ao topo.
  - Troca de jogo em escada de 55 ms (500/650, sobe 14), arte nova por cima da antiga; estados jogando ("JOGANDO · n MIN"), baixando, não instalado, sem arte, arte clara e carregando.
  - **Ficou de fora:** radar de gênero, Requisitos, tempo para zerar e conquistas (sem fonte); desfoque 4 → 0 da troca, sombra de texto e desfoque ao vivo (o Slint não tem; a barra fina usa o canvas a 97%); chips Compartilhar/Windows, Atualizar e Pausar do download (sem equivalente no app).
- **Rebranding · Movimento (lippdev/tako-launcher#199, UX-007 etapa 11):** os tempos do desktop passam pelos tokens do MovimentoBase (uma curva, crossfade de arte em `ease`, arte 800 + 1,8 s); troca de vista com fade e escala 0,985 (450/550), interrompível; diálogo em 450; um teste varre `desktop.slint` e `desk-*.slint` e reprova qualquer duração que não zere com Reduzir animações (aceita `on ? Ui.x : 0ms`). Saiu o recorte em L do vidro flutuante e o que restava da BarraLateralUnida. **Ficou de fora:** a TrocaDeModo (desktop ↔ tela cheia) não mudou, porque mexe no lado da tela cheia. **Com isso o UX-007 está concluído** (a Loja segue fora, sem área no app).

## Correções
- **Escurecimento sem faixa no meio:** lateral que só desce + elipse atrás do texto (FundoDoJogo › Escurecimento); some a "sombra" sobre arte clara.

## Propostas desta rodada
- **Contagem da lista:** padding 9/8 (texto na coluna da miniatura, último ícone no centro do ▶); filtro ativo sem número, com chips fixos abaixo da contagem.
- **Barra superior unificada** com a barra da janela (ver BarraDaJanela); sem busca; perfil como na Steam (avatar + nome ⌄, sem saldo) e Tela cheia só com o ícone de controle.
- **Busca:** embaixo das abas no painel da lista; flutuante e centralizada em Grade/Categorias; no topo do painel em Configurações; Ctrl + K flutuante em Loja e Perfil (CampoDeBusca).
- **BarraLateralUnida:** opção em Aparência › Layout.
- **Fase 1:** fileira Sobre · Detalhes · Sessões; Metacritic entra na faixa de números do destaque; o Trailer vai para a fase 2.
- **Lista com ícone do jogo:** a linha usa o ícone (28, como na Steam; reserva: ícone do .exe, depois a inicial), não a capa; capa só na Grade.
- **NaoInstalados:** seção por loja na lista e na grade (grupos que abrem e fecham), ⬇ Instalar na linha e no centro da capa, anel de progresso, pílula Instalar com loja e tamanho, botão dividido para escolher a loja.
- **StatusDoJogo:** status do usuário (Jogando, Quero jogar, Pausado, Zerado, Abandonado) como select no canto superior direito do destaque, à esquerda da pílula da loja; no PainelLateral como filtro.
- **DestaqueFases fase 2:** HowLongToBeat, Conquistas Steam, Preço, Trailer, Requisitos x seu PC, Guia de platina (PowerPyx), Detalhes completos e Imagens (sem repetir Sobre e Sessões); cartões sem desfoque ao vivo para a rolagem não travar.
- **AtualizacaoDoApp:** "Procurar atualizações…" responde no item (Procurando / Em dia / Erro); diálogo novo com novidades e Reiniciar agora como ação principal.

## Etapa 10 — Áreas novas (dependem do backlog)

Só depois das decisões de cada item: TelaLoja e TelaJogoNaLoja (DEALS-001, fonte de preços), TelaPerfil com as abas PerfilSessoes, PerfilColecoes e PerfilFamilia (UX-004, IDEA-SOC-10; o bloco Conquistas só com o agregador IDEA-SOC-02), PainelDownloads (DL-001). As prévias já definem o visual; a guia ou a pílula da barra só aparece quando a área existir.

## Como conferir

- Cada etapa compara a janela real com a prévia correspondente, na mesma resolução (1600×900 e 960×600), com a arte de Hades II e com uma arte azul/escura.
- Contraste: as prévias foram medidas texto a texto sobre o fundo real (vidro + arte, no ponto mais claro da caixa); a regra é ≥ 4,5:1, ou 3:1 em texto grande. O mesmo método vale para capturas da janela real.
- Desempenho (PRD §8): ≤ 150 MB em repouso, rolagem da lista sem queda de quadros, nada redesenhando com o jogo aberto. **Já feito no app:** uma textura por imagem, arte em degraus de 1080/1440/2160, cache de 32 MB, linhas em vagas fixas (5,7 → 3,8 ms por quadro), sem recorte arredondado em itens repetidos, nada redesenhado com a janela minimizada.
