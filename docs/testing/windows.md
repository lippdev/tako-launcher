# Teste manual no Windows (MIG-020 a MIG-024)

Roteiro curto para validar em hardware real o que a CI não alcança. Use um PC com Windows 10 ou 11, um monitor e, se possível, uma TV na HDMI, um fone ou caixa de som extra e um controle (Xbox ou PlayStation).

Antes de começar: abra o Tako uma vez e confira na pasta de dados (`%LOCALAPPDATA%\Tako` na versão instalada, ao lado do executável na portátil) que não existe o arquivo `cursor-hidden`.

## 1. Resolução e taxa (MIG-020)

1. Em Configurações › Tela, confira que a lista mostra todas as telas conectadas, com o nome do monitor (não `\\.\DISPLAY1`).
2. Escolha uma resolução e taxa diferentes da atual (ex.: 1920 × 1080 · 60 Hz) e aplique.
3. A imagem muda e o Tako pede confirmação. **Não confirme**: espere o tempo acabar. A resolução anterior volta sozinha.
4. Repita e **confirme**. A nova resolução fica.
5. Configure um jogo com "Tela de jogo" = só a TV, resolução 4K e HDR ligado. Abra o jogo.
6. Durante o jogo: só a TV está ligada, como principal, na resolução pedida, com HDR (Configurações do Windows › Tela mostra HDR ativo).
7. Feche o jogo. Monitor e TV voltam exatamente como antes: mesmas telas ligadas, a mesma principal, posições, resolução, taxa (59,94 continua 59,94), rotação e HDR.
8. Reinicie o PC com o jogo aberto (simula queda). Abra o Tako: a configuração anterior é restaurada.
9. Com uma tela na vertical (90°), repita 5–7 e confira que a rotação volta.

## 2. Saída de áudio (MIG-021)

1. Em Configurações › Áudio, a lista mostra todas as saídas ativas pelo nome (ex.: "Alto-falantes (Realtek)"), com a padrão marcada.
2. Configure o jogo para sair no fone. Abra o jogo: o som sai no fone. No Painel de Som do Windows, o fone é o padrão **e** o padrão de comunicação.
3. Feche o jogo: a saída padrão, o volume e o mudo de cada saída voltam como estavam.
4. Mude o volume e o mudo pelo Tako: o ícone de volume do Windows acompanha.
5. Se o Tako disser que não consegue trocar a saída padrão neste Windows, anote a versão do Windows (`winver`) no relato.

## 3. Esconder o cursor (MIG-022)

1. Ligue "Esconder o ponteiro" e abra um jogo: o ponteiro some em todo lugar, inclusive sobre outros apps.
2. Feche o jogo: o ponteiro volta, com o esquema do usuário (teste com um esquema diferente do padrão, ex.: "Windows Preto").
3. Queda: com o ponteiro escondido, encerre o Tako pelo Gerenciador de Tarefas. O ponteiro continua escondido e existe `cursor-hidden` na pasta de dados.
4. Abra o Tako de novo: o ponteiro volta na hora e o arquivo `cursor-hidden` some.

## 4. Mouse pelo controle (MIG-023)

1. Ligue o mouse pelo controle e abra um jogo ou emulador com menu que só aceita mouse.
2. Analógico esquerdo move o ponteiro, suave, mais rápido quanto mais empurrado.
3. Analógico direito rola para cima/baixo e para os lados.
4. A clica; dois toques = botão direito; segurar A arrasta (solte e confira que o botão não ficou preso).
5. Com o jogo rodando como administrador, o ponteiro não se move (limitação do Windows): confira que o Tako não trava nem enche o log.
6. Repita 2–4 com controle PlayStation.

## 5. Abrir jogos de cada loja (MIG-024)

Para cada um: abre, o Tako detecta o jogo rodando, "Encerrar jogo" fecha a janela (sem matar o processo) e, ao sair, o Tako volta com foco.

1. **Steam**: jogo instalado. Durante o jogo, o Tako reconhece o `RunningAppID`; o fim vale quando a Steam diz que o jogo parou.
2. **Epic**, **GOG Galaxy**, **EA app**, **Ubisoft Connect**, **Battle.net**: um jogo de cada (abre pela URI da loja).
3. **Xbox / Microsoft Store**: um jogo da Store (abre por `shell:AppsFolder\…`, sem PID): o Tako acompanha pelo processo do jogo.
4. **Jogo manual**: um `.exe` com argumentos e pasta de trabalho; um atalho `.lnk`; um `.bat`.
5. **Abrir pasta** do jogo: abre no Explorador de Arquivos.

## 6. Abrir ao entrar no Windows

1. Ligue "Abrir ao iniciar o Windows". Em `regedit`, `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` tem o valor `Tako` = `"<caminho do Tako.exe>" --startup`.
2. Saia e entre de novo no Windows: o Tako abre.
3. Mova a pasta do Tako portátil: a opção aparece desligada (o valor aponta para outra cópia).
4. Desligue a opção: o valor `Tako` some do registro.

## 7. Teste da tela de jogo

1. Configurações › Tela e áudio: escolha a TV como tela de jogo e um modo diferente do atual; "Testar".
2. O Tako vai para a TV e pergunta "Você consegue ver esta tela?". "Sim" volta as telas e mostra "Tela de jogo confirmada".
3. Repita sem responder: depois de 15 s as telas voltam sozinhas e o Tako volta para o monitor principal.

## 8. Central durante o jogo

1. Com um jogo aberto, segure View + Y (Create + △ no PlayStation): a Central abre por cima do jogo.
2. Com o teclado, Ctrl+Shift+Tab (ou o atalho escolhido em Configurações › Controles) abre a Central.
3. Depois do jogo, Ctrl+Shift+Tab volta a funcionar no navegador (o atalho foi liberado).

## 9. Importar do Playnite

1. Com o Playnite instalado e jogos manuais nele, Configurações › Biblioteca › Importar do Playnite.
2. Confirme a leitura: a prévia mostra jogos novos, jogos já existentes e coleções. "Importar" traz os jogos, favoritos, tempo e categorias como coleções.
3. Com o Playnite aberto, a leitura pode falhar com "está em uso": feche o Playnite e repita.

## 10. Migração das alphas C#

1. Num PC com uma alpha C# (biblioteca, coleções, escolhas de arte e chave da IGDB salvas), instale a versão Rust por cima.
2. A biblioteca, as coleções, o tempo de jogo e as capas escolhidas aparecem iguais. Existe `tako.db.alpha-backup` na pasta de dados.
3. Configurações › Capas e metadados: a IGDB aparece ligada e com a chave. Os arquivos `*.dat` da alpha viraram `*.dat.migrado`.

## 11. Temas

1. Configurações › Aparência › Tema: escolha "Tako Alto contraste". A tela muda na hora, sem reabrir, e continua em tela cheia.
2. "Instalar tema…" com um `.takotheme` válido: aparece a mensagem de instalado e o tema entra na lista.
3. Abra com `--safe-mode`: o tema padrão é usado; Configurações › Sistema › "Sair do modo seguro" reabre o Tako com o tema escolhido.

## 12. Modo Xbox pelo AnyFSE (PLAT-007)

Precisa de um Windows 11 com a experiência em tela cheia disponível (portátil como o ROG Ally ou ativada com o
XboxFullscreenExperienceTool).

1. Sem o AnyFSE: Configurações › Geral › "Modo Xbox do Windows" mostra "Como configurar"; o painel traz os passos, o
   caminho deste Tako.exe e o link da página.
2. Instale o AnyFSE (`AnyFSE.Installer.exe`). Reabra o painel: mostra "AnyFSE instalado".
3. Em AnyFSE › Configure, escolha executável personalizado com o caminho mostrado no painel. Em Configurações do
   Windows › Jogos › Experiência em tela cheia, escolha o AnyFSE como app inicial.
4. Entre no modo Xbox (Win+F11 ou reiniciando no modo): o Tako abre em tela cheia, sem a área de trabalho por trás, e o
   controle navega sem clicar antes.
5. Abra um jogo e feche-o: o Tako volta em tela cheia, com o foco no jogo. Repita pela Central (Encerrar jogo).
6. Saia do Tako (Sair para o desktop): o comportamento segue a opção "Exit FSE on Launcher exit" do AnyFSE.

## 13. Tela por jogo (PERF-002)

Precisa de duas telas (ex.: monitor e TV).

1. Configurações › Tela de jogo: escolha a TV, com uma resolução. Num jogo, Configurações deste jogo › "Tela deste jogo"
   mostra "Padrão: TV".
2. Troque para o monitor: "Usar a tela de jogo" passa a "Padrão: Sim" e "Resolução e taxa" a "Padrão: manter a atual";
   as resoluções oferecidas são as do monitor. Escolha uma e o HDR.
3. Abra o jogo: só o monitor fica ligado, com a resolução e o HDR escolhidos; a TV desliga. Ao fechar, tudo volta.
4. Desconecte o monitor: a linha mostra "Monitor (desconectada)" e abrir o jogo avisa que a tela não está conectada.
5. Outro jogo, sem tela própria, continua indo para a TV com a resolução do Tako.

Preset por monitor:

6. Configurações › Tela de jogo: com a TV, escolha 4K 120 Hz e ligue o HDR. Troque a tela para o monitor: resolução
   e HDR voltam ao padrão ("manter a atual", sem HDR). Escolha 1080p 60 Hz. Volte à TV: 4K 120 Hz e HDR voltam; volte
   ao monitor: 1080p 60 Hz volta. Feche e abra o Tako: continua assim.
7. Deixe a TV como tela de jogo. No jogo do passo 2, com o monitor como tela própria e "Resolução e taxa" em Padrão: a
   linha mostra "Padrão: 1920 × 1080 · 60 Hz" (o preset do monitor) e o HDR "Padrão: Desligado". Abra o jogo: o
   monitor fica em 1080p 60 Hz. Escolhendo "manter a atual" no jogo, a resolução não muda.
8. VRR (G-Sync/FreeSync) não entra no preset: o Windows não tem API pública para ligá-lo por tela.

## 15. Tema por endereço (THEME-004)

Precisa de internet e de um repositório no GitHub com uma release que tenha um `.takotheme` (pode ser um de teste).

1. Configurações › Aparência › Tema › "Instalar por endereço…": digite o endereço (ex.: `https://github.com/autor/tema`).
   Aparece "Baixando o tema…" e depois "instalado"; o tema entra na lista com o endereço na descrição.
2. Endereço errado (`gitlab.com/x/y`, repositório sem release ou release sem `.takotheme`): aparece o motivo, nada é
   instalado.
3. "Atualizar temas" (só aparece com tema por endereço): sem release nova, "já estão na última versão". Publique uma
   release nova no repositório e repita: o tema é atualizado e, se estiver em uso, a tela muda na hora.
4. Remova o tema: "Atualizar temas" some.

## 16. Botões e perfis por controle (INPUT-001)

Precisa de dois controles (ex.: um Xbox e um DualSense).

1. Configurações › Controles › "Botões do Tako · perfil Padrão": Aceitar mostra A. Aperte A na linha: Aceitar passa a
   B, Voltar a A, e o aviso diz as duas trocas. As dicas da barra de baixo e o texto "Aperte B para salvar" da captura
   da combinação seguem o botão novo.
2. "Perfis de botões": os dois controles aparecem, os dois no Padrão; o que está em uso diz "Controle em uso".
   "Novo perfil…" com o nome "Arcade": o controle em uso passa ao Arcade (uma cópia dos botões dele).
3. Use o outro controle: continua no Padrão (A aceita). Na linha dele, aperte A: passa ao Arcade e os botões seguem.
4. "Abrir com o controle" (na lista de botões do perfil Arcade): grave Guia + X. Abra um jogo: Guia + X abre a Central no controle do Arcade, e
   View + Y abre no controle do Padrão (cada um a sua). A combinação sem View nem Guia é recusada.
5. "Renomear perfil…" e "Excluir perfil": os controles do perfil excluído voltam ao Padrão.
6. Feche e abra o Tako: perfis, escolha de cada controle e combinações continuam. O teclado (Enter, Esc) não muda.
7. Dois controles do mesmo modelo têm o mesmo GUID no SDL: os dois usam o mesmo perfil (esperado).

## 17. Esconder o controle do jogo com a Central aberta (INPUT-005)

Precisa de um jogo que lê o controle sem estar em foco (ex.: Resident Evil 2) e, para os passos 2 a 5, do HidHide
instalado (https://github.com/nefarius/HidHide/releases, reinicie o PC depois).

1. Sem o HidHide: Configurações › Controles mostra "Driver opcional: esconder o controle do jogo"; A abre a página
   oficial no navegador. Numa instalação nova, a mesma linha aparece no passo 3 do assistente. Abra a Central durante
   o jogo: tudo como antes (o jogo anda junto).
2. Com o HidHide: a linha vira a opção "Esconder o controle do jogo com a Central aberta", ligada.
3. Abra o jogo, mexa no controle (o jogo responde), abra a Central e navegue: **o jogo não deve se mexer**. Volte ao
   jogo: o controle volta a funcionar nele.
4. Confira no HidHide Configuration Client: com a Central aberta, o controle aparece marcado e o `tako.exe` na lista de
   aplicativos; depois de voltar ao jogo, os dois somem (o que você tinha configurado antes continua lá).
5. Com a Central aberta, finalize o `tako.exe` pelo Gerenciador de Tarefas e abra o Tako de novo: o controle volta a
   aparecer para os outros programas.
6. Desligue a opção: a Central volta a se comportar como no passo 1.

Anote no PR: o passo 3 funciona com o controle já aberto pelo jogo, ou só depois de reconectar o controle? (O HidHide
documenta que mudanças valem para quem abre o controle depois; e controles Xbox por XInput podem continuar visíveis.)
Teste com um DualSense e com um controle Xbox.

## 18. Emuladores e ROMs (SRC-001)

Precisa de pelo menos o RetroArch (com um núcleo, ex.: Snes9x) e de algumas ROMs; melhor ainda com um emulador
avulso (DuckStation, PCSX2, Dolphin, PPSSPP, Ryujinx, Cemu ou RPCS3) já configurado com a pasta de jogos dele.

1. Configurações › Biblioteca: a loja "Emuladores" aparece. Sem nenhum emulador, ela diz como resolver.
2. Biblioteca › Emuladores e ROMs: cada emulador instalado aparece como "Encontrado" com o caminho (instalador,
   Scoop, RetroArch da Steam). Um que esteja em outra pasta: A → escolha o executável; "Procurar sozinho" desfaz.
3. "Adicionar pasta…": escolha uma pasta com uma subpasta por sistema (`snes`, `psx`, `gc`…) ou a pasta de um sistema
   e troque o sistema dela no diálogo. A biblioteca relê e os jogos aparecem em Emuladores.
4. Os jogos das playlists do RetroArch aparecem com o nome e a capa que o RetroArch baixou; os da pasta do
   DuckStation/PCSX2/Dolphin/etc. aparecem sem precisar adicionar nada no Tako. Um jogo de vários discos (`.m3u`)
   aparece uma vez; o `.bin` de um `.cue` não aparece sozinho.
5. Abra um jogo de cada emulador: ele abre em tela cheia, direto no jogo, e o Tako acompanha (Central, tempo
   jogado, volta ao fechar o emulador).
6. "Emulador de cada sistema": troque PlayStation entre Automático, DuckStation e RetroArch e abra o mesmo jogo:
   abre no escolhido.
7. Uma ROM de um sistema sem emulador (ex.: Wii U sem o Cemu) não aparece e entra no aviso da loja.
8. Adicionar jogo › uma ROM solta (ex.: `.sfc`): escolha o sistema; o botão vira "Jogar" e abre no emulador do
   sistema. Sem emulador para ele, aparece o aviso "Nenhum emulador deste PC abre este sistema".

## 19. Código de saída do jogo (SESSION-003)

Precisa de um executável que feche com erro. Crie `C:\TakoTeste\dll.bat` com `exit /b -1073741515` e
`C:\TakoTeste\erro.bat` com `timeout /t 15 >nul` e `exit /b -1073741819` na linha seguinte.

1. Adicionar jogo › `dll.bat`. Abra: o aviso "… fechou logo depois de abrir" começa com "Código de saída 0xC0000135
   (3221225781): uma DLL que o jogo precisa não foi encontrada…", seguido da sugestão de antes.
2. Adicionar jogo › `erro.bat`. Abra e espere os 15 s: o aviso "… fechou com um erro" mostra `0xC0000005` e a
   explicação de violação de acesso (passou dos 10 s e avisa mesmo assim).
3. Repita o passo 2 encerrando pela Central (Encerrar jogo) antes dos 15 s: volta com "Bem-vindo de volta", sem aviso.
4. Um jogo de verdade que feche normalmente depois de jogar: "Bem-vindo de volta", sem aviso.
5. Um jogo aberto pela loja (Steam, Epic) que feche sozinho em poucos segundos: o aviso de antes, sem código.
6. Em `logs\`, a linha "terminou fora do normal" traz o código em hex.

## 20. Jogos não instalados: instalando e espaço livre (SRC-005)

Precisa de Steam, Epic e GOG Galaxy com conta, e da chave da Web API da Steam em Configurações › Biblioteca e lojas.

1. Abrir o Tako: a lista de não instalados da Steam já vem atualizada (sem esperar 1 hora). Em "Procurar jogos instalados" também.
2. Escolher um jogo pequeno da Steam › Instalar. A Steam abre a janela de instalação; confirme. Em até 3 s o jogo passa a
   "instalando" com a porcentagem andando (pausar na Steam congela o número; cancelar volta a "não instalado").
3. Quando a Steam termina, o jogo vira instalado sozinho, com o aviso "foi instalado", sem reiniciar o Tako.
4. Se a Steam estiver numa fila de downloads (jogo esperando a vez), conferir se o Tako mostra "instalando" sem porcentagem ou
   ainda "não instalado" (o `StateFlags` dessa fila não foi confirmado; anotar o valor do `appmanifest_<id>.acf`).
5. Epic: Instalar abre o Launcher na página do jogo. Durante a instalação, conferir se o jogo aparece como "instalando"
   (depende do `bIsIncompleteInstall` do `.item` aparecer logo no começo; sem porcentagem) e se vira instalado no fim.
6. GOG: Instalar abre o Galaxy na página do jogo (`goggalaxy://openGameView/<id>`). O Tako não vê o andamento do
   Galaxy: o jogo só vira instalado quando a releitura de 30 s encontra o jogo instalado.
7. Jogo da conta em duas lojas (ex.: Steam e GOG) não instalado: as duas lojas aparecem como opção de instalar.
8. O espaço livre mostrado é o do disco da Steam (biblioteca padrão) e bate com o Explorador de Arquivos; sem Steam, é o do `C:`.
   Com a Steam em outro disco (ex.: `D:\Steam`), mostra `D:`.
