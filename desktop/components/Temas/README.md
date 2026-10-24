# Temas

Cinco temas em Configurações › Tema; o **TAKO** é o principal e o padrão.

| Tema | Fundo | Destaque | Notas |
|---|---|---|---|
| **TAKO** | `#0b0b0e` (preto tinta, superfícies levemente azuladas) | cor da arte do jogo na tela (ou pedra `#d8c8bc`) | barra de cima em vidro fosco por cima do conteúdo; abas em caixa alta fina com sublinhado em tentáculo que brilha; lista sem divisória; arte some na lista e no canvas |
| Escuro | `#0a0a0a` | escolhida (padrão azul `#4d9bff`) | monocromático, como desenhado |
| Meia-noite | `#0b0e15` | escolhida | igual ao Escuro com fundo azulado |
| Preto OLED | `#000000` | escolhida | igual ao Escuro com preto puro |
| Liquid Glass | `#07080c` + arte desfocada como luz | escolhida | superfícies translúcidas (7%), desfoque 28 (12–60), reflexo no topo; opção de controles em pílula; sem arte, aurora na cor de destaque |

## Cores da arte do jogo (TAKO)
- Liga em "Cores da arte do jogo" (padrão: ligado). O TAKO amostra a arte de fundo (64×36), pega o matiz mais forte (bins de 20°, ignorando cinzas, pretos e brancos), e monta o destaque com saturação 62–92% e luz 50–58%.
- Só o destaque e dois brilhos de canto (`ambient-1` 7%, `ambient-2` 4%) mudam; superfícies, linhas e pretos ficam os do logo. A barra de cima sempre mantém as cores do logo.
- Tudo desliza para a paleta nova em 1 s (`tempo-cor`). Destaque claro (luminância > 0,6): o texto em cima dele vira escuro (`#14110f`).
- Desligado: destaque pedra `#d8c8bc`.

## Cor de destaque
Pedra TAKO, Laranja, Ouro, Verde, Ciano, Azul, Violeta, Rosa, Branco ou uma cor própria. O destaque é só para **estado**: progresso, atualização, chave ligada, ponto de novidade, item escolhido.

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
