# Histórico de alterações

Notas em português do Brasil; a versão em inglês (Estados Unidos) fica em `CHANGELOG.en-US.md`. Antes de publicar uma versão, adicione uma seção `## [VERSÃO]` com o changelog daquela versão **nos dois arquivos**. O workflow publica a seção correspondente à tag na release (em inglês, com link para esta) e falha se faltar alguma.

## [Unreleased]

### Novidades
- **Lista de jogos fixa (TAKO):** a lista da biblioteca agora é uma coluna de 292 colada à borda esquerda, de baixo da barra até o pé da janela, sem divisória no tema TAKO. No topo, as abas Todos · Favoritos · Instalados e a busca com o botão Ordenar, ver e filtrar, que reúne ordem, modo de exibição, filtros, coleções, Adicionar jogo e Atualizar biblioteca. Coleções saem das abas e entram nesse menu (a coleção ativa vira um chip no lugar de Visão geral). A barra lateral unida e a largura da barra lateral saem das Configurações.
- **Marca nova no desktop:** o polvo, a palavra TAKO e タコ na barra de cima, o polvo como ícone do app (janela do desktop, instalador e pacotes, sobre uma placa escura) e ícones novos de traço fino na interface do desktop. A tela cheia não muda.
- **Cores da arte do jogo (TAKO):** no tema TAKO, o destaque e os brilhos de canto da janela pegam a cor mais forte da arte do jogo na tela e deslizam para a nova em 1 s. Dá para desligar em Configurações › Aparência; desligado (ou sem arte), fica a pedra do logo. A cor de destaque escolhida vale nos outros temas.
- **Temas e cor de destaque no desktop:** em Configurações › Aparência, escolha TAKO (o novo padrão), Escuro, Meia-noite, Preto OLED ou Liquid Glass (com arte como luz ambiente, opacidade e controles em pílula) e a cor de destaque, inclusive uma cor própria. As cores, os raios e os tempos de animação seguem o novo design system; a tela cheia não muda.
- **Barra de cima refeita no desktop:** 52 de altura, de borda a borda, com a aba Biblioteca em letras finas e o sublinhado que brilha, o perfil como avatar com anel na cor de destaque (o menu dele abre a página do Perfil), o botão de tela cheia e os botões da janela menores. Dá para chegar a todos eles pelo teclado.
- **Downloads na barra de cima:** o botão ↓ antes do perfil mostra com um número quantas coisas o Tako está fazendo em segundo plano (atualizar a biblioteca, procurar atualização, downloads, capas) e ganha um selo quando algo dá erro; ao concluir ou falhar, um aviso diz o resultado. Clicar abre a lista, com o andamento, "Tentar de novo" no erro e "Limpar concluídas".
- **ScreenScraper com a sua conta de desenvolvedor:** quem tem credenciais de desenvolvedor do ScreenScraper pode colocá-las nas Configurações e usar o ScreenScraper mesmo em versões que não as trazem.
- **Modo tela cheia do Windows:** use o Tako como tela inicial nas Configurações.
- **Downloads por torrent:** baixe jogos das fontes que você mesmo adiciona, num painel com pausa, retomada e fila. Vem desligado.
- **Jogos não instalados:** os jogos da sua conta nas lojas que não estão no PC aparecem depois dos instalados, agrupados por loja, na lista e na grade. Cada grupo abre e fecha, e o Tako lembra como você deixou.
- **Instalar pela loja:** o botão ⬇ na linha, na capa e no destaque abre a instalação no cliente da loja, mostrando a loja e o tamanho. Jogo comprado em mais de uma loja deixa escolher por qual instalar.
- **Progresso da instalação:** na Steam, o Tako mostra a porcentagem com um anel na lista e na grade e "Baixando… 42%" no destaque. Quando termina, o jogo vira instalado sem reiniciar. Na Epic aparece "Instalando" sem número.
- **Espaço livre:** o destaque mostra quanto sobra no disco e avisa quando falta espaço para o jogo.
- "Instalar" também no menu do jogo e na seleção de vários jogos, que abrem um por vez na loja.
- **Ícone do jogo na lista:** a lista mostra o ícone do jogo, como na Steam (da loja ou do executável no Windows, ou a inicial do título); a capa fica só na Grade.
- **Jogos da Amazon:** no Windows, os jogos da sua conta Amazon aparecem como não instalados e o Instalar abre o Amazon Games.
- **VRR por jogo:** no Linux (KDE e wlroots), a tela de jogo pode ligar o VRR só para aquele jogo e desligá-lo no fim, inclusive depois de uma queda.
- **Prévia do tema:** na tela cheia, escolher um tema mostra uma prévia de 15 s; A aplica, B volta ao anterior.
- **Entrada do app:** a tela cheia abre com uma animação curta do polvo, que qualquer tecla pula e que some com "Reduzir animações".
- **Polvo do Tako:** o polvo animado aparece nos estados vazios, na espera da biblioteca e no "Bem-vindo de volta" do desktop.
- **Ferramentas junto do jogo:** nas configurações do jogo, programas extras (overlay de FPS, mods) abrem com o jogo e podem fechar no fim, com a sua permissão.
- **Links `tako://` no macOS:** atalhos e o Stream Deck abrem jogos e telas do Tako também no Mac.
- **Pacotes Linux:** PKGBUILDs para o AUR e Flatpak no repositório (ainda não publicados).
- **Bottles e itch.io no Linux:** os jogos instalados pelo Bottles e pelo app do itch.io aparecem na biblioteca.
- **API local:** outros programas da sua máquina (Stream Deck, scripts) podem ler a biblioteca e abrir jogos por uma API em 127.0.0.1 com token. Vem desligada.
- **Editar o tema:** cor de destaque, fonte e fundo (cor ou imagem) do tema atual, com prévia ao vivo e "Restaurar padrão".
- **Criar tema:** parta do tema padrão ou de um instalado, ajuste cores, fonte e fundo, dê um nome e salve como tema novo; exporte como `.takotheme` para compartilhar.
- **Música e sons do tema:** na tela cheia, o tema pode trazer música de fundo e sons de navegação, com volume próprio em Configurações › Som. Nunca tocam durante o jogo.
- **Tema Fita:** tema oficial novo para a tela cheia, com os jogos num carrossel horizontal e a arte em tela inteira.
- **Proton e Wine no Linux:** jogos de Windows adicionados à mão podem abrir com o Wine do sistema, o Proton da Steam (inclusive GE-Proton) ou um runner do Lutris, com prefixo próprio.
- **Backup e restauração:** Configurações › Sistema gera um `.takobackup` com a biblioteca, as configurações, os temas e as capas escolhidas. Restaurar guarda antes uma cópia do estado atual; permissões e o token da API local nunca entram no backup, e comandos, argumentos e variáveis de ambiente que vierem dele pedem confirmação antes de valer.
- **Sunshine e Moonlight:** em Configurações › Integrações, exporte os jogos para o Sunshine/Apollo e abra-os pelo Moonlight. O Tako mostra a prévia, faz backup do `apps.json` e nunca mexe nos apps que você já tinha.
- **Console Mode:** Configurações › Integrações mostra se o Console Mode está instalado e qual versão da API ele oferece.

- **Atualizar biblioteca:** botão novo na barra da lista do desktop relê as lojas na hora.
### Correções
- **Modo tela cheia do Windows:** ligar o Tako como tela inicial volta a funcionar; antes o Windows recusava o pacote (0x80073CF6).
- **Ícones da Steam com falha no endereço:** quando o endereço principal do ícone de um jogo da Steam não responde na CDN, o Tako busca o ícone da comunidade pela API da loja e não mantém a falha gravada para sempre.
- **Busca de atualizações:** conexões lentas continuam enquanto recebem dados, com compressão gzip e falha clara quando a rede para.
- **Retorno ao atualizar a biblioteca:** enquanto a biblioteca atualiza, o botão gira e a barra mostra "Atualizando…"; clicar de novo avisa que já está em andamento, e no fim um aviso traz o total de jogos, os novos e a loja que falhou.
- **Fundo e vidro no desktop:** some a faixa escura vertical no meio do fundo sobre arte clara: o escurecimento lateral só desce e a elipse atrás do texto chega a zero antes da borda. As barras e os cartões de vidro desfocam a arte já escurecida e usam a tinta sólida até a cópia desfocada chegar (UI-GLASS-001).
- **Instruções em passos:** aparecem uma por linha.
- **Abertura mais rápida:** a biblioteca aparece inteira assim que o Tako abre, com a última leitura das lojas, enquanto a leitura nova roda por trás.
- **Arte do jogo escolhido:** o fundo e o logo agora trocam junto com o jogo, instalado ou não. A arte da Steam abre direto do cache da própria Steam, o jogo escolhido por último carrega primeiro e o Tako não segura mais a arte do jogo anterior enquanto a nova chega. Capas e fundos que falharem por rede lenta são tentados de novo, no desktop e na tela cheia, sem precisar reiniciar.
- **Ícones da Steam:** os ícones dos jogos da Steam voltaram a aparecer na lista e no seletor de imagens. A Steam mudou o endereço deles e o antigo deixou de responder.
- **Confirmação de argumentos e variáveis:** argumentos extras, variáveis de ambiente e Proton/Wine definidos por jogo agora entram na confirmação, junto dos comandos antes e depois; quem já usava confirma uma vez.
- **Lista lenta e imagens que não carregavam (alpha.15):** os ícones da lista têm fila própria e não seguram mais as capas e os fundos; a interface só redesenha quando uma imagem chega, e arte que a loja não tem não é pedida de novo a cada abertura.
- **Tempo de jogo:** jogo com sessão mas sem tempo registrado mostra "Não registrado" em vez de "Ainda não jogado".
- **Janela no Windows:** Minimizar, Maximizar e Fechar da barra do desktop voltam a responder; o encaixe do Maximizar continua.
- **Interface que fecha e reabre ao jogar:** logs novos para diagnosticar o problema (BUG-001).

## [0.2.0-alpha.14]
### Novidades
- **Barra de cima única:** a barra do Tako agora é a própria barra da janela, de borda a borda, com Minimizar, Maximizar e Fechar na ponta direita. No Windows, o vazio da barra arrasta a janela, o duplo clique maximiza e o Maximizar mostra os layouts de encaixe.
- **Perfil no topo:** avatar e nome numa pílula, como na Steam, com Ver perfil, Trocar perfil… e Editar perfil. A Tela cheia virou um botão só com o ícone de controle.
- **Busca no lugar do conteúdo:** no painel da lista, embaixo das abas; centralizada na barra da Grade e das Categorias; no topo do painel das Configurações. Na Loja e no Perfil, Ctrl + K abre a busca flutuante.
- **Barra lateral unida:** opção em Configurações › Aparência que cola a lista na barra de cima, num vidro só, com largura Normal ou Estreita.
- **Status do jogo:** Jogando, Quero jogar, Pausado, Zerado ou Abandonado, num seletor no canto do destaque, no menu do jogo e na seleção de vários. Jogar pela primeira vez marca Jogando, e dá para filtrar por status.
- **Filtros ativos:** aparecem como chips fixos embaixo da contagem, cada um com ✕, e "Limpar" quando há mais de um.
- **Destaque:** a fileira de cartões agora é Sobre, Detalhes e Sessões, e a nota do Metacritic entra na faixa de números quando o jogo tem. O trailer foi para a segunda fase, que não repete o que a primeira já mostra e rola mais leve.
- **Atualizações:** "Procurar atualizações…" responde no próprio menu (procurando, em dia ou erro com Tentar de novo). O diálogo novo mostra o download, as novidades da versão e "Reiniciar agora". Se a instalação falhar, o Tako volta para a versão anterior e avisa.
- **Emuladores:** todos têm o logo oficial, também na tela de emuladores das Configurações.
- Botões em coluna: a ação principal fica em cima e as duas de baixo dividem a largura igualmente.

## [0.2.0-alpha.13]
### Novidades
- **Primeira abertura nova:** tela de carregamento com o polvo se montando enquanto o Tako procura seus jogos; lojas encontradas, emuladores com os sistemas e jogos de cada um e a pasta de ROMs, capas com o antes e depois e a escolha entre buscar na IGDB ou usar só o que as lojas têm, como abrir o Tako e o resumo final com cada loja e emulador.
- **Perfil:** nova guia na barra de cima, com o tempo jogado na semana em barras, os jogos mais jogados, as sessões por dia e as coleções em cartões. O fundo é a arte do jogo mais jogado do mês.
- **Etiquetas:** o editor mostra as etiquetas do jogo como chips com ×, sugere as que já existem (com quantos jogos cada uma tem) e cria uma nova direto do campo.
- **Revisar capas:** a fila mostra a capa atual e a proposta lado a lado, a confiança em porcentagem, "Aceitar" e "Escolher outra" em cada linha, "Aceitar as N" para as de confiança alta e "Desfazer" depois de aceitar.
- **Controles:** todos os controles conectados aparecem, e os testes ficam juntos num grupo.
- **Diálogos:** as ações ficam num rodapé, com a principal à direita.
- **Abrindo o jogo:** o botão mostra um anel girando; se demorar, aparecem "Ainda abrindo…" e "Cancelar".
- **Avisos:** ficam parados enquanto o mouse está em cima.
- **Carregando:** a biblioteca mostra blocos de espera também no destaque e nos cartões.
- Textos cortados mostram o texto inteiro ao passar o mouse, e a contagem de jogos cabe na janela pequena.

## [0.2.0-alpha.12]
### Correções
- **Logo do jogo:** jogos com logo não mostram mais o nome em texto antes de trocar para o logo. Enquanto o logo carrega, o espaço do título fica reservado, e o logo aparece suavemente quando chega.

## [0.2.0-alpha.11]
### Novidades
- **Mais leve e mais fluido:** o Tako usa bem menos memória (cada imagem fica uma vez só na placa de vídeo e o fundo é carregado no tamanho da janela), a lista de jogos rola sem travar e o app acompanha monitores de 120 Hz ou mais.
- **Lista de jogos mais compacta:** linhas mais baixas, sem o ícone da loja; a loja do jogo escolhido aparece no canto superior direito. Escolher um jogo não desloca mais nada na linha.
- **Adicionar jogo:** botão "+" ao lado da contagem de jogos e "Adicionar jogo…" no menu do Tako.
- **Mais sobre o jogo:** role sobre o destaque (ou aperte Page Down) para ver uma segunda tela com a descrição inteira, todos os detalhes, as sessões e as imagens. Role para cima, Page Up ou Esc voltam.
- **Abertura em etapas:** ao abrir, a tela aparece por partes, de forma suave. "Reduzir animações" desliga.
- **Troca de jogo sem piscar:** o vidro e as cores mudam junto com a arte.
- A barra da janela não repete mais o logo, e o logo da barra superior está mais nítido e reage ao mouse.

## [0.2.0-alpha.10]
### Novidades
- **Primeira abertura:** o subtítulo aparece inteiro, o passo "Como você quer abrir o Tako?" cabe em janelas pequenas e o último passo tem "Revisar capas". Sem nenhum jogo encontrado, oferece "Adicionar jogo à mão" e "Procurar de novo".
- **Categorias:** quando nenhum jogo tem status, a tela explica como marcar e oferece "Agrupar por Loja".
- **Jogando:** o destaque mostra há quanto tempo a sessão atual está aberta.
- **Configurações deste jogo:** o que você mudou mostra o padrão do Tako embaixo, e "Usar tudo do Tako" pergunta antes de apagar as mudanças.
- **Controles:** a bateria fraca aparece em amarelo, e "Restaurar atalhos" pergunta antes.
- **Avisos:** ficam 4 segundos, ou 8 quando têm "Desfazer", e ficam legíveis sobre qualquer arte.
- **Jogo que fechou sozinho:** o aviso mostra a capa, depois de quanto tempo ele fechou e "Reparar arquivos" quando a loja permite.

## [0.2.0-alpha.9]
### Novidades
- **Logo do jogo no destaque:** o título do jogo vira o logo oficial (da Steam, ou do SteamGridDB quando falta). Sem logo, com pouco contraste sobre a arte ou se você preferir, volta o nome em texto. Escolha em Configurações › Aparência › "Título dos jogos", ou jogo a jogo na aba Logo de Trocar imagens.
- **Volume que se arrasta:** o volume dos sons de navegação virou um controle deslizante, com clique na trilha, roda do mouse, setas (Shift anda mais), Home/End, botões de mudo e de 100% e um som de amostra ao soltar.
- **Confirmação de tela:** trocar a tela de jogo ou a resolução pergunta "Manter esta configuração?" e volta sozinho em 15 s se você não confirmar.
- **Barra da janela:** clique direito abre o menu da janela, o botão de maximizar acompanha Win+↑/↓, a barra esmaece quando a janela perde o foco e aparece "Atualização pronta" quando há uma versão nova baixada.
- **Troca de jogo:** a arte nova chega deslizando de leve e o texto sobe ao entrar; com o teclado, a arte só troca quando você para num jogo.
- **Trocar imagens:** Estilo virou uma lista e as setas andam pela grade como ela aparece.
- **Coleções:** a aba vazia oferece "Nova coleção…".
- O ícone do Tako aparece na barra de tarefas e nos atalhos do Windows, os logos das lojas ficam sem contorno em todos os menus e gêneros aparecem como "Ação - Plataforma".

## [0.2.0-alpha.8]
### Novidades
- **Janela própria:** o Tako desenha a barra da janela sobre a arte do jogo, com o logo e os botões Minimizar, Maximizar e Fechar (arraste pela barra; duplo clique maximiza).
- **Escolhas viram listas:** no modo desktop, toda escolha entre várias opções (idioma, tela, resolução, áudio, perfis, filtros, ordenação) abre uma lista junto do botão, com ✓ na atual. Chaves só ligam e desligam.
- **Coleções:** a aba Coleções escolhe a coleção numa lista com quantos jogos cada uma tem, "Nova coleção…" e "Organizar coleções", e lembra a última.
- **Trocar imagens:** a escolha de capa, fundo, logo e ícone virou um popup grande com a grade na largura toda, uma prévia e os botões Usar, Voltar à original e Cancelar; clicar só escolhe.
- **Animações:** troca de página, de aba, de modo (lista, grade, categorias), de jogo, popups, painéis, menus e avisos agora animam como no design, e "Reduzir animações" continua desligando tudo.
- **Troca de jogo sem piscar:** a arte nova entra por cima da antiga, assentando devagar; a antiga fica até o fim.
- **Arte de fundo:** cobre a janela inteira em qualquer tamanho, sem esticar e sem faixas, no padrão de fundo da Steam (3840 × 1240).
- **Primeira abertura:** a tela das lojas mostra o que o Tako achou e tem "Procurar jogos" e "Escolher pastas…"; Capas e Modo ganharam Voltar.
- Cartões Sobre e Detalhes cabem em qualquer janela, títulos longos quebram em duas linhas e encolhem, a janela pequena usa linhas compactas e as Configurações ocupam a largura certa em telas grandes.
- Painéis laterais abrem abaixo da barra, a barra de rolagem da lista encosta na borda e os ícones (lojas, lista, grade) não ficam mais com contorno.

## [0.2.0-alpha.7]
### Novidades
- **Novo visual do modo desktop:** a arte do jogo em foco ocupa a janela inteira e a barra, a lista e os cartões flutuam sobre ela em vidro. Sem arte, o fundo pega a cor do jogo. Títulos em Poppins e textos em Plus Jakarta Sans.
- **Barra superior:** o logo do Tako abre um menu com Configurações (Ctrl + ,), Trocar perfil, Ir para a tela cheia, Procurar atualizações e Sair do Tako. A busca fica na barra e abre com Ctrl + K (Ctrl + F continua).
- **Lista de jogos:** abas em seletor, ícone da loja de cada jogo, ▶ na linha escolhida, inverter a ordem e filtrar na linha da contagem, e o polvo quando não há o que mostrar.
- **Grade e Categorias:** além da lista, a biblioteca pode aparecer como grade de capas ou em fileiras agrupadas por status, coleção, etiqueta, gênero, loja ou instalação, com "Ver todos". A escolha fica salva no perfil.
- **Seleção de vários jogos:** Ctrl ou Shift + clique e Ctrl + A marcam jogos para pôr numa coleção, etiquetar, favoritar, marcar como zerado ou apagar os adicionados à mão de uma vez.
- **Destaque do jogo:** etiqueta de estado, faixa com a última sessão e o tempo de jogo, Jogar em destaque (Voltar ao jogo enquanto ele roda, Instalar pela loja quando não está instalado) e botões para favoritar, mais ações e as Configurações deste jogo. Cartões Sobre o jogo (com "Mais"), Detalhes, Sessões e Trailer, quando o jogo tem.
- **Diálogos, painéis, menus e avisos** no mesmo vidro, com transições curtas que somem com "Reduzir animações".
- **Primeira abertura em tela inteira:** lojas, emuladores (quando há algum), capas, como abrir o Tako (em janela ou em tela cheia) e o resumo dos jogos encontrados. A escolha vira "Abrir em", em Configurações › Geral.
- Depois de favoritar ou mexer em coleções de vários jogos, o aviso oferece **Desfazer**. Botões só de ícone e nomes longos mostram uma dica ao parar o mouse em cima.
- **Configurações** com a mesma estrutura da biblioteca, sobre o cinza, e busca na barra (Ctrl + K); a bateria do controle aparece em Controles.
- **Reduzir transparência** em Configurações › Aparência deixa painéis e botões opacos no modo desktop.
- A tela cheia não muda.

## [0.2.0-alpha.6]
### Novidades
- **Modo desktop:** o Tako agora abre numa janela para mouse e teclado, com a biblioteca em lista, o destaque do jogo com Jogar, favoritar e mais ações, busca, filtrar e ordenar, abas Todos, Favoritos e Coleções, e as Configurações dentro da janela.
- **Tela cheia como "Big Picture":** a interface para controle e TV continua lá. Troque pelo botão Tela cheia ou F11, e volte pela Central ou por Configurações › Geral. Os temas valem na tela cheia.
- Com um controle conectado, o Tako pergunta se você quer a tela cheia ao abrir (dá para não perguntar de novo). Aberto com `--big-picture`, pelo modo Xbox (AnyFSE) ou pelo Console Mode, vai direto para a tela cheia.
- No desktop, a janela fica minimizada durante o jogo e volta quando ele fecha.

## [0.2.0-alpha.5]
### Novidades
- **Assistente da primeira execução:** idioma, lojas encontradas e provedores de capas, com controle ou teclado.
- **Organização:** tags próprias (aplicadas em lote e usadas como filtro), coleções automáticas que se atualizam sozinhas a partir dos filtros, e quanto cada jogo e cada disco ocupam, com ordenar por tamanho.
- **Perfis locais:** várias pessoas no mesmo PC, cada uma com biblioteca, favoritos, tempo de jogo e configurações próprios, e a pergunta "Quem está jogando?" ao abrir.
- **Lojas:** jogos que você tem e não instalou (Steam, Epic e GOG; Amazon no Linux e no macOS), com Instalar pela loja e aviso quando termina. Jogos não Steam adicionados à Steam. Emuladores e ROMs (RetroArch, ES-DE, Dolphin, PCSX2, DuckStation, PPSSPP, Ryujinx, Cemu, RPCS3 e pastas de ROMs), cada um abrindo no emulador certo.
- **Capas da biblioteca inteira:** capa, fundo, logo e ícone sozinhos, e "Revisar capas" mostra só o que ficou em dúvida.
- **Ao jogar:** aviso quando falta o app da loja, com link para a página oficial; "Reparar arquivos" nos jogos da Steam; aviso quando o jogo fecha logo ao abrir, não abre ou fecha com erro, com o código e o que ele costuma significar; variáveis de ambiente e comandos antes e depois de jogar, com a sua permissão.
- **Tela:** cada jogo pode abrir em outra tela, com resolução e HDR próprios, e o Tako lembra a resolução e o HDR de cada monitor.
- **Controles:** escolha os botões de cada ação do Tako, com um perfil por controle e o atalho da Central entre eles. Com o HidHide (opcional), o jogo deixa de ver o controle enquanto a Central está aberta (Windows).
- **Acessibilidade:** tamanho do texto, sons de navegação com volume próprio, e "Reduzir animações" agora vale também para a abertura do jogo.
- **Links `tako://`** abrem uma tela ou um jogo de fora do Tako (atalhos, Stream Deck), no Windows e no Linux.
- **Temas:** instalar um tema pelo endereço do repositório no GitHub e atualizá-lo depois; documentação da API de temas e `tako theme check` para autores.
- **Windows:** modo Xbox do Windows 11 pelo AnyFSE, com os passos em Configurações › Geral.
- **Desempenho do Tako** em Configurações › Sistema: memória, CPU e tempo de abertura.
- **Versões noturnas:** uma build por dia da develop, para quem escolher em Configurações › Geral.

## [0.2.0-alpha.4]
### Correções
- O portable (e o instalado) não pede mais atualização para a versão que já está aberta.
- Passar o mouse sobre os cards não faz mais a fileira nem a página rolarem; o mouse só destaca, e quem rola é o controle ou o teclado.
- Menos memória: o cache de imagens ficou bem menor e a biblioteca com muitos jogos não fica mais decodificando capas sem parar.
- Dentro da Central, o atalho de teclado (Ctrl+Shift+Tab por padrão) volta para o jogo, como a dica na tela já dizia.

## [0.2.0-alpha.3]
### Novidades
- Status de cada jogo: Jogando, Zerado, Pausado, Abandonado ou Quero jogar. Muda pelo menu do card ou por Opções nos detalhes, aparece no cabeçalho dos detalhes e tem filtro próprio.
- Jogos quebrados: o Tako confere sozinho os jogos adicionados à mão cujo arquivo ou pasta sumiu. O filtro "Só quebrados" mostra todos de uma vez, e jogar um deles oferece Editar ou Remover.
- Histórico de sessões: cada vez que um jogo roda fica registrada a data, a hora e a duração (as últimas 100 por jogo), em Opções › Sessões nos detalhes.
- Bateria dos controles sem fio no canto superior e na Central, com aviso quando fica fraca.
- Se o controle desconecta, o Tako avisa e não perde o lugar; ao reconectar, a navegação continua de onde parou.

## [0.2.0-alpha.2]
### Novidades
- Abrir o Tako de novo traz a janela que já está aberta para a frente. Durante o jogo, abre a Central por cima do jogo.
- O atalho de teclado da Central volta a ser escolhido apertando as teclas, como antes.
- Ao adicionar um `.exe` no Windows, o nome do jogo vem do nome do produto gravado no arquivo, e não mais do nome do arquivo.
- Seletor de imagens: o SteamGridDB ganha filtros (estilo, formato, animadas) no botão X e carrega mais imagens ao chegar no fim da lista.
- Fontes e IDs: um ID digitado só é usado depois de confirmar ("Usar {jogo}?").

### Correções
- Diálogos que abrem sozinhos (a pergunta "Você consegue ver esta tela?", a prévia do Playnite, o aviso de atualização pronta e o relatório do sistema) só apareciam depois de apertar uma tecla. Agora aparecem na hora.
- Windows: a release sai só para x64 (Intel/AMD).

## [0.2.0-alpha.1]
### Novidades
- **O Tako agora é escrito em Rust, com interface em Slint, e roda no Windows, no Linux e no macOS.** Instalador e portátil no Windows; deb, AppImage e tar.gz no Linux; dmg e zip no macOS. (PRD v0.2)
- Durante o jogo, a janela do Tako fecha e fica só um processo pequeno, sem janela e sem desenhar nada. Ele acompanha o jogo, esconde o cursor, move o ponteiro pelo controle e devolve tela e áudio no fim. (ADR-0026)
- **Central rápida reduzida:** Início sem fechar o jogo, voltar ao jogo, áudio, tela, ponteiro e encerrar jogo. Abre com Select + Y no controle e, no Windows, com Ctrl+Shift+Tab.
- **Temas trocáveis na hora**, incluindo o novo "Tako Alto contraste". Um tema pode trocar só as cores do padrão (`tokens.toml`). Se um tema quebrar, o Tako volta ao padrão.
- Linux: lojas Steam (nativa, Flatpak e Snap), Heroic e Lutris. macOS: Steam e Heroic (Epic e GOG). "Adicionar jogo" aceita `.desktop`, AppImage, binários e `.app`.
- Quem vem das alphas mantém biblioteca, coleções, tempo de jogo, capas escolhidas e configurações. As chaves dos provedores vão para o cofre de senhas do sistema.

### Mudanças
- Saíram do Tako e vão para o Console Mode: controle da TV, contador e limite de FPS, Steam na bandeja e "modo console ao abrir". (ADR-0025)

### Para desenvolvedores
- Workspace Rust em `crates/`; ADR-0023 a ADR-0026. Os ADRs 0011 a 0022 viraram histórico.
- `tako --bench` e o job `bench` do CI medem a §8.1 do PRD. Estado da paridade em `docs/marco-2/paridade.md`; roteiros de teste em `docs/testing/`.

## [0.1.0-alpha.10]
### Novidades
- Mais lojas na biblioteca: jogos da **Epic Games**, do **GOG** e do **Xbox / PC Game Pass** aparecem sozinhos e abrem pelo próprio cliente da loja. Cada loja liga e desliga em Configurações › Biblioteca e lojas, e um jogo presente em duas lojas aparece uma vez para cada loja, com o aviso "Também em". (#39, #40)
- **Menu Modo Console** na barra lateral, com cartões para Tela de jogo, TV, Áudio do jogo, Steam na bandeja, Contador de FPS e Limite de FPS, cada um mostrando se está ligado, desligado ou precisa de atenção. O assistente "Configurar para jogar na TV" guia passo a passo. (#45)
- **Modo console ao abrir:** com a opção ligada, o Tako abre direto na TV e só devolve as telas para a mesa ao sair ou em "Voltar ao PC". Atalho de emergência: Ctrl+Alt+Shift+D. A volta para a mesa não trava mais a tela. (#42)
- **TVs LG (webOS) e Samsung (Tizen)**, além das Google TV e Android TV: ligar, trocar para a entrada do PC e colocar em espera. (#38)
- **Coleções e configurações por jogo:** crie coleções, que aparecem na tela inicial e nos filtros. Cada jogo pode ter a própria tela de jogo, resolução, HDR, saída de áudio, limite de FPS e contador. O botão ☰ numa capa abre as ações rápidas. (#41)
- **Dados dos jogos pela IGDB e pelo ScreenScraper**, com a sua própria chave (Twitch) ou conta, guardadas cifradas neste PC. A Steam continua sendo consultada para todos os jogos. O ScreenScraper identifica ROMs de emulador pelo arquivo, e "Adicionar jogo" já aceita ROMs com a escolha do sistema. Abrir a ROM no emulador vem depois. (#46, #47)
- **Central de Controle com o jogo ao vivo por trás**, como na Steam, quando o jogo roda em janela ou sem bordas. O atalho agora é segurar **Select + △** (Create + △ no PlayStation, View + Y no Xbox) ou **Ctrl+Shift+T**, e os dois podem ser trocados em Configurações › Controles. (#44)
- Configurações › Sistema › **Teste de controle** mostra ao vivo o que o controle envia e copia o resultado para você mandar quando algo der errado. (#43)

### Correções
- **DualSense e DualSense Edge pelo Bluetooth com a Steam fechada:** o direcional para cima ficava preso e os botões não respondiam. O Tako também soma o controle virtual da Steam e o PlayStation sem um cancelar o outro, e ignora o eco de setas do teclado que a Steam envia. (#43)
- **Configurações:** organizadas em categorias, sem itens cortados pela barra de dicas, sem textos técnicos e com as opções de desenvolvedor escondidas. A barra de dicas encurta em vez de cortar, e senhas e chaves aparecem mascaradas. (#45)
- **Janelas abertas na Central:** os títulos não ficam mais cortados e o cartão focado aparece inteiro. (#44)

### Para desenvolvedores
- ADR-0019 (fontes de biblioteca), ADR-0020 (organização das Configurações), ADR-0021 (IGDB) e ADR-0022 (ScreenScraper). O checklist de hardware vai até a seção 24.
- A credencial de software do ScreenScraper vem dos segredos `SCREENSCRAPER_DEVID` e `SCREENSCRAPER_DEVPASSWORD` do CI. Sem eles, o ScreenScraper aparece como indisponível.

## [0.1.0-alpha.9]
### Novidades
- Contador de FPS próprio, sem RTSS: o Tako mede o FPS, o tempo de quadro e o 1% low pelo PresentMon da Intel, como o CapFrameX, sem entrar no jogo. Acabam os conflitos com o MSI Afterburner e com anti-cheat. O contador aparece numa janela própria por cima do jogo, nos estilos Só FPS, Compacto e Detalhado. (#36)
- Para medir, o Windows precisa liberar o Tako uma única vez. "Mostrar FPS" é ligado nas Configurações, com teclado e mouse, porque a confirmação do Windows não aceita controle. Depois disso é preciso sair e entrar no Windows uma vez. No instalador há a opção "Ativar contador de FPS", desmarcada por padrão, para resolver tudo na instalação. Na Central de Controle, enquanto o contador não estiver ativo, as opções de FPS ficam esmaecidas com o aviso. (#36)
- Limite de FPS pelo driver de vídeo (NVIDIA, AMD e Intel), também sem RTSS. O valor que você tinha volta no fim do jogo e também depois de uma queda do Tako. Opções: sem limite, 30, 40, 45, 60, 90, 120, 144 e "Tela − 3" para VRR. (#35)

### Mudanças
- O Tako não usa nem altera mais o RTSS. Se você usa RTSS ou Afterburner, eles continuam como estão. Um limite aplicado pelo RTSS em versões anteriores do Tako não é desfeito sozinho. (#36)
- Em tela cheia exclusiva o contador não aparece por cima do jogo, mas a Central de Controle mostra o FPS. (#36)

### Para desenvolvedores
- [ADR-0017](docs/adr/0017-contador-de-fps-com-presentmon.md) (contador de FPS e permissão) e [ADR-0018](docs/adr/0018-limite-de-fps-pelo-driver.md) (limite pelos drivers). O checklist de hardware ganhou as seções 13 e 14.

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
