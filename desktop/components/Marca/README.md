# Marca

O logo do TAKO: o polvo, a palavra TAKO em caixa alta fina e espaçada e タコ embaixo, entre traços.

## Peças
- **Polvo** (`tako-mark.png`): grafite claro com contorno lilás. Na barra de cima mede 50×40, com `drop-shadow(0 0 8px)` lilás a 18% (`brand-glow`).
- **Palavra** (`tako-wordmark.png`): TAKO em traço fino, 68×14 na barra.
- **Kana** (`tako-kana.png`): — タコ —, 46×8, 85% de opacidade, 6 abaixo da palavra.
- **Horizontal** (`tako-logo-horizontal.png`): as três juntas, para capa, carregamento e documentos.

## Regras
- Só sobre fundo escuro (`canvas`, `titlebar`). Não recolorir, não contornar, não pôr sobre a arte de um jogo.
- Na barra de cima a marca é um botão (abre o MenuDoTako), com ⌄ ao lado.
- O polvo também aparece nos vazios (ListaVazia, estados sem resultado) e no aviso de jogo abrindo, nunca como enfeite em botão.
- Minimizada, a janela vira um dock com o polvo de 22 e "Restaurar TAKO".

Base: protótipo do mantenedor (`launcher.html`, out/2026), tema TAKO. Prévia gerada por `gen/rebrand.py` a partir do protótipo rodando, com arte de exemplo.
