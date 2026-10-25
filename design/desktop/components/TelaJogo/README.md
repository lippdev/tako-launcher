# TelaJogo

A página do jogo, igual na biblioteca e na loja: só muda o conjunto de ações.

## Destaque (560–700)
- Arte do jogo de borda a borda, que some à esquerda (máscara de 220 no tema TAKO) e embaixo no canvas.
- Embaixo à direita: situação (INSTALADO, ATUALIZAÇÃO DISPONÍVEL…), logo do jogo (até 440×140), descrição e ações (★ favorito, Gerenciar, Jogar). Embaixo à esquerda: barra de dados.
- Canto superior direito: chips (situação do jogo, Windows, Compartilhar).
- Barra fina no topo (62): ← Voltar, trilha "Biblioteca › jogo"; ao rolar ganha fundo e mostra capa + Jogar.

## Corpo
Conteúdo à esquerda (trailer, capturas, texto, recursos, requisitos) e dados à direita (Detalhes, radar de gênero e, fixos ao rolar, números e conquistas). Ver TelaJogoRolada.

Trocar de jogo não recarrega a página: ver TrocaDeJogo.

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
