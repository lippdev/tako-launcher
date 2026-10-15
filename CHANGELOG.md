# Histórico de alterações

Notas em português do Brasil; a versão em inglês (Estados Unidos) fica em `CHANGELOG.en-US.md`. Antes de publicar uma versão, adicione uma seção `## [VERSÃO]` com o changelog daquela versão **nos dois arquivos**. O workflow publica a seção correspondente à tag na release (em inglês, com link para esta) e falha se faltar alguma.

## [Unreleased]

## [0.1.0-alpha.8]
### Novidades
- Tela de jogo, como no Console Mode: em Configurações escolha a tela onde você joga (a TV, por exemplo). Ao abrir um jogo, o Tako liga essa tela, desliga ou cobre de preto as outras, aplica resolução, taxa e HDR se você quiser e devolve tudo ao fechar. Na primeira vez numa tela nova ele pergunta "Você consegue ver esta tela?" e volta sozinho se ninguém responder. Desligada por padrão. (#30)
- Áudio e TV: "Saída de áudio durante o jogo" troca o som para a TV ao jogar e volta depois. Google TV e Android TV podem ser ligadas pela rede e trocar para a entrada do PC no início do jogo, com espera opcional ao sair. (#31)
- Jogos da Steam acompanhados com mais precisão (pelo jogo que a própria Steam diz estar rodando) e a opção de mandar a Steam para a bandeja ao sair do jogo. (#31)
- Central de Controle sobre o jogo bem mais completa: janelas abertas para alternar ou fechar, volume e saída de áudio, uso real de CPU, GPU e RAM, contador e limite de FPS pelo RTSS, esconder o ponteiro ou movê-lo com o analógico e abrir o ControlFS. (#33, #31)
- Busca, filtros e ordenação na biblioteca: teclado na tela para o controle, busca sem acento, filtros (instalados, favoritos, recentes, fonte, gênero) e ordenação por nome, último jogado, tempo de jogo, nota ou data de adição. Funciona também nos temas do Playnite. (#32)
- Bibliotecas grandes sem travar: com 1000 jogos a biblioteca abre em cerca de 0,1 s (antes 1,8 s) e o Tako usa 170 MB parado (antes 507 MB); com 5000 jogos, 177 MB (antes mais de 2 GB). As capas carregam só quando aparecem na tela. (#29)

### Mudanças
- O Tako agora é software livre sob a licença AGPL-3.0, a mesma do Console Mode, e usa o código de monitores, áudio, Steam, TV e menu de sessão dele. (#27)
- A versão portátil ficou com menos da metade do tamanho: 69 MB (antes 141 MB). (#28)

### Para desenvolvedores
- [ADR-0014](docs/adr/0014-licenca-agpl.md) (licença), [ADR-0015](docs/adr/0015-monitores-do-console-mode-e-tela-de-jogo.md) (tela de jogo) e [ADR-0016](docs/adr/0016-audio-steam-e-tv-na-sessao.md) (áudio, Steam e TV). O checklist de hardware ganhou as seções da tela de jogo, da Central de Controle e de áudio/Steam/TV.
- Cenário de 5000 jogos e tempo de "abrir a biblioteca" no `tools/Launcher.Bench`. (#29)

## [0.1.0-alpha.7]
### Novidades
- Monitores e áudio voltam ao normal depois do jogo: o Tako registra resolução, taxa de atualização, monitor principal, saída de som, volume e mudo antes de abrir o jogo e, ao terminar, restaura só o que o jogo mudou. Se o Tako cair durante o jogo, ele oferece restaurar na próxima abertura. Dá para desligar em Configurações. (#22)
- Temas do Playnite na interface inteira: em Configurações → Temas, "Usar este tema" troca a tela inicial do Tako pela do tema, com sua biblioteca, Jogar, detalhes e voltar funcionando por controle e teclado. O botão Menu sempre leva às Configurações do Tako; qualquer falha volta para a interface padrão. (#24)
- O Tako se protege sozinho: um supervisor leve reabre a interface se ela travar ou fechar inesperadamente, depois tenta o modo seguro e, em último caso, deixa você no desktop. Os jogos não são fechados. (#23)
- Configurações → Diagnóstico do sistema gera um relatório (sem dados pessoais) com versão, Windows, monitores, controles, Steam e Xbox mode, para anexar quando algo der errado. (#21)

### Correções
- Segurança: temas não conseguem mais carregar imagens ou vídeos de fora da própria pasta, da internet ou da rede, mesmo montando o caminho durante a execução. (#25)

### Para desenvolvedores
- IPC por named pipes com protocolo versionado entre supervisor e interface ([ADR-0012](docs/adr/0012-supervisor-e-ipc.md)); contratos próprios de monitores e áudio ([ADR-0013](docs/adr/0013-estado-de-monitores-e-audio.md)). (#22, #23)
- `tools/Launcher.Bench` e job `benchmark` no CI com a linha de base em `docs/marco-1/benchmark.md`. (#21)
- Plano e status do Marco 1 em `docs/marco-1/plano.md`.

## [0.1.0-alpha.6]
### Novidades
- Jogos acompanhados de verdade: ao jogar, o Tako percebe quando o jogo termina (pelo processo ou, nos jogos da Steam, pela pasta do jogo), registra o tempo jogado real, volta para a frente e devolve o foco ao jogo, com "Bem-vindo de volta". Em jogos adicionados, Opções → Editar muda nome, argumentos, pasta de trabalho e como reconhecer o fim do jogo. (#18)
- Temas do Playnite: importe pacotes `.pthm` (com checagem de segurança antes de gravar qualquer arquivo) e gerencie tudo em Configurações → Temas: prévia, diagnóstico de compatibilidade, usar, remover e restaurar o padrão. Temas parciais herdam o tema padrão. Três temas reais foram homologados (PS5ish, Toggle e Revisited). (#19)
- Modo seguro: se o Tako fechar errado duas vezes seguidas, ele abre só com o tema embutido e sem atualizações, com a opção de voltar ao normal. Um erro inesperado mostra a tela "Algo deu errado" com saída para o desktop em um passo. (#17)
- "Abrir em tela cheia ao iniciar o Windows" agora funciona nas versões instalada e portátil. Configurações mostra se o PC tem o Xbox mode. (#17)
- Suas configurações e a biblioteca ficam num banco de dados (SQLite) e sobrevivem a reinícios; o arquivo antigo é importado sozinho. Logs de diagnóstico, sem dados pessoais, em Configurações → Abrir pasta de logs. (#16)
- Atualização automática: ao abrir, o Tako procura uma versão nova e a baixa em segundo plano, conferindo o arquivo com o SHA-256 publicado no GitHub. Quando termina, pergunta se reinicia agora ou instala ao sair; durante um jogo, o aviso espera o jogo acabar. Funciona na versão instalada (roda o instalador em silêncio) e na portátil (troca o .exe). Em Configurações: verificar agora, desligar a atualização automática e receber versões de teste. (#15)

### Correções
- B na tela "Iniciando" agora cancela de verdade (antes o jogo abria mesmo assim). (#17)
- Controle desconectado, trocado ou com a janela fora de foco não deixa mais o foco "andando sozinho". (#17)

### Atualizando da 0.1.0-alpha.5
- Esta é a última atualização manual: baixe o instalador ou o portátil da release. A partir daqui o Tako se atualiza sozinho.

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
