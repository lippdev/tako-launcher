# Histórico de alterações

Notas em português do Brasil; a versão em inglês (Estados Unidos) fica em `CHANGELOG.en-US.md`. Antes de publicar uma versão, adicione uma seção `## [VERSÃO]` com o changelog daquela versão **nos dois arquivos**. O workflow publica a seção correspondente à tag na release (em inglês, com link para esta) e falha se faltar alguma.

## [Unreleased]

## [0.1.0-alpha.5]
### Novidades
- Sua biblioteca de verdade: os jogos instalados pela Steam aparecem sozinhos (todas as pastas de biblioteca, com tempo jogado e última vez, sem precisar da Steam aberta), e qualquer outro jogo entra por "Adicionar jogo" (executável, atalho ou link). Jogar abre o jogo de verdade. Favoritos e histórico ficam salvos. (#13)
- Dados e imagens automáticos, como no Plex: descrição em português, gêneros, lançamento, desenvolvedora, nota do Metacritic e capas e fundos em alta resolução, pela loja da Steam. Jogos adicionados são encontrados pelo nome; se vier o jogo errado, Detalhes → Opções → Corrigir correspondência mostra os resultados para escolher. (#13)
- Opções do jogo: abrir na Steam, trocar a capa por uma imagem sua, abrir a pasta e remover da biblioteca (os arquivos do jogo não são apagados). (#13)
- Controles PlayStation (DualSense, DualSense Edge e DualShock 4), por USB ou Bluetooth, sem Steam nem DS4Windows: as dicas mostram ✕ ○ □ △ e o botão PS abre a Central de Controle. (#12)
- Telas 2K e 4K nítidas, e o layout ocupa a tela toda em qualquer formato (ultrawide 21:9, 16:10). (#12)
- Instalador e versão portátil, no mesmo formato do Console Mode. (#14)
- Prévia de temas Fullscreen do Playnite 10 em Configurações: 31 dos 33 arquivos do tema padrão carregam, pelo carregador seguro de temas. (#7, #8)

### Mudanças
- Nome novo: **Tako**, com o polvo no ícone e na barra lateral. (#11)
- A Central de Controle abre com Shift+Tab no teclado (como a sobreposição da Steam), no lugar de G. (#12)
- Títulos com fonte leve no estilo do PS5, ocupando a largura da tela; barra lateral aberta com o logo do Tako. (#12)
- Os jogos de exemplo saíram do app. (#13)

### Para desenvolvedores
- Interface WPF do Marco 0 a partir do protótipo aprovado, com teste de fumaça que percorre todas as telas e publica capturas no CI. (#6, #7)
- Carregador XAML restrito para temas e adaptador de temas do Playnite 10. (#3, #7, #8)
- Spike do Xbox mode / FSE com a ferramenta FseProbe; o kit acompanha as releases enquanto o spike estiver aberto. (#4)
- Releases pelo `build/Publish-Tako.ps1` (portátil num único .exe e instalador Inno Setup); toda PR gera os dois no CI. (#10, #14)

## Versões anteriores
As pré-versões 0.1.0-alpha.1 a 0.1.0-alpha.4 foram builds de teste internas, sem changelog próprio; o que elas trouxeram está resumido em 0.1.0-alpha.5.
