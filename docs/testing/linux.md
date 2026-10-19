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
- [ ] VRR (PERF-002), com uma tela FreeSync/Adaptive Sync:
  - [ ] KDE: com `kscreen-doctor output.<tela>.vrrpolicy.never`, Configurações › Tela de jogo mostra "Ligar o VRR" disponível. Ligue e abra um jogo: `kscreen-doctor -j` mostra `"vrrPolicy": 2` durante o jogo e `0` depois de fechar. Com a política já em automático antes, o Tako não muda nada ao fechar.
  - [ ] KDE com uma tela sem VRR: o JSON não traz `vrrPolicy` e a linha aparece desligada com "Esta tela não informa VRR".
  - [ ] Sway/Hyprland: `wlr-randr --json` traz `adaptive_sync`; com ele `false`, ligar o VRR para o jogo deixa `true` durante o jogo e `false` depois. Sem o campo (wlr-randr antigo), a linha fica indisponível.
  - [ ] X11: a linha aparece indisponível com o motivo (o VRR é do driver, `Option "VariableRefresh"`); `xrandr --prop` mostra `vrr_capable` só para conferência.
  - [ ] GNOME: a linha aparece indisponível com o motivo (ainda não implementado).
  - [ ] Configurações deste jogo › "VRR" em Ligado/Desligado vence o padrão da tela.
  - [ ] Feche o Tako à força durante o jogo (`kill -9`): ao abrir de novo, o VRR volta a desligado. Com "Restaurar tela e som ao sair do jogo" desligada, o VRR fica ligado depois do jogo (é o esperado).
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
- [ ] Ferramentas junto do jogo (EXT-001, ex.: `/usr/bin/xeyes`): pedem permissão, abrem antes do jogo sem travar a interface e,
  com "Fechar as ferramentas no fim", fecham quando o jogo termina.

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

## Jogos não instalados: instalando e espaço livre (SRC-005)

- [ ] Steam nativa e Flatpak: Instalar em um jogo da conta abre `steam://install/<appid>`; em até 3 s o jogo aparece "instalando" com a porcentagem e, ao terminar, vira instalado sozinho (os `appmanifest` do Flatpak ficam em `~/.var/app/com.valvesoftware.Steam`).
- [ ] Cancelar a instalação na Steam: o jogo volta a "não instalado". Pausar: a porcentagem para.
- [ ] Heroic (Epic/GOG/Amazon): o jogo mandado instalar só vira instalado na releitura de 30 s (o Tako não vê o andamento do Heroic).
- [ ] O espaço livre mostrado é o do disco da Steam (ou de `~` sem Steam), com o ponto de montagem certo (ex.: `/`, `/home`, `/mnt/jogos`) e o número igual ao de `df -h`. Biblioteca da Steam em outro disco montado: mostra o ponto de montagem dele.

## Bottles e itch.io (SRC-002)

### Bottles
- [ ] Garrafas nativas (`~/.local/share/bottles/bottles`) e do Flatpak `com.usebottles.bottles` (`~/.var/app/com.usebottles.bottles/data/bottles/bottles`) são descobertas.
- [ ] Programas adicionados a cada garrafa (`External_Programs` do `bottle.yml`) aparecem na biblioteca com a loja Bottles.
- [ ] O comando de lançamento usa `bottles-cli run -b <garrafa> -p <programa>` (nativo) ou `flatpak run --command=bottles-cli com.usebottles.bottles run -b <garrafa> -p <programa>` (Flatpak).
- [ ] Arquivo `bottle.yml` malformado ou corrompido não causa pânico e mantém a integridade da leitura.

### itch.io
- [ ] Banco `butler.db` em `~/.config/itch/db/` (ou Flatpak `~/.var/app/io.itch.itch/config/itch/db/`) é lido via SQLite em modo somente leitura.
- [ ] Jogos instalados (`caves` com `install_location` e `game`) aparecem na biblioteca com o título, capa e tempo de jogo.
- [ ] Executável do jogo é resolvido automaticamente ou via `.itch.toml`; quando encontrado, o Tako inicia o processo diretamente e rastreia o ciclo de vida e tempo de jogo.
- [ ] Na ausência do binário direto, o jogo faz fallback para lançamento via protocolo `itch://install?game_id=<id>&launch` (requer app itch na versão mínima 26.17.0, que trouxe `&launch`).
- [ ] Banco ausente ou corrompido não causa pânico e emite aviso na tela quando o cliente não estiver instalado.

## Pacotes do AUR e Flatpak (PLAT-002)

Roteiro de publicação e permissões em [packaging-linux.md](../packaging-linux.md). O CI valida os arquivos; isto é o que só máquina real mostra.

### AUR (`tako-launcher-bin` e `tako-launcher`) em Arch ou CachyOS

- [ ] `cd packaging/aur/tako-launcher-bin && makepkg -si`: instala sem erro; `pacman -Qi tako-launcher-bin` mostra as dependências; `pacman -Qlp`/`-Ql` lista `/usr/bin/tako`, `tako.desktop`, o ícone, a regra udev e a licença (e **não** `Tako_Data`).
- [ ] Idem para `packaging/aur/tako-launcher` (compila do código-fonte; anotar o tempo e a versão do Rust do Arch, o projeto pede 1.97).
- [ ] O Tako abre pelo menu de aplicativos, com o ícone; o dado vai para `~/.local/share/tako` (não para uma pasta ao lado do binário).
- [ ] `xdg-mime query default x-scheme-handler/tako` mostra `tako.desktop`; `xdg-open "tako://settings"` abre as Configurações (com o Tako fechado e aberto).
- [ ] Depois do `pacman -U`, o `uinput` já vale sem copiar a regra (preparação acima); sair e entrar na sessão e testar o cursor pelo controle.
- [ ] Configurações › Atualizações: o Tako só avisa que há versão nova e não oferece instalar (é o pacman que atualiza).
- [ ] Remover o pacote (`pacman -R`) não apaga `~/.local/share/tako`.
- [ ] CachyOS (KDE e Hyprland/gamescope): abrir, trocar a tela e o áudio como nas seções acima.

### Flatpak (`io.github.lippdev.Tako`)

- [ ] Gerar o `cargo-sources.json` e compilar como em packaging-linux.md; `flatpak run io.github.lippdev.Tako` abre.
- [ ] Biblioteca: Steam nativa e Steam Flatpak, Heroic nativo e Flatpak, Lutris nativo e Flatpak, itch.io nativo e Flatpak aparecem (um de cada que houver; o Bottles fica oculto no Flatpak). Biblioteca da Steam e do itch.io em `/mnt`, `/media` ou `/run/media` aparece; em outro caminho não aparece até `flatpak override --user --filesystem=<pasta>:ro io.github.lippdev.Tako`.
- [ ] Abrir um jogo da Steam e do Heroic/Lutris pelo Tako: abre no host (link da loja pelo portal). Anotar se o Tako acompanha o jogo até fechar (esperado: só a Steam pelo `RunningAppID`).
- [ ] Controle: o controle é reconhecido e navega; o cursor pelo controle move o mouse (precisa de `--device=all` e da regra udev no host).
- [ ] Áudio: lista de saídas, troca da padrão, volume e mudo. Segredos: salvar uma chave de API e reabrir.
- [ ] Telas: X11 e GNOME trocam o modo; KDE e wlroots escondem as opções de tela sem erro.
- [ ] `tako://settings` pelo navegador abre o Flatpak (`flatpak run` pelo `.desktop` exportado); Atualizações não oferece instalar.
- [ ] Anotar o que quebrou por causa da sandbox (executável, emulador, "Abrir com o sistema") para decidir sobre o backend `flatpak-spawn --host`.
- [ ] Com o Flatseal: `flatpak info --show-permissions io.github.lippdev.Tako` bate com o manifesto (sem `home` nem `org.freedesktop.Flatpak`).

## Downloads por torrent (DL-001)

- [ ] A pasta padrão é `$XDG_DOWNLOAD_DIR/Tako` (ou `~/Downloads/Tako`).
- [ ] Configurações › Downloads vem desligado. Ligar mostra o aviso legal e de IP; Cancelar mantém desligado; "Entendi e aceito" liga e não pergunta de novo.
- [ ] Adicionar fonte com `http://` é recusado; com uma fonte `https://` sua de teste (só itens fictícios ou de conteúdo livre), a linha mostra o número de itens. Remover apaga a fonte e o cache.
- [ ] Num jogo não instalado, ⋯ › "Baixar de uma fonte" lista só itens parecidos; confirmar vincula ao jogo, a pílula "↓ N%" aparece antes do perfil e o jogo mostra o progresso na lista.
- [ ] No painel: pausar para na hora (sem confirmação); cancelar pede confirmação e "Cancelar e apagar" remove a pasta. Com uma instalação da Steam em andamento, ela aparece em BAIXANDO "pela Steam" e os botões abrem o cliente.
- [ ] Ao terminar: aviso, item em CONCLUÍDOS com "Abrir pasta"; o gerenciador de arquivos abre na pasta e nada é executado. O envio para (sem semear).
- [ ] Reabrir o Tako com um download em andamento: ele volta pausado e continua ao retomar.
- [ ] Durante um jogo com a janela minimizada, o download segue (confira o tamanho da pasta) e nada anima.
