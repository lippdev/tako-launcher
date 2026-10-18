# Telas principais — PRD §8.4

`crates/tako-ui/src/ui_tests.rs` cobre a jornada por comandos do controle no estado da interface:
Início → detalhes → voltar, grade → menu → voltar, grade → detalhes → voltar, busca → cancelar e entrada/saída
das categorias de Configurações. O teste exige que o jogo e o índice de foco sejam preservados e não executa jogos.

Outro teste cria o tema real no backend de software do Slint, aplica todas as propriedades produzidas pelo shell
e renderiza Início, Biblioteca, Detalhes e Configurações, em pt-BR e en-US, a 720p, 1080p e 4K. Reprova erro no
contrato Rust/Slint e quadros inteiramente vazios. Não abre janela nativa nem depende de servidor gráfico.
A feature `renderer-software` do wrapper Slint é habilitada apenas como dependência de desenvolvimento.

O contraste WCAG AA (4,5:1) é conferido para os dois tokens de texto sobre o painel branco do tema padrão.
Isso não certifica contraste sobre capas, fundos variáveis, transparências ou temas de terceiros.

Na bancada compartilhada: `scripts/bancada.sh run B cargo test -p tako-ui ui_tests --locked`.
O CI completo continua obrigatório: `scripts/bancada.sh ci B`.

## Teste manual pendente

- Controle real: navegar por todas as telas, abrir/cancelar camadas e confirmar o foco ao reconectar o controle.
- Teclado, mouse e troca para o controle: confirmar indicação de foco e rolagem sem saltos inesperados.
- Nos três tamanhos e nos dois idiomas: conferir recorte, sobreposição, dicas, legibilidade e textos longos.
- Confirmar foco com artes claras/escuras e tema de alto contraste, além de reduzir animações.
- Biblioteca vazia, 1 jogo e biblioteca grande: saída para Configurações/Desktop sempre alcançável.
- Prévia de tema (THEME-002), com controle real na tela cheia: Aparência › Tema, escolher um tema instalado, ver o
  tema inteiro com as dicas "Aplicar tema" e "Voltar ao anterior (N s)"; A salva, B volta, e sem resposta volta em 15 s
  (a contagem anda a cada segundo). Reabrir o Tako durante a prévia deve abrir o tema de antes. Um tema com `.slint`
  quebrado avisa e mantém o anterior.
- Repetir com backend GPU nas três plataformas; software headless não mede fluidez nem certifica a GPU.

O teste automatizado é uma regressão de navegação, vínculo e renderização; comparação artística de screenshots
e medição visual de contraste sobre arte continuam sendo validação humana.
