# Checklist de hardware: Linux

Valida a matriz da [ADR-0024](../adr/0024-matriz-de-plataforma.md). Rodar em cada sessão disponível (X11, KDE Wayland, GNOME Wayland, Sway ou Hyprland, gamescope/Steam Deck) e anotar o resultado na ADR. Antes de cada bloco, anote a sessão: `echo $XDG_SESSION_TYPE $XDG_CURRENT_DESKTOP`.

## Preparação

- [ ] Instalar a regra: `sudo cp packaging/linux/60-tako-uinput.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger`; sair e entrar de novo na sessão.
- [ ] `ls -l /dev/uinput` mostra acesso para o usuário (`getfacl /dev/uinput`).
- [ ] KDE: `kscreen-doctor -j` funciona. wlroots: `wlr-randr --json` funciona. Áudio: `pactl -f json list sinks` ou `wpctl status`.

## Telas

- [ ] A lista mostra todas as telas conectadas, com a principal certa e o modo atual.
- [ ] Os modos de cada tela aparecem do maior para o menor, sem repetição.
- [ ] Trocar para outra resolução e taxa: a tela muda; voltar restaura.
- [ ] Taxa fracionária (59,94 Hz) é escolhida quando pedida "60" e não existe 60,00.
- [ ] Com duas telas: ligar só a TV, jogar, sair do jogo: as duas voltam como estavam (posição, principal, modo).
- [ ] Restaurar duas vezes seguidas não muda nada na segunda.
- [ ] GNOME: depois de restaurar, nada foi gravado em `~/.config/monitors.xml` (método temporário).
- [ ] KDE com escala 125%: trocar o modo mantém a escala.
- [ ] KDE Plasma 6 com tela HDR: ligar e desligar o HDR.
- [ ] gamescope: a tela aparece com o tamanho da sessão e as opções de troca ficam indisponíveis.
- [ ] Compositor sem suporte: as opções de tela somem, sem erro na tela.

## Áudio

- [ ] A lista mostra as saídas com nomes legíveis e a padrão certa.
- [ ] Trocar a saída padrão para a TV (HDMI): o som do jogo sai na TV.
- [ ] Volume e mudo por saída mudam e voltam ao sair do jogo, sem pico de som.
- [ ] Sem `pactl` (só `wpctl`): o mesmo funciona.

## Cursor

- [ ] X11: o cursor some sobre o jogo e volta ao sair.
- [ ] X11: matar o Tako (`kill -9`) com o cursor escondido: o cursor volta sozinho.
- [ ] Wayland: a explicação "só sobre a janela do Tako" aparece; sobre o Tako o cursor some.
- [ ] Wayland com jogo pelo Proton (Xwayland): anotar se o cursor aparece sobre o jogo.
- [ ] gamescope: anotar se o cursor some.

## Cursor pelo controle

- [ ] Sem a regra udev: aparece a orientação para instalar a regra.
- [ ] Com a regra: o analógico move o cursor em X11, Wayland e gamescope.
- [ ] Clique esquerdo, direito e do meio; arrastar com o esquerdo segurado.
- [ ] Rolagem vertical (para cima = para cima) e horizontal (para a direita = para a direita).

## Jogos e processos

- [ ] Abrir um executável, um script `.sh` sem permissão de execução, um AppImage e um `.desktop`.
- [ ] Abrir `steam://rungameid/<id>`, um jogo do Heroic (`heroic://`) e do Lutris (`lutris:rungameid/<id>`).
- [ ] Com um jogo da Steam rodando, o Tako detecta o `RunningAppID` (Steam nativa e Flatpak).
- [ ] Encerrar jogo fecha o jogo (SIGTERM) sem matar o Tako.
- [ ] "Abrir pasta" abre o gerenciador de arquivos.
- [ ] Variáveis de ambiente do jogo (SESSION-004): num executável ou script, o jogo recebe as variáveis (ex.: `MANGOHUD=1`
  liga o MangoHud). Num `.desktop` ou numa URI, o jogo abre sem elas e o log avisa "variáveis de ambiente só valem
  para executáveis".
- [ ] Comando antes de jogar (por `sh -c`, ex.: `notify-send antes; sleep 2`): o jogo só abre depois que ele termina;
  comando que sai com erro ou passa do limite mostra o aviso e o jogo abre do mesmo jeito. Comando depois de jogar
  roda ao fechar o jogo.

## Abrir com o sistema

- [ ] Ligar a opção cria `~/.config/autostart/tako.desktop`; o Tako abre no próximo login.
- [ ] No AppImage, o `Exec` aponta para o arquivo `.AppImage`, não para `/tmp/.mount_*`.
- [ ] Desligar a opção apaga o arquivo.

## Tela de jogo, Central e temas

- [ ] Configurações › Tela e áudio › Testar: o Tako vai para a tela de jogo, pergunta "Você consegue ver esta tela?" e as telas voltam com ou sem resposta (15 s).
- [ ] Com um jogo aberto, segurar Select + Y (Create + △) abre a Central. Não há atalho de teclado durante o jogo neste SO.
- [ ] Aparência › Tema › "Tako Alto contraste" troca na hora e continua em tela cheia.

## Emuladores e ROMs (SRC-001)

- [ ] RetroArch do pacote do sistema (no PATH), do Flatpak (`org.libretro.RetroArch`) e da Steam: aparece como "Encontrado" em Biblioteca › Emuladores e ROMs, e as playlists dele entram na loja Emuladores.
- [ ] DuckStation ou PCSX2 em AppImage dentro de `~/Applications`: encontrado; a pasta de jogos configurada nele entra na biblioteca.
- [ ] Emulador em Flatpak: o jogo abre por `flatpak run <id>` (ROM fora de `~` pode precisar de permissão de pasta no Flatseal: anotar).
- [ ] ES-DE: os jogos da pasta `~/ROMs` aparecem com os nomes e capas do ES-DE; os escondidos nele não aparecem.
- [ ] Abrir um jogo de cada emulador: tela cheia, direto no jogo, e o Tako acompanha até fechar.
