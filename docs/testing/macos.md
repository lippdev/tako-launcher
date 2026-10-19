# Checklist de hardware: macOS

Valida a matriz da [ADR-0024](../adr/0024-matriz-de-plataforma.md). Rodar em Apple Silicon e Intel, com a tela integrada e com um monitor ou TV externos. Anotar versão do macOS e o resultado na ADR.

## Telas

- [ ] A lista mostra "Tela integrada" e as externas, com a principal certa e o modo atual.
- [ ] Os modos aparecem do maior para o menor; HiDPI e não HiDPI do mesmo tamanho não se repetem.
- [ ] Trocar a resolução e a taxa (ex.: 120 Hz para 60 Hz numa tela ProMotion ou externa) e voltar.
- [ ] Tornar a TV a principal: o Dock e a barra de menus vão para ela; restaurar volta.
- [ ] Pedir para desligar uma tela: aparece "não suportado", nada muda.
- [ ] Jogar e sair: telas voltam como estavam; restaurar duas vezes não muda nada na segunda.
- [ ] Depois de sair e entrar de novo na sessão, a configuração do sistema é a original (`ForSession`).
- [ ] Ids das telas iguais entre duas execuções do Tako (e depois de reiniciar o Mac).

## Áudio

- [ ] A lista mostra as saídas (alto-falantes, fones, HDMI, AirPlay) com a padrão certa.
- [ ] Trocar a saída padrão para a TV: o som sai na TV.
- [ ] Volume e mudo mudam e voltam ao sair do jogo; testar uma saída sem volume principal (só canais).

## Cursor

- [ ] O cursor some sobre o Tako.
- [ ] Com o jogo em primeiro plano: anotar se o cursor continua escondido (limitação mostrada na UI).
- [ ] Encerrar o Tako à força com o cursor escondido: o cursor volta.

## Cursor pelo controle

- [ ] Sem a permissão de Acessibilidade: aparece a orientação com o caminho em Ajustes do Sistema.
- [ ] Depois de permitir: o analógico move o cursor; cliques esquerdo, direito e do meio funcionam.
- [ ] Arrastar com o esquerdo segurado move janelas.
- [ ] Rolagem vertical e horizontal na direção certa (anotar se a horizontal está invertida).

## Jogos e processos

- [ ] Abrir um `.app`, um executável e uma URI `steam://rungameid/<id>`.
- [ ] Argumentos chegam ao `.app` (`open -a ... --args`).
- [ ] Com um jogo da Steam rodando, o Tako detecta o `RunningAppID`.
- [ ] Encerrar jogo fecha o jogo sem matar o Tako; "Abrir pasta" abre o Finder.
- [ ] Variáveis de ambiente do jogo (SESSION-004): num executável, o jogo recebe as variáveis; num `.app` fechado, elas
  chegam por `open --env` (com o app já aberto, não chegam). Numa URI, o jogo abre sem elas e o log avisa.
- [ ] Comando antes de jogar (por `sh -c`, ex.: `say antes; sleep 2`): o jogo só abre depois que ele termina; comando
  que sai com erro ou passa do limite mostra o aviso e o jogo abre do mesmo jeito. Comando depois de jogar roda ao
  fechar o jogo.
- [ ] Ferramentas junto do jogo (EXT-001, ex.: `/System/Applications/Calculator.app`): pedem permissão, abrem antes do jogo sem travar a interface e,
  com "Fechar as ferramentas no fim", fecham quando o jogo termina.

## Abrir com o sistema

- [ ] Ligar a opção cria `~/Library/LaunchAgents/dev.lipp.tako.plist`; o Tako abre no próximo login.
- [ ] Desligar a opção apaga o arquivo.

## Tela de jogo, Central e temas

- [ ] Configurações › Tela e áudio › Testar: o Tako vai para a tela de jogo, pergunta "Você consegue ver esta tela?" e as telas voltam com ou sem resposta (15 s).
- [ ] Com um jogo aberto, segurar Select + Y (Create + △) abre a Central. Não há atalho de teclado durante o jogo neste SO.
- [ ] Aparência › Tema › "Tako Alto contraste" troca na hora e continua em tela cheia.

## Emuladores e ROMs (SRC-001)

- [ ] RetroArch, Dolphin, PCSX2, DuckStation e PPSSPP em `/Applications` (ou `~/Applications`): aparecem como "Encontrado" em Biblioteca › Emuladores e ROMs.
- [ ] As playlists do RetroArch (`~/Library/Application Support/RetroArch`) e as pastas de jogos dos outros entram na loja Emuladores.
- [ ] Abrir um jogo de cada: tela cheia, direto no jogo, e o Tako acompanha até fechar (o macOS pode pedir permissão para o emulador ler a pasta das ROMs: anotar).

## Jogos não instalados: instalando e espaço livre (SRC-005)

- [ ] Instalar em um jogo da conta Steam abre `steam://install/<appid>`; em até 3 s o jogo aparece "instalando" com a porcentagem e, ao terminar, vira instalado sozinho.
- [ ] Cancelar a instalação na Steam: o jogo volta a "não instalado". Pausar: a porcentagem para.
- [ ] O espaço livre mostrado é o do volume da Steam (`~/Library/Application Support/Steam`) e bate com o do Finder (Obter Informações do disco). No APFS o volume do sistema aparece como `/`.
