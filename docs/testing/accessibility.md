# Acessibilidade — UX-001

Configurações › Aparência oferece texto em 100%, 110% e 125% e volume próprio para sons de navegação (0–100%).
0% é o padrão e desliga o som. A escala usa a tipografia dos tokens do tema padrão, inclusive no tema de alto
contraste que o importa; não altera paleta ou layout. Temas independentes precisam expor `Ui.text-scale`.
"Reduzir animações" (mesma tela) zera todas as animações do tema pelos tokens `Ui.*-time`, inclusive o fade de
"Abrindo…" para o jogo, e o app deixa de esperar por ele; um teste reprova `animate` com duração fixa. O alto
contraste é o tema "Tako Alto contraste", que acompanha o Tako e é escolhido em Aparência › Tema.

Os tons são sintetizados localmente (24 ms, envelope de entrada/saída), sem arquivos de terceiros, rede ou
reprodução contínua. Direções usam o mesmo tom; confirmar e voltar têm tons distintos. A fila é substituída em
cada comando, evitando acúmulo. Um erro de dispositivo é registrado e a navegação continua sem som; desligar
e ligar o volume permite tentar novamente.

Nenhum som é criado em simulação, durante um jogo, na transição de abertura ou ao sair da interface. A saída
SDL é liberada nessas condições. O supervisor não toca sons. O volume regula somente os tons do Tako.

## Teste manual pendente

- Nos dois idiomas e em 720p/1080p/4K, verificar os três tamanhos em Início, Biblioteca, Detalhes, Configurações,
  painéis e teclado; conferir recortes e textos longos, com o tema padrão e o de alto contraste.
- Ajustar por controle e teclado, reiniciar e conferir persistência; voltar a 100% restaura a tipografia original.
- Com "Reduzir animações" ligado: foco, troca de página, cor de fundo e o "Abrindo…" ao entrar num jogo mudam sem
  transição, e o jogo aparece logo que a janela dele abre.
- Ligar o som, variar o volume, navegar rapidamente, confirmar/voltar e clicar: sem fila de tons atrasados.
- Volume zero: silêncio. Abrir jogo, Central durante o jogo e retorno: nenhum tom durante a sessão.
- Sem saída de áudio ou após desconectar o dispositivo: navegação funciona; desligar/ligar o som permite recuperar.
- Conferir volume percebido em TV/fones e ausência de estalos; a síntese foi verificada numericamente, não ouvida.
