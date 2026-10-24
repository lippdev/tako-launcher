# BarraDaJanela

Os botões da janela na ponta da BarraSuperior: Minimizar · Maximizar · Fechar, 40×36, raio 8, ícones de 15 em `#9a9a9a`, depois de uma linha vertical de 1 (8 de margem). Hover: fundo `#1a1a1a`; Fechar: `window-close-hover` com ícone branco.

- Janela: 1480×860, raio 16, sombra `shadow-window`; maximizada ocupa tudo, sem raio (450 ms).
- Minimizar/Fechar: a janela encolhe para 0,94 e desce 30 com opacidade (450 / 300 ms) e aparece o **dock** embaixo no centro (polvo + "Restaurar TAKO"), que sobe 20 em 350 ms.

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
