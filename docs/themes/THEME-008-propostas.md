# THEME-008: Propostas de temas oficiais para o Tako

Este documento reúne 8 propostas conceituais de temas em tela cheia para o Tako Launcher (Rust + Slint 1.18), atendendo à primeira etapa do item **THEME-008** do PRD.

O objetivo do THEME-008 é oferecer cinco temas oficiais profundamente distintos entre si — indo muito além de simples trocas de paleta em `tokens.toml` —, explorando layouts, modelos mentais de navegação, identidades visuais e ritmos de animação variados para console na TV, handhelds e monitores de mesa.

---

## 1. Fita (Ribbon / Console Moderno)

### Ideia em uma frase
Uma fita horizontal contínua de cartões compactos ancorada no terço inferior da tela, deixando a arte imersiva do jogo reinar no topo com tipografia cinematográfica e HUD minimalista.

### Layout

#### Início
```text
+-------------------------------------------------------------------+
|  [Tako]                  19:45  [XBOX] 85%  (LB/RB alternam abas) |
|                                                                   |
|  TITULO DO JOGO EM DESTAQUE                                       |
|  RPG de Ação · 2024 · 42h jogadas                                 |
|                                                                   |
|  [ > JOGAR ]  [ i Detalhes ]                                      |
|                                                                   |
|  +-----+  +---------+  +-----+  +-----+  +-----+  +-----+         |
|  |     |  | EM FOCO |  |     |  |     |  |     |  |     |  ...    |
|  |  1  |  |  ZOOM   |  |  3  |  |  4  |  |  5  |  |  6  |         |
|  +-----+  +---------+  +-----+  +-----+  +-----+  +-----+         |
|  Recentes: 12 jogos                                               |
|  (A) Escolher  (B) Barra Lateral  (X) Opções  (Y) Buscar          |
+-------------------------------------------------------------------+
```

#### Biblioteca
```text
+-------------------------------------------------------------------+
|  [Tako] Biblioteca (128 jogos)             19:45  [XBOX] 85%      |
|  [Todos]  [Instalados]  [Favoritos]  [Filtros]  [Ordenar]         |
|                                                                   |
|  +---------+  +---------+  +---------+  +---------+  +---------+  |
|  |         |  |         |  | EM FOCO |  |         |  |         |  |
|  |  Capa   |  |  Capa   |  | + Borda |  |  Capa   |  |  Capa   |  |
|  |         |  |         |  | Brilho  |  |         |  |         |  |
|  +---------+  +---------+  +---------+  +---------+  +---------+  |
|  +---------+  +---------+  +---------+  +---------+  +---------+  |
|  |         |  |         |  |         |  |         |  |         |  |
|  |  Capa   |  |  Capa   |  |  Capa   |  |  Capa   |  |  Capa   |  |
|  +---------+  +---------+  +---------+  +---------+  +---------+  |
|                                                                   |
|  (A) Detalhes  (B) Voltar ao Topo/Sidebar  (X) Ações  (Y) Busca   |
+-------------------------------------------------------------------+
```

### Como a navegação por controle funciona
- **Início:** D-pad Esquerda/Direita navega na fita horizontal de jogos recentes; a fita desliza mantendo o item em foco ampliado (escala 1.12x) na posição central. D-pad Cima sobe o foco para os botões de ação do topo (`Jogar` e `Detalhes`). D-pad Baixo expande detalhes rápidos da sessão (estatísticas e notas da versão). `LB`/`RB` saltam entre as coleções da Início.
- **Biblioteca:** Navegação em grade tradicional com colunas definidas dinamicamente (`library-columns: 5`). D-pad Cima no topo da grade move o foco para a barra de chips de filtro/ordenação; Esquerda na primeira coluna abre a barra lateral colapsável.

### Identidade
- **Paleta:** Fundo cinza ardósia grafite profundo (`#0D0F14`), superfícies translúcidas com efeito vidro escurecido (`rgba(20, 24, 33, 0.65)`), anel de foco em branco pérola (`#F5F7FA`) com halo sutil da cor do jogo (`Ui.accent`).
- **Tipografia:** Títulos em **Poppins Bold / ExtraBold** (arrojada, arredondada e moderna); metadados, números e botões em **Plus Jakarta Sans Medium / SemiBold**.
- **Movimento:** Transições suaves com curva spring/cubic-bezier rápida (200ms); ao receber foco, o card se expande ligeiramente com elevação de sombra projetada e borda luminosa.

### O que a API de temas atual já permite
- Totalmente compatível com a API v1: consome `hero`, `home-rows`, `library-tiles`, `library-columns`, `library-chips` e `backdrop`.
- Componentes como `ControlCenter`, `OverlayView` e `RowView` são importados de `@tako/` para suporte a camadas, mas não fazem parte do contrato formal da API v1.
- Virtualização das capas por fileira já testada e suportada no Slint.

### O que faltaria na API
- Suporte a metadados de conquistas rápidas na struct `Hero` ou `Tile` para enriquecer a prévia sem abrir os Detalhes completos.

### Custo estimado
**P (Pequeno):** Estrutura muito próxima do modelo mental padrão do Slint no Tako, com esforço focado em refinar as proporções e as animações de zoom da fita.

---

## 2. Estante Clássica (Vitrine / Skeuomorphic)

### Ideia em uma frase
Uma estante de colecionador onde cada jogo se apresenta como uma caixa física de acrílico ou estojo de DVD alinhado sobre prateleiras de madeira nobre iluminadas por spots de luz.

### Layout

#### Início
```text
+-------------------------------------------------------------------+
|  TAKO COLLECTOR'S SHELF                    19:45  [GAMEPAD]       |
|                                                                   |
|  ==================== PRATELEIRA PRINCIPAL =====================  |
|      _ _ _         /=============\         _ _ _         _ _ _    |
|     |     |       /  EM FOCO    / |       |     |       |     |   |
|     | DVD |      /   SALTA DA  /  |       | DVD |       | DVD |   |
|     |  1  |     /   ESTANTE   /   |       |  3  |       |  4  |   |
|     |_ _ _|    |=============| /          |_ _ _|       |_ _ _|   |
|  ==============|=============|/=================================  |
|  [Spotlight]       SOMBRA PROJETADA                               |
|                                                                   |
|  ==================== CONTINUAR JOGANDO ========================  |
|     [Box 5]       [Box 6]       [Box 7]       [Box 8]             |
|  ===============================================================  |
|  (A) Pegar da Estante (Jogar)   (X) Examinar Capa   (B) Menu      |
+-------------------------------------------------------------------+
```

#### Biblioteca
```text
+-------------------------------------------------------------------+
|  MINHA COLEÇÃO -- TODAS AS PLATAFORMAS           19:45  [GAMEPAD] |
|  [Todas as Caixas] [Steam] [GOG] [Epic] [Emuladores]              |
|                                                                   |
|  --- Playstation 2 / PC DVD-ROM --------------------------------- |
|   |===| |===| |===| |===| |===| |===| |===| |===| |===| |===|     |
|   | 1 | | 2 | |[3]| | 4 | | 5 | | 6 | | 7 | | 8 | | 9 | | 10|     |
|  ===============================================================  |
|  --- Cartuchos & Clássicos -------------------------------------- |
|   [===] [===] [===] [===] [===] [===] [===] [===] [===] [===]     |
|  ===============================================================  |
|  (A) Abrir Estojo  (D-pad) Percorrer Prateleiras  (Y) Procurar     |
+-------------------------------------------------------------------+
```

### Como a navegação por controle funciona
- **Início:** D-pad Esquerda/Direita navega pelos estojos na prateleira; o estojo selecionado projeta-se para a frente em perspectiva simulada com reflexo de plástico no acetato. D-pad Cima/Baixo alterna entre as prateleiras de coleções. Botão `X` vira o estojo para exibir a contracapa com a sinopse.
- **Biblioteca:** Visual de estante infinita. D-pad Esquerda/Direita desliza lateralmente pelos volumes; D-pad Cima/Baixo muda o andar da prateleira. `LT`/`RT` rolam rapidamente de letra em letra (índice alfabético).

### Identidade
- **Paleta:** Tons quentes e orgânicos: fundo nogueira escura (`#1A1412`), prateleiras em carvalho escovado com iluminação âmbar difusa (`#E5A93C`), etiquetas de lombada em papel linho creme (`#F4EEDD`).
- **Tipografia:** Títulos em **Source Sans 3 SemiBold / Bold** (estilo editorial e catalogação de biblioteca de luxo); números e etiquetas em **Inter Medium**.
- **Movimento:** Física de peso e inércia com amortecimento suave ao parar; sombra projetada no chão da prateleira cresce proporcionalmente à elevação da caixa.

### O que a API de temas atual já permite
- `home-rows` mapeia perfeitamente o conceito de prateleiras horizontais com múltiplos `Tile`.
- A API gráfica do Slint suporta gradientes lineares e angulares para produzir reflexos e relevos de plástico realistas sem custo excessivo de shader.

### O que faltaria na API
- Propriedade de proporção de capa ou tipo de embalagem por jogo (ex: cartucho, estojo fino de CD jewel case, ou caixa DVD), permitindo renderizar geometrias personalizadas por plataforma.

### Custo estimado
**M (Médio):** Exige modelagem minuciosa em Slint dos gradientes de estojo de plástico, reflexos e texturas táteis de madeira/acrílico.

---

## 3. Lista & Cinema (Broadcast / Painel de Vídeo & Dados)

### Ideia em uma frase
Layout assimétrico focado em velocidade para grandes bibliotecas: lista vertical ultrarrápida à esquerda integrada a um painel de cinema panorâmico à direita com ficha técnica detalhada e visualizador de capturas.

### Layout

#### Início
```text
+-------------------------------------------------------------------+
|  TAKO BROADCAST                             21:00  [PAD] BAT: 90% |
|                                                                   |
|  [CONTINUAR]            +--------------------------------------+  |
|  > Hades II             |                                      |  |
|    Cyberpunk 2077       |        PAINEL DE CINEMA / ART        |  |
|    Silksong             |     (Arte Panorâmica / Vídeo)        |  |
|    Baldur's Gate 3      |                                      |  |
|    Elden Ring           +--------------------------------------+  |
|    Forza Horizon 5      Hades II: The Olympic Update              |
|    Metaphor: ReFantazio Supergiant Games · Roguelike · 2024       |
|    Ver todos (42)       [ > INICIAR AGORA ]   [ Guia & Conquistas ]|
|                         Tempo: 34h · Conquistas: 28/45 (62%)      |
|  (D-pad V) Rolar Lista  (D-pad >) Acessar Painel  (A) Jogar       |
+-------------------------------------------------------------------+
```

#### Biblioteca
```text
+-------------------------------------------------------------------+
|  BIBLIOTECA GERAL (380 JOGOS)               21:00  [PAD] BAT: 90% |
|  [Buscar: Y] [Filtro: Todos] [Ordem: Alfabetica]                  |
|                                                                   |
|  # A B C [D] E F G H I  +--------------------------------------+  |
|  > Dead Cells           | FOTO / BANNER DE ALTA RESOLUÇÃO      |  |
|    Death Stranding      |                                      |  |
|    Disco Elysium        +--------------------------------------+  |
|    Dishonored 2         Disco Elysium - The Final Cut             |
|    DOOM Eternal         "Um RPG inovador de mundo aberto..."      |
|    Dredge               Instalado no SSD (NVMe) · 22 GB           |
|    Dune: Spice Wars     Última sessão: Ontem às 22:15             |
|                                                                   |
|  (A) Ficha Completa  (X) Menu do Jogo  (B) Barra Lateral          |
+-------------------------------------------------------------------+
```

### Como a navegação por controle funciona
- **Início e Biblioteca:** D-pad Cima/Baixo percorre a lista vertical à esquerda com rolagem instantânea; a arte e as informações do painel direito atualizam-se sem engasgos por debounce visual. D-pad Direita move o foco da lista para as ações do painel direito (`Iniciar`, `Conquistas`, `Gerenciar`). D-pad Esquerda na lista abre a barra lateral ou salta para o índice alfabético.
- `LT`/`RT` atuam como Page Up/Down na lista vertical de jogos.

### Identidade
- **Paleta:** Fundo preto absoluto de broadcast (`#070709`), linhas divisórias cinza-chumbo sutis (`#1E222B`), destaque de seleção em ciano luminoso (`#00E5FF`) com texto de alto contraste (`#FFFFFF`).
- **Tipografia:** Títulos em **Inter Bold / ExtraBold** (alta legibilidade técnica para telas grandes e salas iluminadas); listas e estatísticas tabulares em **Inter Medium / Regular** com numerais alinhados.
- **Movimento:** Cross-fade rápido (120ms) na arte do fundo e revelação deslizante sutil dos textos à direita ao trocar de jogo na lista.

### O que a API de temas atual já permite
- Definindo `library-columns: 1`, a biblioteca do Tako passa a se comportar nativamente como uma lista vertical com controle por D-pad Cima/Baixo.
- Dados de `Tile`, `Hero` e `Details` suprem completamente o painel direito.

### O que faltaria na API
- Suporte a reprodução de trechos de trailer de vídeo (`mpv` ou container nativo) quando o jogo ficar em foco por mais de 1,5 segundo.

### Custo estimado
**M (Médio):** A lógica de layout de 2 colunas é direta no Slint, demandando cuidado na paginação suave da lista de 1 coluna sem gargalos de renderização.

---

## 4. Mosaico Editorial (Bento Grid / Estilo Revista)

### Ideia em uma frase
Uma composição gráfica assimétrica inspirada em publicações de design e no formato Bento Grid, destacando arte, estatísticas da semana e jogos favoritos em módulos de diferentes proporções.

### Layout

#### Início
```text
+-------------------------------------------------------------------+
|  TAKO MAGAZINE                              EDICAO DE OUTUBRO     |
|                                                                   |
|  +-----------------------------+  +--------------+  +----------+  |
|  | EM DESTAQUE (2x2)           |  | RECENTE (1x1)|  | TEMPO    |  |
|  | Metaphor: ReFantazio        |  | Balatro      |  | SEMANA   |  |
|  |                             |  +--------------+  | 18h 40m  |  |
|  | Atlus · Lançamento          |  +--------------+  +----------+  |
|  | [ > JOGAR ]  [ Detalhes ]   |  | RECENTE (1x1)|  +----------+  |
|  |                             |  | Animal Well  |  | 5 NOVO   |  |
|  +-----------------------------+  +--------------+  +----------+  |
|  +-----------------------------------------------+  +----------+  |
|  | BANNER COLEÇÃO: INDIES PREMIADOS (3x1)        |  | CONFIGS  |  |
|  +-----------------------------------------------+  +----------+  |
|  (D-pad) Navegação Espacial 2D   (A) Interagir   (Y) Buscar       |
+-------------------------------------------------------------------+
```

#### Biblioteca
```text
+-------------------------------------------------------------------+
|  ACERVO GERAL -- MOSAICO DINÂMICO                                 |
|  [Todos: 154] [Favoritos] [Terminados] [Filtro de Gênero v]       |
|                                                                   |
|  +--------------+  +----------------------------+  +-----------+  |
|  | Capa Padrão  |  | JOGO FAVORITO (Card Duplo) |  | Capa 1x1  |  |
|  | Celeste      |  | Elden Ring                 |  | Hollow K. |  |
|  +--------------+  | 180 horas · 100% Zerado    |  +-----------+  |
|  +--------------+  | [ Jogar Novamente ]        |  +-----------+  |
|  | Capa Padrão  |  +----------------------------+  | Capa 1x1  |  |
|  | Tunic        |  +----------------------------+  | Dead Cells|  |
|  +--------------+  | Banner: Jogos Instalados   |  +-----------+  |
|                                                                   |
|  (A) Ver Jogo  (X) Reordenar Bloco  (B) Menu Lateral              |
+-------------------------------------------------------------------+
```

### Como a navegação por controle funciona
- **Início e Biblioteca:** Navegação bidimensional livre via algoritmo de vizinho espacial (`spatial navigation`): ao pressionar Direita ou Baixo, o foco salta inteligentemente para o centroide geométrico do bloco vizinho mais próximo. O bloco em foco cresce ligeiramente e ganha uma moldura viva na cor de destaque do jogo.
- Pressionar `B` em qualquer bloco periférico à esquerda retorna à barra lateral unificada.

### Identidade
- **Paleta:** Tons escuros com textura de papel cartão fosco (`#121316`), cartões com cor de superfície sólida grafite (`#1C1E24`), detalhes em amarelo canário suave (`#FEE180`) e magenta contemporâneo (`#E94D8A`).
- **Tipografia:** Títulos em **Poppins ExtraBold** em caixa-alta estilizada; blocos de dados e estatísticas em **Plus Jakarta Sans Bold / Regular**.
- **Movimento:** Transição de arranjo com expansão elástica de blocos ao receber foco (spring animation); os elementos vizinhos reagem com leve micro-deslocamento.

### O que a API de temas atual já permite
- `home-rows` e `hero` conseguem fornecer a massa de dados para preencher os primeiros blocos da Início.
- Os tokens `Ui` suportam toda a parametrização de bordas, sombras e raios dinâmicos.

### O que faltaria na API
- O gerenciador de foco da Início no `tako-ui` atual é baseado em linha e coluna indexada (`home-row-focus` e `home-col-focus`). Um Bento Grid exige que o tema calcule coordenadas espaciais virtuais ou que o Shell exponha navegação 2D bidimensional arbitrária na Início.

### Custo estimado
**G (Grande):** Requer resolver a navegação espacial por controle entre blocos de alturas e larguras desiguais, garantindo que o jogador nunca caia em "becos sem saída" de foco.

---

## 5. Linha do Tempo (Chronicle / Diário de Bordo)

### Ideia em uma frase
Uma viagem cronológica pela sua trajetória como jogador, dispondo os títulos ao longo de um eixo temporal contínuo pontuado por anos, marcos de horas jogadas e datas marcantes.

### Layout

#### Início
```text
+-------------------------------------------------------------------+
|  CRÔNICA TAKO                               22:15  [CONTROLE 1]   |
|                                                                   |
|               OUTUBRO 2026                 SETEMBRO 2026          |
|  ------------------O-----------------------------O--------------  |
|                   / \                           / \               |
|                  /   \                         /   \              |
|        +---------------+             +---------------+            |
|        | HOJE, 14:20   |             | 28 SET, 21:00 |            |
|        | Hades II      |             | Cyberpunk     |            |
|        | 2h na sessão  |             | Missão final  |            |
|        | [ > RETOMAR ] |             | Concluído     |            |
|        +---------------+             +---------------+            |
|                                                                   |
|  Estatística do Mês: 42 horas de jogo · 3 títulos zerados         |
|  (D-pad < / >) Viajar no Tempo   (A) Jogar   (Y) Ir para Data     |
+-------------------------------------------------------------------+
```

#### Biblioteca
```text
+-------------------------------------------------------------------+
|  ARQUIVO HISTÓRICO                       ORGANIZAR: [ANO LANÇAMENTO]
|                                                                   |
|  <<< 2020 ------- 2022 ------- 2024 ------- [2025/2026] >>>       |
|                                                                   |
|  +-------------+  +-------------+  +-------------+  +----------+  |
|  | MARÇO 2025  |  | NOV 2025    |  | FEV 2026    |  | OUT 2026 |  |
|  | Monster H.  |  | GTA VI      |  | Death Str 2 |  | Hades II |  |
|  | Wilds       |  |             |  |             |  |          |  |
|  | Adicionado  |  | Adicionado  |  | Adicionado  |  | Jogando  |  |
|  +-------------+  +-------------+  +-------------+  +----------+  |
|                                                                   |
|  (LB / RB) Saltar Década/Ano   (D-pad) Percorrer   (A) Detalhes   |
+-------------------------------------------------------------------+
```

### Como a navegação por controle funciona
- **Início:** D-pad Esquerda/Direita percorre a linha do tempo horizontalmente; a câmera desliza suavemente sobre o trilho cronológico. O nó central em foco emite pulsos de luz e expande a ficha da sessão. D-pad Cima/Baixo alterna entre o nó principal e eventos secundários do mesmo período (ex: conquistas obtidas naquele dia).
- **Biblioteca:** `LB`/`RB` saltam grandes blocos temporais (anos ou décadas). D-pad Esquerda/Direita navega pelos jogos individuais ordenados pelo marco cronológico selecionado.

### Identidade
- **Paleta:** Fundo índigo espacial profundo (`#0A0D14`), linha temporal em degradê ciano estelar para lilás elétrico (`#00E5FF` → `#9D4EDD`), nós inativos em branco translúcido (`#FFFFFF33`).
- **Tipografia:** Títulos em **Source Sans 3 Light / SemiBold** (ar refinado, cronológico e científico); datas e contadores numéricos em **Inter Bold**.
- **Movimento:** Efeito de paralaxe espacial entre a linha central de tempo, as datas no fundo e as estrelas/partículas de fundo; aterrissagem elástica nos nós ao soltar o direcional.

### O que a API de temas atual já permite
- As strings de metadados (`meta`) e coleções existentes já carregam informações de data e tempo relativo.
- Renderização vetorial fluida de linhas e círculos via nós nativos do Slint.

### O que faltaria na API
- Exposição estruturada de campos de data no `Tile` (`last_played_at: string`, `release_year: int`, `added_at: string`). Atualmente, `Tile` expõe apenas `meta: string` e `accent: color`, exigindo parsing de string pelo tema para agrupar por data.

### Custo estimado
**M (Médio):** A apresentação gráfica da linha horizontal é acessível no Slint, mas a formatação de agrupamentos por data depende de tratamentos adicionais.

---

## 6. Console Retrô CRT (16-Bit Arcade & Scanlines)

### Ideia em uma frase
Uma homenagem vibrante aos fliperamas dos anos 90 e consoles 16-bit, combinando scanlines autênticas, curvatura de tubo CRT, roda de seleção arcade e estética comemorativa de cartuchos.

### Layout

#### Início
```text
+-------------------------------------------------------------------+
| * TAKO ARCADE SYSTEM *                      INSERT COIN [CREDIT 9]|
| ================================================================= |
|                                                                   |
|   /============\      MARQUEE SUPERIOR                            |
|  /  RODA ARCADE \     +----------------------------------------+  |
| | > STREET F. 6 < |   | [ LOGO DO JOGO EM PIXEL / ILUSTRAÇÃO ] |  |
| |   CHRONO T.   | |   +----------------------------------------+  |
| |   MEGA MAN X  | |   ANO: 2024 · 2 PLAYERS · CAPCOM              |
| |   SONIC 3     | |   HI-SCORE: 999.990                           |
|  \  CASTLEVANIA / |   PRESS 1P TO START!                          |
|   \============/      [ > PRESS (A) START ]   [ TEST MENU (X) ]   |
|                                                                   |
| ================================================================= |
|  [SCANLINES ON] [AUDIO FX: STEREO]       (D-pad) SELECT  (A) PLAY |
+-------------------------------------------------------------------+
```

#### Biblioteca
```text
+-------------------------------------------------------------------+
|  SELECT CARTRIDGE / SYSTEM: [ALL ARCADES]           LEVEL-01      |
|                                                                   |
|  ID   TITLE                     GENRE     SYSTEM       STATUS     |
|  ===============================================================  |
|  001  BLAZING CHROME            ACTION    STEAM        INSTALLED  |
| >002  CASTLEVANIA: SOTN         RPG       PSX/EMU      READY 1P<  |
|  003  CHRONO TRIGGER            RPG       SNES/EMU     SAVED      |
|  004  METAL SLUG 3              ARCADE    NEOGEO       HI-SCORE   |
|  005  STREET FIGHTER 6          FIGHT     STEAM        INSTALLED  |
|  006  TMNT: SHREDDER'S REVENGE  BEAT'EM   XBOX         INSTALLED  |
|                                                                   |
|  (A) INSERT COIN (PLAY)  (Y) QUICK SEARCH  (B) EXIT TO CAB        |
+-------------------------------------------------------------------+
```

### Como a navegação por controle funciona
- **Início:** A roda arcade vertical ("wheel") à esquerda gira com D-pad Cima/Baixo acompanhada por efeito de rolagem de tambor; cada mudança atualiza o marquee superior e a ficha técnica de estilo fliperama. Pressionar `A` dispara a clássica animação de "Game Start".
- **Biblioteca:** Menu de serviço tabular de alta velocidade, navegável verticalmente com D-pad Cima/Baixo. D-pad Esquerda/Direita troca a placa de sistema (categoria/emulador/loja) com troca instantânea de tela.

### Identidade
- **Paleta:** Fundo preto fosco de cabine arcade (`#0A0A10`), verde fósforo CRT vibrante (`#39FF14`), amarelo âmbar clássico (`#FFB000`) e detalhes em azul neon de arcade cabinet (`#00F0FF`).
- **Tipografia:** Títulos e tabelas usando **Plus Jakarta Sans Bold** e **Inter ExtraBold** estilizadas em alta densidade com aspecto geométrico sólido; suporte a fontes bitmap/pixel art opcionais empacotadas no tema.
- **Movimento:** Sem atenuações ou easings longos: movimentos secos e com resposta em 1 frame (zero lag), efeito de scanline estático e piscar intermitente de "Press Start" a 2 Hz.

### O que a API de temas atual já permite
- Criação de texturas de scanline usando repetições de linhas ou shaders simples no Slint.
- Consome sem qualquer fricção as propriedades `home-rows`, `library-tiles` e as variáveis de status de controle e bateria.

### O que faltaria na API
- Suporte a feedback sonoro (THEME-007): bips de navegação, som de moeda caindo ao iniciar e clique mecânico do direcional tornariam este tema imbatível. Na ausência de áudio, opera apenas com feedback visual.

### Custo estimado
**P (Pequeno):** A geometria de tabelas e a roda de títulos são extremamente leves de construir e virtualizar no interpretador do Slint.

---

## 7. Monólito Zen (Galeria de Arte OLED / Tipografia Suíça)

### Ideia em uma frase
Um manifesto de design minimalista para telas OLED: 90% da tela é a arte pura do jogo sem sobrecargas gráficas, combinada com tipografia suíça monumental que surge com o foco.

### Layout

#### Início
```text
+-------------------------------------------------------------------+
|                                                     23:00  TAK0   |
|                                                                   |
|                                                                   |
|        ARTE MONUMENTAL EM TELA CHEIA (SEM INTERFERÊNCIAS)         |
|             (Backdrop puro sem vinhetas pesadas)                  |
|                                                                   |
|                                                                   |
|                                                                   |
|  01                                                               |
|  SILKSONG                                                         |
|  Team Cherry -- Em breve -- 2025                                  |
|  [ Iniciar ]  Detalhes                                            |
|                                                                   |
|  01 Silksong    02 Hades II    03 Elden Ring    04 Metaphor       |
+-------------------------------------------------------------------+
```

#### Biblioteca
```text
+-------------------------------------------------------------------+
|  CATALOGUE                                          INDEX: 01-96  |
|                                                                   |
|  +-------+   +-------+   +-------+   +-------+   +-------+        |
|  |       |   |       |   |       |   |       |   |       |        |
|  | Pôster|   | Pôster|   |[PÔSTER|   | Pôster|   | Pôster|        |
|  |       |   |       |   | FOCO] |   |       |   |       |        |
|  |  01   |   |  02   |   |  03   |   |  04   |   |  05   |        |
|  +-------+   +-------+   +-------+   +-------+   +-------+        |
|                          Elden Ring                               |
|                          FromSoftware                             |
|                                                                   |
|  [Apenas contornos de 1px e preto absoluto entre os pôsteres]     |
|  (A) Visualizar  (D-pad) Selecionar  (B) Retornar                 |
+-------------------------------------------------------------------+
```

### Como a navegação por controle funciona
- **Início:** A tela inicial é uma galeria imersiva. D-pad Esquerda/Direita troca suavemente de obra de arte em tela cheia com cross-fade limpo. Pressionar `A` sobre a tipografia inicia o jogo diretamente. D-pad Baixo revela o sumário textual e histórico de notas.
- **Biblioteca:** Grade de pôsteres verticais de proporção cinematográfica separados por generosos espaços pretos vazios. O item em foco ganha uma moldura branca ultrafina de 1px e os itens vizinhos reduzem a opacidade para 40%, focando a atenção total na peça selecionada.

### Identidade
- **Paleta:** Fundo preto absoluto OLED (`#000000`), superfícies em cinza carvão imperceptível (`#0A0A0A`), tipografia em branco puro (`#FFFFFF`) e cinza neutro (`#7E7E7E`), com zero gradientes coloridos arbitrários.
- **Tipografia:** Títulos em **Poppins ExtraBold / Light** em contraste brutal de escala; textos e numeração precisa em **Plus Jakarta Sans Regular** com tracking largo.
- **Movimento:** Transições lentas e refinadas de cross-fade (350ms); sensação de contemplação artística e sofisticação de galeria internacional.

### O que a API de temas atual já permite
- Totalmente suportado pela API v1 atual (`backdrop`, `hero`, `library-tiles`, `library-columns`).
- Aproveita a aceleração gráfica do Slint com altíssima taxa de quadros e consumo de memória quase nulo.

### O que faltaria na API
- Opcionalmente, suporte a pôsteres verticais 1:1.5 de proporção cinematográfica pura sem corte lateral automático.

### Custo estimado
**P (Pequeno):** É o tema mais limpo e leve em termos de código Slint, destacando-se pelo refinamento tipográfico e alinhamentos de grade.

---

## 8. Deck Handheld (HUD Compacto & Ergonômico)

### Ideia em uma frase
Criado especificamente para telas portáteis de 7 a 8 polegadas (Steam Deck, ROG Ally, Legion Go), com alvos de toque gigantes, telemetria de bateria em evidência e navegação rápida por botões de ombro.

### Layout

#### Início
```text
+-------------------------------------------------------------------+
| [LB] INÍCIO      [RB] BIBLIOTECA       15:30  [BAT: 85% ~3h12]    |
| ================================================================= |
|  +-------------------------------------------------------------+  |
|  | ULTIMO JOGO JOGADO                                          |  |
|  | BALATRO                                   [ > CONTINUAR ]   |  |
|  | Sessão anterior: há 20 min                (Botão Gigante)   |  |
|  +-------------------------------------------------------------+  |
|                                                                   |
|  JOGAR A SEGUIR:                                                  |
|  +--------------+  +--------------+  +--------------+             |
|  | [CAPA GIG]   |  | [CAPA GIG]   |  | [CAPA GIG]   |             |
|  | Dead Cells   |  | Slay Spire   |  | Dave Diver   |             |
|  +--------------+  +--------------+  +--------------+             |
|  (A) Jogar  (X) Detalhes  (Y) Teclado Virtual  [Menu: Central]    |
+-------------------------------------------------------------------+
```

#### Biblioteca
```text
+-------------------------------------------------------------------+
| [LB] INÍCIO      [RB] BIBLIOTECA       15:30  [BAT: 85% ~3h12]    |
| [X] Filtro: Instalados (24)   [Y] Teclado Busca   [*] Ordem: Rec. |
|                                                                   |
|  +----------------------+  +----------------------+               |
|  | [CAPA] Hades II      |  | [CAPA] Cyberpunk     |  (Grade 3     |
|  | Inst: SSD · 12 GB    |  | Inst: MicroSD · 70 GB|   colunas     |
|  +----------------------+  +----------------------+   para        |
|  +----------------------+  +----------------------+   leitura     |
|  | [CAPA] Silksong      |  | [CAPA] Balatro       |   a 40 cm)    |
|  | Inst: SSD · 8 GB     |  | Inst: SSD · 1.2 GB   |               |
|  +----------------------+  +----------------------+               |
|  (A) Ações Rápidas  (D-pad) Selecionar  (Toque) Toque Direto      |
+-------------------------------------------------------------------+
```

### Como a navegação por controle funciona
- **Início e Biblioteca:** `LB`/`RB` mudam de página instantaneamente (Início ↔ Biblioteca ↔ Configurações) sem necessidade de transitar pela barra lateral. D-pad navega com passos largos entre cards grandes. Todos os botões na tela têm tamanho mínimo de 72px físicos para toque com os polegares nas telas handheld.
- Botão `Start`/`Menu` ou toque no ícone de bateria abre a Central de Controle com controles de brilho, TDP e áudio do Tako.

### Identidade
- **Paleta:** Contraste robusto para visibilidade sob luz solar ou ambientes externos: azul cobalto industrial (`#1E40AF`), superfícies sólidas grafite escuro (`#181A20`), textos em alto contraste (`#FFFFFF` e `#E2E8F0`), indicador de bateria com gradiente verde/amarelo/vermelho dinâmico.
- **Tipografia:** **Inter Bold** em tamanhos aumentados em 130% em relação ao desktop para máxima legibilidade à distância de braço; rótulos de botões em **Plus Jakarta Sans Bold**.
- **Movimento:** Transições rápidas e econômicas (100ms) sem efeitos de blur ou transparências caras, priorizando 60fps constantes e economia de bateria da APU.

### O que a API de temas atual já permite
- As variáveis `pad-battery`, `clock`, `input-icon` e `hints` já entregam a telemetria necessária.
- Áreas de clique (`TouchArea`) mapeadas com suporte tátil total.

### O que faltaria na API
- Mapeamento direto de `LB` e `RB` no nível da página do tema para alternar entre as rotas principais (`home` e `library`) sem passar pelo callback de clique na Sidebar.

### Custo estimado
**P (Pequeno):** Layout muito funcional e de fácil estruturação no Slint, com ganhos ergonômicos imediatos em PCs portáteis.

---

## 3. Matriz comparativa das propostas

| # | Proposta | Metáfora Central | Dispositivo-Alvo | Tipografia Principal | Complexidade Slint | Faltas na API Atual | Custo |
|---|---|---|---|---|---|---|:---:|
| 1 | **Fita (Ribbon)** | Console de sala moderno | TV / Handheld | Poppins / Plus Jakarta | Baixa | Nenhuma crítica | **P** |
| 2 | **Estante Clássica** | Caixas físicas de colecionador | TV / Desktop | Source Sans 3 / Inter | Média | Geometria por mídia | **M** |
| 3 | **Lista & Cinema** | Broadcast com painel de dados | TV / Desktop | Inter | Média | Vídeo embutido | **M** |
| 4 | **Mosaico Editorial** | Bento Grid / Revista visual | TV / Monitor | Poppins / Plus Jakarta | Alta | Foco 2D não-linear | **G** |
| 5 | **Linha do Tempo** | Crônica cronológica de sessões | TV / Desktop | Source Sans 3 / Inter | Média-Alta | Campos de data no Tile | **M** |
| 6 | **Console Retrô CRT** | Arcade dos anos 90 / Scanlines | TV / Handheld | Plus Jakarta / Inter | Baixa | Sons de navegação | **P** |
| 7 | **Monólito Zen** | Galeria de arte pura para OLED | TV OLED / Monitor | Poppins / Plus Jakarta | Baixa | Nenhuma | **P** |
| 8 | **Deck Handheld** | HUD ergonômico portátil | Handhelds (Steam Deck) | Inter | Baixa | Abas via LB/RB | **P** |

---

## 4. Recomendação: quais 5 escolher e por quê

Para compor o conjunto dos **cinco temas oficiais do Tako**, recomendamos escolher propostas que:
1. Atendam aos principais perfis de usuários do Tako (jogadores de TV na sala, donos de handhelds, entusiastas de retrô e colecionadores com grandes bibliotecas);
2. Ofereçam contrastes de navegação e layout verdadeiramente radicais (evitando variações da mesma fórmula);
3. Possuam viabilidade imediata na API v1 de temas, permitindo entrega rápida e alta estabilidade.

### Os 5 temas recomendados

1. **Fita (Ribbon / Console Moderno)**  
   *Por quê:* É o padrão de ouro que qualquer usuário de console de última geração (PS5, Apple TV) espera encontrar. Serve como o tema de sala de estar mais acessível, cinematográfico e familiar, com custo de implementação baixo.

2. **Deck Handheld (HUD Compacto)**  
   *Por quê:* O Tako tem como público prioritário donos de Steam Deck, ROG Ally e Legion Go (PRD §1.2). Um tema dedicado a telas pequenas de 7 polegadas com botões gigantes e foco em economia de bateria e legibilidade é essencial para a identidade multiplataforma do launcher.

3. **Lista & Cinema (Broadcast / Lista de Alta Densidade)**  
   *Por quê:* Resolve o calcanhar de Aquiles dos launchers de console: bibliotecas massivas (500 a 2.000+ jogos). Navegar em grades nessas bibliotecas é exaustivo; a lista vertical de rolagem ultrarrápida com painel de detalhes rico oferece uma alternativa ergonômica fundamental.

4. **Console Retrô CRT (16-Bit Arcade)**  
   *Por quê:* O Tako integra emuladores nativamente (MIG-019). Oferecer um tema autêntico de arcade e console clássico com scanlines e roda de títulos celebra a herança dos jogos e cria enorme conexão emocional com o público retrogamer.

5. **Estante Clássica (Vitrine / Colecionador)**  
   *Por quê:* Traz calor, humanidade e apelo tátil com suas prateleiras de madeira e estojos físicos em 3D. Diferencia o Tako de praticamente todos os outros launchers contemporâneos (Steam Big Picture, Playnite, Pegasus), que tendem a ser frios e planos.

### Por que deixar as outras 3 para fases posteriores?
- **Mosaico Editorial:** Embora visualmente atraente, exige uma navegação espacial 2D não-linear que entra em conflito com o modelo mental de linhas e colunas indexadas da API v1 do Tako (`home-row-focus`), acarretando custo alto (G).
- **Linha do Tempo:** Depende de metadados tipados de data e ano ainda não padronizados no struct `Tile` da API pública.
- **Monólito Zen:** Seu layout e princípios podem ser em grande parte incorporados como uma variação de alto contraste ou minimalista do tema **Fita**.
