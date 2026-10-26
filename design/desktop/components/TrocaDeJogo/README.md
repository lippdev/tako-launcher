# TrocaDeJogo

Trocar de jogo na página do jogo não recarrega nada: as partes (situação, logo, descrição, preço, ações, dados, chips) saem em 200 ms (opacidade, desce 14, desfoque 4), a página volta ao topo, a arte troca em crossfade e as partes voltam **em escada de 55 ms** (`--d` 0 a 5), em 500/650 ms. O corpo faz só opacidade (150 / 400). Com Reduzir animações, troca na hora.

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
