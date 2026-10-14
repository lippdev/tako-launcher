---
version: alpha
name: UltraLauncher Default Theme
description: |
  Tema padrão do UltraLauncher (MVP-013). Interface de sofá, 10 pés, controlada por joystick.
  A arte do jogo em foco define a cor da tela inteira: o fundo assume o tom de destaque do jogo
  e as superfícies são camadas brancas translúcidas sobre ele. Referência de canvas: 1920x1080,
  escalada uniformemente (Viewbox) para outras resoluções.

colors:
  # Tom de destaque padrão; em uso real vem do jogo em foco (game.accent).
  accent: "#C8102E"
  accent-fallback: "#2B3A55"
  # Derivados do accent (calculados em runtime, aqui os valores para o fallback vermelho).
  accent-deep: "#7A0A1C"
  canvas: "#0E1116"
  on-accent: "#FFFFFF"
  on-accent-muted: "#FFFFFFB8"
  on-accent-subtle: "#FFFFFF80"
  surface-1: "#FFFFFF1F"
  surface-2: "#FFFFFF33"
  surface-sidebar: "#FFFFFF2E"
  surface-solid: "#FFFFFF"
  on-surface-solid: "#11141A"
  focus-ring: "#FFFFFF"
  scrim: "#00000099"
  success: "#3DDC84"
  warning: "#FFB020"
  error: "#FF5A5F"
  store-steam: "#1B2838"
  store-epic: "#2A2A2A"
  store-gog: "#7A3FD1"
  store-manual: "#3D5A80"

typography:
  display:
    fontFamily: Inter
    fontSize: 72px
    fontWeight: 800
    lineHeight: 1.1
    letterSpacing: -0.03em
  headline-lg:
    fontFamily: Inter
    fontSize: 56px
    fontWeight: 800
    lineHeight: 1.0
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Inter
    fontSize: 34px
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: -0.01em
  title-md:
    fontFamily: Inter
    fontSize: 26px
    fontWeight: 600
    lineHeight: 1.2
  body-lg:
    fontFamily: Inter
    fontSize: 26px
    fontWeight: 400
    lineHeight: 1.45
  body-md:
    fontFamily: Inter
    fontSize: 22px
    fontWeight: 400
    lineHeight: 1.45
  label-lg:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: 600
    lineHeight: 1.2
  label-md:
    fontFamily: Inter
    fontSize: 20px
    fontWeight: 600
    lineHeight: 1.2
  label-sm:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: 500
    lineHeight: 1.25
  overline:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: 0.16em
  glyph:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: 800
    lineHeight: 1

rounded:
  sm: 8px
  md: 14px
  lg: 22px
  full: 999px

spacing:
  xxs: 4px
  xs: 8px
  sm: 16px
  md: 24px
  lg: 32px
  xl: 48px
  xxl: 64px
  safe-x: 72px
  safe-y: 48px

motion:
  focus: 160ms
  page: 280ms
  color: 600ms
  easing: cubic-bezier(0.22, 1, 0.36, 1)
  focus-scale: 1.08

components:
  sidebar:
    backgroundColor: "{colors.surface-sidebar}"
    width: 400px
  sidebar-collapsed:
    width: 120px
  sidebar-item:
    textColor: "{colors.on-accent}"
    typography: "{typography.label-lg}"
    rounded: "{rounded.md}"
    height: 64px
    padding: 20px
  sidebar-item-selected:
    backgroundColor: "{colors.surface-solid}"
    textColor: "{colors.on-surface-solid}"
  sidebar-item-focused:
    backgroundColor: "{colors.surface-2}"
  button-primary:
    backgroundColor: "{colors.surface-solid}"
    textColor: "{colors.on-surface-solid}"
    typography: "{typography.label-lg}"
    rounded: "{rounded.full}"
    height: 76px
    padding: 44px
  button-secondary:
    backgroundColor: "{colors.surface-2}"
    textColor: "{colors.on-accent}"
    typography: "{typography.label-lg}"
    rounded: "{rounded.full}"
    height: 76px
  game-card:
    rounded: "{rounded.md}"
    width: 204px
    height: 306px
  game-card-label:
    textColor: "{colors.on-accent}"
    typography: "{typography.label-md}"
  game-card-meta:
    textColor: "{colors.on-accent-subtle}"
    typography: "{typography.label-sm}"
  search:
    backgroundColor: "{colors.surface-1}"
    textColor: "{colors.on-accent-muted}"
    typography: "{typography.label-md}"
    rounded: "{rounded.full}"
    height: 60px
    width: 420px
  hint-bar:
    textColor: "{colors.on-accent-muted}"
    typography: "{typography.label-sm}"
    height: 72px
  glyph-button:
    backgroundColor: "{colors.surface-solid}"
    textColor: "{colors.on-surface-solid}"
    typography: "{typography.glyph}"
    rounded: "{rounded.full}"
    size: 34px
---

# UltraLauncher — tema padrão

## Overview

Launcher de jogos para TV e handheld, usado do sofá com controle. A referência visual é uma biblioteca estilo "app de mídia": barra lateral com a organização da biblioteca, um destaque grande com o jogo atual e fileiras de capas. A tela inteira assume a cor do jogo em foco, como se a arte "vazasse" para a interface.

Princípios:

- **Controle primeiro.** Todo elemento interativo é alcançável pelo direcional/analógico, e sempre existe exatamente um elemento em foco, visível de longe.
- **A arte é a protagonista.** O chrome é branco translúcido sobre o tom do jogo; nenhuma cor de marca compete com a capa.
- **Legível a 3 metros.** Nenhum texto abaixo de 18px no canvas de 1080p.
- **Implementável em WPF.** Só usar efeitos que o WPF reproduz sem custo alto (ver Do's and Don'ts).

## Colors

- **Accent (dinâmico):** cada jogo tem um `accent` extraído da arte (cor dominante saturada, pré-calculada na importação). O fundo é um gradiente de `accent` (topo-esquerda) para `accent-deep` (base). Ao trocar o foco, a cor anima em `motion.color`.
- **accent-deep:** o accent escurecido ~45% em luminância. Garante contraste do texto branco na metade de baixo.
- **Fallback (#2B3A55):** azul-ardósia para jogos sem arte ou com cor dominante pouco saturada.
- **Superfícies:** brancos com alpha (`surface-1` 12%, `surface-2` 20%, `surface-sidebar` 18%). Funcionam sobre qualquer accent.
- **Surface solid / on-surface-solid:** branco opaco com texto escuro, reservado à ação principal da tela (Jogar) e ao item ativo da barra lateral.
- **Contraste:** se o accent tiver luminância relativa > 0.35, escurecer até atingir 4.5:1 com branco antes de usar como fundo.

## Typography

Inter (SIL OFL), empacotada com o app. **Todo texto usa um token da escala acima**; nenhum tamanho avulso.

- **Título de jogo:** sempre `display` (72px), em qualquer tela (destaque, detalhes, jogando), com no máximo duas linhas. O tamanho não varia com o comprimento do nome. O bloco do destaque é ancorado embaixo, então um título de duas linhas cresce para cima e os botões nunca mudam de posição.
- **Descrição:** no máximo 3 linhas (`body-lg`).
- **Datas:** dd/mm/aaaa.

 Títulos de jogo usam `display` em todas as telas, peso 800, com tracking negativo, fazendo o papel do logo do jogo quando não há arte de logo. `overline` (caixa alta, tracking largo) marca contexto: "CONTINUAR JOGANDO", "INSTALADO".

## Layout

- **Canvas:** 1920x1080, escalado uniformemente. Área segura: `safe-x` 72px e `safe-y` 48px (overscan de TV).
- **Barra lateral:** 400px quando em foco; recolhe para 120px (só ícones) quando o foco vai para o conteúdo, liberando espaço para o destaque.
- **Conteúdo:** cabeçalho (título da página + busca), destaque (hero) ocupando ~55% da altura e fileiras horizontais de capas.
- **Início:** a primeira fileira é sempre **Jogados recentemente** (só jogos já jogados, do mais recente ao mais antigo), e o destaque abre no último jogado ("Continuar jogando"). Ao fim de uma sessão, o jogo volta para o primeiro lugar e a fileira volta ao início.
- **Barra de dicas:** rodapé fixo com os glifos do controle e a ação de cada botão na tela atual.
- **Cabeçalho fixo:** título da página, busca e status não rolam. O conteúdo rola numa moldura que esmaece antes do cabeçalho e antes da barra de dicas, então nada passa por trás desses textos.
- **Status do sistema:** canto superior direito, acima da busca: entrada atual (ícone de controle com o nome, ou teclado) e a hora. Aparece em todas as telas.
- **Grade da biblioteca:** capas 2:3 (204x306), 7 colunas quando a barra lateral está recolhida. O fundo e o accent acompanham a capa em foco, como na Início.

## Elevation & Depth

Profundidade vem de camadas de branco translúcido e de escala, não de sombra. A única sombra é a do elemento em foco (sombra preta 40%, blur 32px, deslocamento 16px), uma por vez. O hero usa gradientes de escurecimento (scrim) sobre a arte para destacar o texto.

## Shapes

Capas com `rounded.md` (14px); botões e busca em pílula (`full`); itens da barra lateral em `md`. Nada de cantos vivos.

## Components

- **Foco:** anel branco de 4px afastado 6px do elemento, escala `motion.focus-scale` em capas, transição `motion.focus`. Itens de lista não escalam: ganham fundo `surface-2`.
- **Game card:** capa 2:3, nome (`label-md`, uma linha com reticências) e loja + tamanho (`label-sm`) abaixo. Em foco, só a capa escala; o texto não se move. O destaque da Home troca para o jogo.
- **Fileiras:** o item em foco fica ancorado na primeira posição e a fileira rola por baixo dele, até o fim da lista.
- **Botão primário (Jogar):** pílula branca com ícone de play. Um por tela.
- **Barra de dicas:** glifos A/B/X/Y/LB/RB/Menu desenhados como círculos brancos com a letra; troca para teclas (Enter, Esc...) quando a última entrada foi teclado.
- **Menu rápido (Central de Controle):** aberto pelo botão Guia (Xbox/PS/Steam; tecla G no teclado) durante o jogo, no padrão da Central de Controle do PS5. Diferente do PS5, fica no **topo**: o jogo continua visível, escurecido de cima para baixo, e a hora e o controle descem para o canto inferior esquerdo enquanto o menu está aberto.
  - **Cartões vivos** (fileira abaixo da barra): Jogo (arte, tempo de sessão, total), Capturas (última captura ou convite), Desempenho ao vivo (FPS grande, gráfico dos últimos segundos, modo do HUD; A alterna o HUD), Áudio (volume e saída) e Controle (bateria). O cartão do jogo é maior e recebe o foco ao abrir.
  - **Barra de ícones** (no topo, centralizada): Início, o jogo (capa com ponto verde), Capturar, Desempenho, Áudio, Controle, Tela, Encerrar. O ícone em foco vira círculo branco e mostra o nome abaixo.
  - **Painel do ícone** (melhoria sobre o PS5, que troca de tela): A abre um painel de 600px logo abaixo da barra, alinhado ao ícone, com linhas grandes (marcador redondo, chave liga/desliga, barra de volume). O ícone de origem fica marcado e os cartões saem de cena (nunca dois painéis sobrepostos). B fecha o painel; o painel de Encerrar já é a confirmação.
  - **Painéis e opções:** com um painel aberto, ← → levam o painel ao ícone vizinho (Desempenho, Áudio, Controle, Tela); ícones sem painel só recebem o foco, e Encerrar nunca abre deslizando. Escolher uma opção aplica na hora, confirma com um aviso e fecha o painel, com o foco de volta no ícone. No volume, ← → ajustam.
  - **Navegação:** ← → ficam na fileira atual; ↑ de um cartão vai ao ícone correspondente e ↓ volta ao cartão. B ou Guia voltam ao jogo. "Início" volta ao launcher sem fechar o jogo (o status mostra o jogo em execução e o Guia retorna a ele).
- **HUD de desempenho:** canto superior esquerdo do jogo (Desligado, Só FPS, Completo), números tabulares, fundo preto 60%.
- **Restrição técnica do menu rápido:** sem injeção gráfica (proibida pelo PRD, §10), a sobreposição é uma janela topmost transparente; ela funciona sobre jogos em tela cheia sem bordas, mas não sobre tela cheia exclusiva. Nesse caso, o Guia deve trazer o launcher para frente (como alternar janela). Validar no Windows antes de prometer o comportamento.
- **Sair para o desktop:** sempre presente no fim da barra lateral (princípio "recovery-first" do PRD).

## Do's and Don'ts

- Do manter exatamente um elemento em foco e restaurar o foco anterior ao voltar (B).
- Do lembrar a posição em cada fileira ao sair e voltar para ela.
- Do usar transform (escala/translação), opacidade e animação de cor: são baratos no WPF.
- Don't usar blur de fundo (`backdrop-filter`) ou vidro fosco: o WPF não tem equivalente nativo e o custo em GPU é alto.
- Don't usar sombra em listas inteiras; só no item em foco.
- Don't depender de hover ou de mouse para qualquer ação.
- Do aceitar mouse em tudo como alternativa: passar o mouse foca sem rolar a tela, clicar ativa, a roda navega, as dicas do rodapé são clicáveis, clicar fora fecha o painel (ou o menu) e a barra de volume aceita clique e arraste.
- Don't versionar arte de jogos no repositório (é protegida por direitos autorais): o app baixa capas e heros das lojas para um cache local, e o protótipo as carrega direto dos CDNs.
- Do ter fallback quando não há arte: gradiente do accent com o título do jogo.
