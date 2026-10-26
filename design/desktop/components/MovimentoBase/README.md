# MovimentoBase

O movimento do protótipo: **uma curva** — `cubic-bezier(0.22, 1, 0.36, 1)` — rápida no começo e longa no fim, para vistas, lista, abas, menus, painéis e botões; crossfades de arte em `ease`.

| O quê | Tempo |
|---|---|
| Hover (cor, fundo) | 200 ms |
| Menus e dicas | 150 (opacidade) / 200 (posição) |
| Chave, check, estados | 250–300 |
| Troca de vista | 450 opacidade + 550 escala 0,985 → 1 |
| Troca de jogo | partes em escada de 55 ms: 500 opacidade e desfoque 4 → 0, 650 sobe 14 |
| Lista abrir/recolher | 500 (largura) |
| Arte do jogo | 800 crossfade + 1,8 s escala 1,05 → 1 |
| Destaque da loja | 900 crossfade, zoom lento 7,5 s, 7 s por item |
| Cores da arte | 1 s |

Reduzir animações: todas as durações e atrasos viram 0.

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
