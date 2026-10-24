# BarraSuperior

A barra de cima, de borda a borda, 52 de altura.

## Da esquerda para a direita
- **Marca** (botão, abre o MenuDoTako) com ⌄.
- **Abas** LOJA · BIBLIOTECA a 40 da marca, 22 entre elas: Poppins 12/400 em caixa alta com 0,32 em de espaçamento; a atual em branco 500 com brilho de texto e o **sublinhado em tentáculo** (3 de altura, degradê da cor de destaque, brilho de 12). Hover: o sublinhado aparece a 35%. Nas Configurações nenhuma aba fica marcada.
- À direita, botões fantasma de 36 (sem fundo nem borda; hover com brilho da cor de destaque embaixo): Downloads (AtividadeNaBarra), **perfil** (avatar redondo de 24 com anel da cor de destaque e ponto verde online, nome, ⌄) e Tela cheia (controle).
- Linha vertical e os botões da janela (BarraDaJanela).

## Material (tema TAKO)
Vidro fosco por cima do conteúdo: degradê da tinta violeta `titlebar-tint` (75%) para o canvas (52%), desfoque 22 e saturação 1,5; linha de baixo branca 6%. O conteúdo rola por baixo. A barra mantém as cores do logo mesmo com as cores da arte ligadas. Nos outros temas: `#070707` opaco com linha.

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
