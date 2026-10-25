Modo desktop do TAKO: a janela para mouse e teclado onde se joga, compra, organiza a biblioteca e configura (ADR-0027). É o modo principal e convive com a tela cheia, que tem o próprio design system (**Tako Design System — Tela cheia**). Os dois usam o mesmo `Shell`, biblioteca, configurações e ações.

**Rebranding (out/2026).** Este sistema foi refeito a partir do protótipo do mantenedor (`launcher.html`): logo novo, paleta, tipografia, superfícies, botões, telas, temas e movimento. O **tema TAKO** é o principal. Tudo o que vinha antes (Liquid Glass flutuando sobre a arte, painel de 344, cápsulas de vidro) foi substituído; o Liquid Glass continua como um dos temas.

## A referência: TelaBiblioteca

A Visão geral da biblioteca é o padrão de todo o desktop. Toda tela nova usa as mesmas peças:

- **Estrutura:** BarraSuperior de 52 de borda a borda (marca · abas LOJA BIBLIOTECA · downloads · perfil · tela cheia · janela) e a ListaDeJogos fixa de 292 à esquerda (só na biblioteca; some na loja e nas configurações). Conteúdo com margem de 40, 48 entre seções, 16 entre colunas.
- **Abaixo de todo destaque:** conteúdo à esquerda, dados à direita (fixos ao rolar).
- **Destaque:** dados embaixo à esquerda; identidade (situação, logo do jogo, descrição) e ações embaixo à direita.

## Princípios (do protótipo)

1. **Uma linguagem de superfície:** o grupo de blocos — grupo com contorno, calha de 6, blocos com o valor e uma faixa de rótulo em caixa alta (GrupoDeBlocos).
2. **Um primário por região:** botão branco com chip escuro do ícone. Secundário: fantasma com chip. Terciário: botão quadrado só de ícone (BotaoDesktop).
3. **Composição do destaque:** dados embaixo à esquerda · identidade e ações embaixo à direita.
4. **Conteúdo à esquerda, dados à direita** (fixos) abaixo de todo destaque.
5. **UI monocromática;** a cor de destaque só para **estado** (progresso, atualização, chave, item escolhido); verde só para "ligado" (instalado, online, continuar jogando).
6. **Estrutura do TAKO:** abas na barra de cima, lista fixa da biblioteca, perfil no canto superior direito.
7. **Uma página do jogo para loja e biblioteca:** só muda o conjunto de ações.

## Marca

O polvo + TAKO em caixa alta fina + タコ (ver **Marca**). Na barra de cima: polvo 50×40 com brilho lilás, palavra 68×14, kana 46×8. Só sobre fundo escuro; nunca sobre a arte de um jogo. O polvo aparece também nos vazios e nos avisos do próprio TAKO ("Tentáculos no controle · abrindo …").

## Temas

TAKO (principal), Escuro, Meia-noite, Preto OLED e Liquid Glass (ver **Temas**). No TAKO, com "Cores da arte do jogo" ligado, a cor de destaque vem da arte do jogo na tela e desliza em 1 s; superfícies e pretos ficam os do logo; a barra de cima mantém sempre as cores do logo. Sem arte ou com a opção desligada, o destaque é a pedra `#d8c8bc`.

## Cor

- `canvas` #0b0b0e · `titlebar` #08080b · `surface` #0d0d10 · `surface-raised` #121317 · `tile` #141519 · `tile-foot` #101014.
- Linhas: `line` 7,5% e `line-strong` 13%, levemente lilás no TAKO.
- Texto: `text` #f4f1f0 · `text-muted` #958f94 · `text-dim` #625d63.
- Estado: `accent` (da arte / escolhido), `success` #3ddc84, `warning` #ffcf4d (favorito), `error` #ff7b7b, `wish` #ff5d7a; Metacritic verde/amarelo/vermelho.
- Primário `#f4f4f4` com chip `#0d0d0d` e texto `#111216`.
- Os nomes antigos (`glass-*`, `on-accent*`, `settings-*`) continuam como apelidos com os valores novos.

## Tipografia

- **Poppins** (display): títulos, números, preços, ações primárias, abas da barra (12/400, caixa alta, 0,32 em).
- **Plus Jakarta Sans** (base): todo o resto.
- Escala: título de página 26/800 · configurações 28/700 · seção 19/700 · valor de bloco 22/700 · linha 14 · texto 13,5 · secundário 12,5 · rótulo em caixa alta 12/700. Nada abaixo de 12.

## Superfícies e forma

Raios: ícone 7 · controle 9 · linha 10 · botão 11 · grupo/caixa 16 · destaque/diálogo 18 · pílula 999. Contornos de 1 px em vez de sombra; sombra só no que flutua (menus `shadow-float`, diálogos `shadow-layer`, janela `shadow-window`). Item escolhido: branco 9% com contorno 8% (no TAKO, degradê da cor de destaque).

## Ícones

Traço de 1,7, pontas e junções redondas, 18 px (15 em menus e controles pequenos), na cor do texto; `fill` só em ▶, estrela e coração ligados. Conjunto do protótipo (casa, busca, relógio, calendário, controle, download, atualizar, estrela, coração, engrenagem…), em `assets/Icones`.

## Movimento

Uma curva para quase tudo, `cubic-bezier(0.22, 1, 0.36, 1)`; crossfades de arte em `ease`. Hover 200 · menus 150/200 · vistas 450/550 (escala 0,985) · troca de jogo em escada de 55 ms (500/650, sobe 14, desfoque 4 → 0) · lista 500 · arte 800 + 1,8 s · destaque da loja 900 + 7 s por item · cores da arte 1 s. Ver **MovimentoBase** e os laços do grupo Movimento. Reduzir animações zera durações e atrasos.

## Acessibilidade

- Texto mínimo 12; alvo mínimo 24×24; todo botão só de ícone com nome (`aria-label`) e dica.
- Contraste ≥ 4,5:1: rótulos sobre a arte têm sombra de texto e os degradês do destaque seguram arte clara (EstadoArteClara).
- **Reduzir animações** (Aparência) e **Efeitos de vidro** (desligar o desfoque) e **Tamanho da interface** 90/100/110/125%.
- A cor nunca sozinha: situação, confiança, bateria e Metacritic sempre com texto ou número.

## Entradas

- **Mouse:** clique abre, duplo clique joga, botão direito abre o menu.
- **Teclado:** Ctrl + K busca geral, Ctrl + B recolhe a lista, Esc fecha a camada de cima, F11 vai para a tela cheia.
- **Controle:** navega pela interface (opção) e o TAKO oferece a tela cheia ao detectar um controle.

## Regras do produto

- **Escolhas são select** (CampoDeSelecao); valor contínuo é deslizante (ControleDeslizante); chave só liga e desliga. As setas ‹ › só rolam fileiras de capas (como no protótipo), nunca trocam um valor.
- **Sem métricas inventadas:** nota, tempo para zerar e conquistas só com a fonte respondendo; sem dado, o bloco some.
- **Desempenho:** uma textura por imagem, arte no tamanho da janela, listas em vagas fixas, nada redesenhado com a janela minimizada ou um jogo aberto. O desfoque (barra de cima, Liquid Glass) é do conteúdo que rola por baixo: no Slint, use a cópia desfocada no cache ou desligue com Efeitos de vidro.

## Componentes

1. **Marca:** Marca, Temas.
2. **Navegação:** BarraSuperior, BarraDaJanela, MenuDoTako, AtividadeNaBarra, CampoDeBusca, Abas.
3. **Ações:** BotaoDesktop, JogarEAbrir.
4. **Biblioteca:** ListaDeJogos, BibliotecaGrade, NaoInstalados, ListaVazia, StatusDoJogo, SeletorDeColecao, SelecaoMultipla, BibliotecaCategorias.
5. **Jogo:** Destaque, TituloDoJogo, FundoDoJogo, CartaoSessoes (barra de dados), GrupoDeBlocos, AcoesDoJogo, PilulaDaLoja (chips), CartaoTrailer, CartaoDetalhes, Conquistas, CartaoSobre, CapaDoJogo, EtiquetasDoJogo.
6. **Camadas:** MenuDeContexto, PainelDownloads, Aviso, PainelLateral, Dialogo, PadraoPopup, ErroDoJogo, CampoDeTexto, SeletorDeImagens, ConfiguracoesDoJogo, AssistenteInicial, AtualizacaoDoApp.
7. **Loja:** PopupOferta, PopupCupom.
8. **Configurações:** CategoriaDeConfiguracao, LinhaDeConfiguracao, ControleDeslizante, CampoDeSelecao.
9. **Estados:** EstadoCarregando, EstadoNaoInstalado, EstadoJogando, EstadoSemArte, EstadoArteClara, EstadoSemTransparencia, JanelaPequena, ListaTexto125.
10. **Telas:** TelaBiblioteca, TelaJogo, TelaJogoRolada, TelaJogoNaLoja, TelaLoja, TelaLojaExplorar, TelaListaDeDesejos, TelaPerfil (+ PerfilSessoes, PerfilColecoes, PerfilFamilia), TelaConfiguracoes, TelaTema, TelaControles, FilaDeCapas.
11. **Onboarding:** TelaCarregamento, TelaOnboarding → OnboardingEmuladores → OnboardingCapas → OnboardingModo → OnboardingPronto, com PassosDoOnboarding, LojaEncontrada e EscolhaDoOnboarding.
12. **Movimento:** MovimentoBase, EntradaDoApp, TrocaDeJogo, TrocaDeTela, TrocaDeModo, TrocaDePopup, PainelEAviso, MenusEDicas, PassosDoOnboardingMovimento.

Saíram no rebranding: **DestaqueFases** (a página do jogo agora rola: TelaJogo e TelaJogoRolada) e **BarraLateralUnida** (a lista já é fixa e sem divisória no tema TAKO).

## Sobre as prévias

As prévias são o protótipo rodando (as mesmas classes, com escopo `.tk` no `bundle.css`), traduzido para o português, com arte de exemplo: a arte de God of War Ragnarök e as capas da fileira vêm da captura de tela do mantenedor; as outras são a arte de Hades II e as artes de teste do sistema. As artes pertencem aos seus donos e estão aqui só para mostrar o visual sobre arte real.
