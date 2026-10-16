# ADR-0024: Emulação — emuladores detectados por tabela, pastas de ROMs como fonte da biblioteca e saída pelo controle

- Status: Proposta
- Data: 2026-10-16

## Contexto

O épico EMU do PRD pede detectar emuladores (EMU-001), cadastrar emuladores personalizados (EMU-002), importar pastas
de ROMs (EMU-003), parâmetros por emulador (EMU-004), RetroArch (EMU-005), perfis de controle (EMU-006), capas e
metadados (EMU-007) e inicialização com retorno ao launcher (EMU-008). Aceite: executar conteúdo legalmente obtido com
um emulador configurado, preservando o retorno à interface principal.

Já existia: `Game.Platform`/`Game.RomPath` e "Adicionar jogo" aceitando ROMs com a escolha do sistema
([ADR-0022](0022-metadados-screenscraper.md)), mas o botão dizia "Abrir no emulador (em breve)"; ScreenScraper e IGDB
por plataforma; fontes de biblioteca ligáveis ([ADR-0019](0019-fontes-de-biblioteca.md)); a sessão de jogo real
(tela de jogo, áudio, TV, Central de Controle, FPS, acompanhamento do processo e volta com foco).

## Decisão

1. **Tabela de emuladores em dados** (`Launcher.Core/Emulation/emulators.json`, embutida e testada): RetroArch e os
   standalone comuns — Dolphin, PCSX2, DuckStation, PPSSPP, RPCS3, Cemu, Ryujinx (e Ryubing), os derivados do yuzu
   (Eden, Citron, Sudachi, Suyu), xemu, Xenia (e Canary), melonDS, mGBA, Flycast, Redream, MAME e Vita3K. Cada entrada
   tem nome, executáveis aceitos, sistemas (chaves de `ScreenScraperSystems`), argumentos padrão, a opção de tela
   cheia, a tecla de sair do próprio emulador, pastas padrão, apps do Scoop, ids do winget e nomes em "Aplicativos
   instalados". A tabela também traz: nomes de pasta por sistema (inclui os do ES-DE: `psx`, `gc`, `genesis`…), a ordem
   de preferência por sistema (PS2 → PCSX2 antes do RetroArch), os cores sugeridos do RetroArch por sistema, o mapa
   "banco de dados do `.info`" → sistema, as opções de atalho do `retroarch.cfg` e as BIOS conhecidas dos standalone.
   Emulador novo ou argumento corrigido = editar o JSON; o teste confere a consistência (sistemas existem, marcadores
   conhecidos, todo sistema tem pasta reconhecida e alguém que o abra, menos o 3DS).
2. **Detecção** (`EmulatorDetector`, EMU-001), só lendo disco e registro, nesta ordem: executável escolhido pelo
   usuário → pastas padrão (e uma subpasta a mais, para zips extraídos) → Scoop (`%SCOOP%`/`~\scoop` e o global) →
   winget (`WinGet\Packages\<id>_*`) → registro Uninstall (HKLM, WOW6432Node e HKCU: `InstallLocation`, senão a pasta do
   `DisplayIcon`). O disco e o registro ficam atrás de interfaces (`IEmulationFileSystem`, `IRegistryReader`) e os
   caminhos do Windows são tratados por `WinPath`, então os testes rodam no Mac/Linux com um PC fictício
   (`Fixtures/Emulation/pc.txt`). RetroArch: os cores vêm de `cores\*_libretro.dll` e o `info\*.info` diz sistemas
   (`database`), extensões e firmware; sem `.info`, vale a tabela de sugestões; core sem sistema conhecido é ignorado.
3. **Emuladores personalizados** (EMU-002): qualquer `.exe`, com os sistemas marcados numa página de chaves e
   argumentos próprios (padrão `{rom}`).
4. **Argumentos** (EMU-004): modelos com `{rom}`, `{romdir}`, `{romname}`, `{core}`, `{config}` e `{fullscreen}`.
   Caminhos entram entre aspas sozinhos (o usuário escreve `{rom}`, não `"{rom}"`); valor ausente some com o espaço;
   marcador desconhecido impede salvar. Camadas: argumentos do jogo (substituem) › do emulador (substituem os da tabela) ›
   da tabela; os extras do sistema somam no fim; os extras do jogo da UX-007 somam depois de tudo. O editor é uma
   sobreposição feita para o controle: os marcadores entram com A, "Colar", "Padrão", "Limpar" e uma prévia da linha de
   comando com uma ROM de exemplo.
5. **Escolha do emulador** (`EmulatorResolver`, EMU-005): a do jogo, senão a do sistema, senão a primeira opção
   sugerida (preferência da tabela, personalizados, e no RetroArch os cores sugeridos primeiro, os que aceitam a
   extensão antes). Escolha que sumiu (emulador desinstalado, core removido) cai para outro core do mesmo emulador ou
   para a sugestão. RetroArch abre com `-L "<core>" "<rom>" -f`; o `.cfg` por jogo entra com
   `--appendconfig "<cfg>"`. Sem nenhuma opção, a mensagem diz o que instalar ("Instale Flycast, Redream ou RetroArch
   com o core Flycast…") e leva a Configurações › Emulação.
6. **Pastas de ROMs como fonte "Emulação"** (`EmulationLibrarySource`, `GameStore.Emulation`, EMU-003): pasta de um
   sistema, ou raiz com subpastas reconhecidas (inclusive `roms\<sistema>`), até 3 níveis, ignorando `media`, `bios`,
   `images`… Extensões do sistema, `.m3u` em todos, `.zip/.7z` só em cartucho e arcade. Multi-disco: `.m3u` esconde os
   discos que lista; `.cue` e `.gdi` escondem as trilhas; "(Disc N)" soltos na mesma pasta viram um jogo com N discos (o
   primeiro abre). Id estável `emu:<sistema>:<sha256 curto do caminho>`. Liga/desliga como as lojas (`DisabledStores`);
   o aviso diz "Nenhuma pasta de ROMs" ou "Sem emulador para PS2 e Dreamcast". Leitura incremental: cada pasta guarda a
   lista de arquivos com a data dela (`cache\rom-scan.json`); pasta sem mudança não é listada de novo.
7. **Inicialização e retorno** (EMU-008): a ROM passa pela sessão de sempre (`StartRealSessionAsync`): tela de jogo,
   TV, áudio, Central de Controle, contador e limite de FPS, registro de tempo e volta com foco. Só muda o que é
   iniciado (o emulador com a linha de comando montada) e o acompanhamento (o processo do emulador e filhos; sem
   processo, a pasta do emulador). O botão dos detalhes vira "Jogar", e os detalhes mostram "Emulador: RetroArch ·
   Snes9x". As ROMs do "Adicionar jogo" (ADR-0022) usam o mesmo caminho.
8. **Sair do emulador pelo controle** (EMU-006): com um emulador aberto pelo Tako e o Tako atrás, segurar **View +
   Menu** (Select + Start; Create + Options no DualSense) por meio segundo, lido pelo mesmo laço do atalho da Central.
   Fechamento gracioso (`EmulatorExitSequence`, testado): WM_CLOSE na janela principal → depois de 3 s, a tecla de sair
   do emulador (Esc, duas vezes no RetroArch) → depois de mais 3 s, o Tako vem para a frente e pergunta "… não fechou"
   com "Encerrar o emulador" (à força, perde o que não foi salvo) e "Voltar ao jogo" (padrão). Liga/desliga em
   Configurações › Emulação. No RetroArch, "Atalhos do RetroArch" também configura o próprio RetroArch
   (`input_quit_gamepad_combo = "4"`, Start + Select segurados, e `quit_press_twice = "false"`) **sem sobrescrever** o que
   o usuário mudou: só troca chave ausente ou com o valor padrão, mantém o resto do arquivo igual e guarda
   `retroarch.cfg.tako-backup` antes da primeira mudança. Perfis de controle completos por emulador (remapear botões)
   ficam para depois: cada emulador tem o próprio formato e o padrão deles já funciona com controles Xbox/PlayStation.
9. **BIOS** (nunca baixadas): RetroArch pelo `.info` do core (arquivos obrigatórios na pasta `system`, ou na
   `system_directory` do `retroarch.cfg`); standalone pela tabela (DuckStation e PCSX2 `bios`, firmware do RPCS3,
   `prod.keys` do Ryujinx e dos derivados do yuzu). Faltando, o Tako avisa antes de abrir, com a pasta exata, e deixa
   "Abrir mesmo assim"; Configurações › Emulação › BIOS lista tudo.
10. **Capas, metadados e sistemas** (EMU-007): nada novo no agente — as ROMs já têm `Game.Platform`, então ScreenScraper
    → IGDB → Steam seguem a ADR-0022. Na biblioteca, a fonte "Emulação" aparece na barra lateral e o painel de filtros
    ganha a seção "Sistema" (SNES, PlayStation…) quando há jogos de mais de um sistema (`LibraryCriteria.Systems`).
11. **Configurações › Emulação** (categoria própria, ADR-0020): "Configurar emulação" (assistente: emuladores → pastas →
    sistemas → pronto, reaproveitando a tela do assistente da TV), mostrar jogos de emulador, pastas de ROMs, procurar
    de novo, emuladores (página por emulador: executável, argumentos, sistemas; personalizados com sistemas e remover),
    emulador de cada sistema, BIOS, tela cheia, sair pelo controle e atalhos do RetroArch. As páginas usam o mesmo
    detalhe em linhas do Modo Console (`SettingsDetailViewModel`, agora com título e item da barra lateral
    configuráveis). Tudo fica em `emulation.json` na pasta de dados.

## Consequências

- O Tako abre ROMs pelos emuladores que o usuário já tem, sem baixar emuladores, BIOS ou jogos, e volta à biblioteca
  com o foco no jogo. Emulador novo é uma entrada no JSON.
- Argumentos e caminhos da tabela foram escritos pela documentação de cada emulador e precisam de validação real
  (checklist de hardware, seção 25): em especial `-batch`/`-nogui` do PCSX2 e DuckStation, `-dvd_path` do xemu, o
  `-config window:fullscreen=yes` do Flycast, o Vita3K (jogos instalados nele) e o `input_quit_gamepad_combo = "4"` do
  RetroArch.
- A tecla de sair vai por `PostMessage` para a janela principal; emuladores que leem o teclado só pela entrada bruta
  podem ignorar — por isso a confirmação de encerrar no fim.
- `GameStore` ganhou `Emulation`; quem faz `switch` em lojas precisa tratar o valor novo (os existentes caem no padrão).
- Jogos de PS3 instalados no RPCS3 (pastas `PS3_GAME`) e o 3DS ficam de fora por enquanto.
