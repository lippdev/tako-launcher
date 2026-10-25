# FundoDoJogo

A arte de fundo do jogo (`library_hero`, 3840×1240).

- Duas camadas trocam de lugar na troca de jogo: a nova entra em 800 ms (opacidade) e assenta de 1,05 para 1 em 1,8 s.
- Tema TAKO: some na esquerda (máscara de 220; 90 na loja) e embaixo no canvas; degradê da direita a 60%.
- Sem arte: degradê da cor do jogo (EstadoSemArte). Arte clara: os mesmos degradês seguram o texto (EstadoArteClara).

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
