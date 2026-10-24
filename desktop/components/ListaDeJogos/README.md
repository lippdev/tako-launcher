# ListaDeJogos

A lista fixa da biblioteca (292), só na aba BIBLIOTECA.

## Topo
Abas Todos · Favoritos · Instalados; busca com Ordenar, ver e filtrar; botão de recolher (Ctrl B); **Visão geral** (ícone de casa) e uma linha.

## Linha (42)
**Ícone do jogo** 28 (raio 7), como na Steam: o ícone da loja (`clienticon`); sem ele, o ícone do executável e, por último, a inicial. A capa nunca entra na linha: fica só na Grade e nas fileiras. Recolhida, o mesmo ícone em 46.

Na linha: nome 14 e, à direita, ponto azul de atualização ou a % do download. Hover branco 5%; escolhida: no tema TAKO, degradê da cor de destaque (17% → 5%), nos outros branco 9% com contorno 8%. Não instalado: nome `#858585` e ícone apagado. Dica com nome e situação ("Jogado ontem, 12:38").

## Modos
- **Recolhida** (76, Ctrl B): só ícones de 46 (raio 11), em cinza menos o escolhido e o do hover; o escolhido ganha anel; a busca vira lupa (abre a busca geral).
- **Grade**: 3 capas por linha (8 de espaço), nome no hover, escolhida com anel branco de 2.
- O menu Ordenar, ver e filtrar: ordenar (Jogado por último, A–Z, Tempo de jogo), ver (Lista, Grade), mostrar só (Com atualização, Jogos instalados), Adicionar um jogo…, Atualizar biblioteca.

Ao filtrar, as linhas entram em escada (−6 px, 350 ms, 15 ms cada); um jogo novo pisca a cor de destaque (1,4 s).

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
