# Estudo SESSION-001 + INPUT-002: retomada rápida e remapeamento de controles para os jogos

- Status: estudo concluído (sem código). Os experimentos de hardware estão no fim de cada parte.
- Data: 2026-10-17
- Itens: SESSION-001 (retomada rápida, "como o Quick Resume do Xbox Series") e INPUT-002 (remapear controles para
  os jogos, "como a Entrada Steam"). Os dois começam por um estudo (PRD).
- Ideias relacionadas: IDEA-INPUT-03 (perfis por jogo), 04 (giroscópio), 06 (macros), 08 (perfis da comunidade).

Os dois itens mexem no mesmo lugar: o que o Tako faz com o jogo **enquanto ele roda**, hoje responsabilidade do
supervisor (ADR-0026). Por isso estão num documento só.

---

## Parte 1: SESSION-001, retomada rápida

### Pergunta

O Tako consegue deixar um jogo "em pausa" sem fechar, abrir outro e voltar ao primeiro exatamente onde estava, nos
três SOs, sem quebrar o jogo nem o PC?

### Resumo

1. **No console, o Quick Resume é do sistema, não do jogo.** No Xbox Series, cada jogo roda numa partição
   virtualizada, e o console grava o estado dela no SSD (por isso sobrevive a desligar). No PC não existe nada
   equivalente para jogos Win32: o PRD já limita o item a "fica na memória, não sobrevive a reiniciar".
2. **Congelar o processo é possível nos três SOs, sem driver nem administrador:**
   - Windows: `NtSuspendProcess`/`NtResumeProcess` (ntdll, não documentadas, mas estáveis desde o XP e usadas por
     ferramentas como o PsSuspend da Sysinternals, mantido pela Microsoft). A alternativa documentada é suspender
     thread a thread (`SuspendThread` com `THREAD_SUSPEND_RESUME`), que tem corrida com threads novas.
     `DebugActiveProcess` também congela, mas anexa um depurador: é exatamente o que o anti-cheat procura.
   - Linux: `SIGSTOP`/`SIGCONT` na árvore do jogo, ou o *freezer* do cgroup v2 (`cgroup.freeze`), que pega a árvore
     inteira de uma vez, inclusive os processos do Wine/Proton. Há precedente direto: o plugin **Pause Games** do
     Decky Loader no Steam Deck (`SDH-PauseGames`) manda `SIGSTOP` para os filhos do *reaper* da Steam quando o
     jogo perde o foco, e `SIGCONT` na volta.
   - macOS: `SIGSTOP`/`SIGCONT` (mesma ideia; o macOS não tem cgroup).
3. **O que o congelamento não resolve:**
   - **Memória:** o jogo continua inteiro na RAM, e a VRAM fica reservada. No Windows (WDDM 2.x) o sistema pode
     tirar da VRAM as alocações de um processo parado quando outro precisa (o contexto parado não roda, e o VidMm
     move a memória), mas a RAM do sistema precisa aguentar os dois jogos mais o que foi tirado da VRAM.
   - **Rede:** jogo online cai (as conexões vencem o tempo). O PRD já tira esses jogos do item.
   - **Anti-cheat:** EAC, BattlEye, Vanguard e afins podem ver threads paradas, tempos impossíveis ou o próprio
     acesso ao processo como adulteração. O PRD tira esses jogos do item, e a regra §6.7 (não mexer em anti-cheat)
     vale.
   - **Áudio e entrada:** o dispositivo de áudio aberto em modo exclusivo (WASAPI exclusivo) fica preso; o
     controle segue aberto pelo jogo parado (sem efeito, mas sem soltar). O plugin do Steam Deck relata estalos de
     áudio que a pausa *resolve* antes de suspender o sistema, e jogos que "não gostam" de muitas pausas longas.
   - **Tempo:** ao voltar, o jogo vê um salto enorme no relógio. A maioria limita o passo do quadro; alguns
     disparam timers acumulados ou acham que travaram.
   - **Tela:** um jogo em tela cheia exclusiva (DX9/DX11 antigos) parado com a tela de outro jogo por cima pode
     não recuperar a troca de modo. Em janela sem borda ou no modo *flip* (DX12, Vulkan), o risco cai muito.
4. **Na arquitetura do Tako:** o supervisor acompanha **um** jogo (`Supervisor::running: Option<Running>`), com a
   tela e o áudio da sessão guardados num diário (`SessionEnvironmentGuard`), e restaura tudo no fim. Retomada
   rápida pede mais:
   - **vários jogos acompanhados**, um ativo e os outros congelados;
   - o **ambiente de cada sessão**: o congelado guarda a tela e o áudio dele, e o ativo aplica os seus;
   - **restaurar com segurança na queda:** se o supervisor cair, um jogo congelado fica parado para sempre. O
     diário precisa listar os congelados, e a próxima execução descongela (como o cursor e o HidHide já fazem).

### Recomendação

Fazer, mas como recurso **opcional e por jogo**, ligado só quando o jogo passa por uma lista de segurança:

- **Fica de fora sozinho:** jogos com anti-cheat conhecido. Detectar pelos arquivos na pasta do jogo
  (`EasyAntiCheat/`, `BattlEye/`, `*_BE.exe`) e pelos dados das lojas quando elas informam (a Steam marca jogos
  com anti-cheat na página da loja).
- **O usuário escolhe por jogo:** "Permitir retomada rápida", desligada por padrão nas primeiras versões, com o
  aviso de que jogos online caem.
- **Mecanismo por SO:** Windows `NtSuspendProcess` na árvore do jogo (a mesma que o acompanhamento já conhece);
  Linux `cgroup.freeze` quando o jogo roda num cgroup próprio (systemd-run `--scope`), senão `SIGSTOP` na
  árvore; macOS `SIGSTOP`.
- **Limite:** um jogo congelado por vez, e só com RAM livre suficiente. Medir o *working set* do jogo e recusar se
  o novo não couber, para não levar o PC a trocar memória com o disco.

### Fatias (depois deste estudo)

1. **Contrato no core** `devices::ProcessFreezer` (`freeze(pid_tree)`, `thaw`, `recover_after_crash`), com
   versão falsa, e diário dos congelados. Testes de queda: congelado + supervisor morto → a próxima execução
   descongela.
2. **Supervisor com dois jogos:** o ativo e um congelado, cada um com o ambiente de sessão dele. A sessão de hoje
   continua igual quando o recurso está desligado (o caso padrão).
3. **Interface:** "Pausar e abrir outro" na Central e "Voltar a <jogo>" no Início. Precisa de design (a rodada
   atual é sem design novo).
4. **Detecção de anti-cheat e opção por jogo** em Opções do jogo.

### Experimentos de hardware pendentes

Num PC com dois jogos offline (um DX11 em tela cheia e um DX12/Vulkan em janela sem borda):

- Congelar com o PsSuspend (`pssuspend <pid>`), abrir outro jogo, jogar 10 min, fechar e retomar (`pssuspend -r`).
  Anotar: a imagem volta? O áudio volta? O controle responde? A tela muda de modo direito?
- Repetir com 30 min e com o segundo jogo usando mais VRAM que a placa tem (para ver a VRAM ir para a RAM).
- No Linux/Steam Deck: o mesmo com `kill -STOP`/`-CONT` num jogo pelo Proton, e com `cgroup.freeze`.

---

## Parte 2: INPUT-002, remapear controles para os jogos

### Pergunta

O Tako consegue entregar ao jogo um controle remapeado (botões, analógicos, gatilhos, giroscópio e mouse/teclado),
com perfis por jogo, como a Entrada Steam, nos três SOs?

### Resumo

1. **Remapear para o jogo exige um controle virtual:** o jogo precisa ver um dispositivo novo, o "controle do
   Tako", e não o controle de verdade. Ler o controle e reescrever as entradas no meio do caminho não existe sem
   driver. Injetar no jogo é proibido (§6.7).
2. **Windows:** não há API de usuário para criar um controle XInput ou HID virtual. As opções:
   - **ViGEmBus**: o driver de barramento usado pelo DS4Windows e afins. Foi **aposentado e arquivado em
     02/11/2023** por conflito de marca. Continua funcionando e instalável (driver assinado), mas não recebe
     correções. O autor anunciou um sucessor, **VirtualPad**, ainda sem data.
   - **Driver próprio** (VHF/UMDF): exige certificado EV e assinatura de atestação da Microsoft. Não é viável
     para o projeto agora.
   - **A Entrada Steam do próprio Steam:** se o jogo abre pela Steam (ou como "jogo fora da Steam" nela), a Steam
     faz o remapeamento com o driver dela. O Tako pode **abrir a configuração da Steam** para o jogo
     (`steam://controllerconfig/<appid>`) em vez de reinventar.
   - **Só teclado e mouse:** `SendInput` não precisa de driver (o Tako já usa no "ponteiro pelo controle",
     `PointerInjector`). Cobre "controle → teclado/mouse" para jogos sem suporte a controle, mas não
     "controle → controle".
3. **Linux:** `uinput` cria um controle virtual sem driver novo (o Tako já cria um mouse virtual em
   `tako-platform/src/linux/input.rs` com a crate `evdev`). O controle de verdade precisa ser "escondido" do jogo
   com `EVIOCGRAB` (o Tako passa a ser o único leitor), senão o jogo recebe as duas entradas. Exige permissão em
   `/dev/uinput` (regra udev do pacote; o checklist de Linux já cobre isso para o ponteiro).
4. **macOS:** um HID virtual exige DriverKit com a *entitlement* `com.apple.developer.driverkit.family.hid.virtual.device`,
   que a Apple concede caso a caso. Sem isso, só teclado e mouse (`CGEvent`, já usado no ponteiro).
5. **Esconder o controle de verdade:** no Windows, o HidHide (INPUT-005, PR #97) é o mesmo caminho dos
   remapeadores da comunidade: esconde o real e deixa o virtual. Tem a mesma limitação dos controles Xbox por
   XInput, que podem continuar visíveis.
6. **O motor de remapeamento é código puro:** leitura do controle, perfil e saída do controle virtual. Ele cabe
   no `tako-core` (como `input::controller_mouse`), testável sem hardware. Os perfis por controle do INPUT-001
   (PR #91) já trazem a ideia de perfil por GUID. O INPUT-002 acrescenta o perfil por jogo, os analógicos (curvas,
   zonas mortas: IDEA-INPUT-09), os gatilhos e as saídas para teclado e mouse.
7. **Macros e anti-cheat:** controle virtual em si é comum e aceito, porque a Entrada Steam faz isso. **Macros**
   (um botão gerando sequências com tempo) entram em zona de risco em jogos competitivos, e a IDEA-INPUT-06 já
   pede "respeitando limites". Fora do primeiro corte.

### Recomendação

Dividir pelo que cada SO permite, sem prometer o mesmo nos três:

| | Controle → controle | Controle → teclado/mouse | Esconder o real |
|---|---|---|---|
| Windows | ViGEmBus opcional, oferecido como o HidHide (página oficial; o Tako não instala). Trocar pelo VirtualPad quando sair. | `SendInput` (sem driver) | HidHide (INPUT-005) |
| Linux | `uinput` (sem driver novo) | `uinput` (teclado/mouse virtuais) | `EVIOCGRAB` |
| macOS | Não (DriverKit com *entitlement* da Apple) | `CGEvent` | Não |

E em qualquer SO, para jogos da Steam: botão "Configurar controle na Steam" que abre `steam://controllerconfig/<appid>`.
É a saída de menor custo e a que o usuário já conhece.

**Precisa de ADR antes de codar:**

- o contrato `devices::VirtualGamepad` (criar, enviar estado, destruir, com recuperação após queda: um controle
  virtual largado não pode ficar "segurando" um botão);
- usar um driver aposentado (ViGEmBus) como opcional;
- quem é dono do controle durante o jogo. Hoje o supervisor lê o controle só para o atalho da Central. Com
  remapeamento, ele vira o único leitor e precisa de um laço de 1 ms a 4 ms, em vez da espera por eventos
  de hoje (2 s), o que muda a meta de CPU durante o jogo (PRD §8.1).

### Fatias (depois da ADR)

1. **Motor puro** no core: perfil (botões, eixos com zona morta e curva, gatilhos como botão ou eixo, saídas de
   teclado e mouse) e a função `estado do controle → saída`, com testes. Perfil por jogo nas Opções do jogo,
   reaproveitando a tela de botões do INPUT-001.
2. **Teclado e mouse** nos três SOs (sem driver): reaproveita o `PointerInjector` e acrescenta teclas.
3. **Linux:** controle virtual por `uinput` + `EVIOCGRAB`.
4. **Windows:** ViGEmBus opcional + HidHide.
5. **Steam:** botão "Configurar controle na Steam" para jogos da Steam (independe dos outros, pode vir primeiro).

### Experimentos de hardware pendentes

- Windows com ViGEmBus e HidHide instalados: criar um Xbox 360 virtual (com a ferramenta de teste do ViGEm),
  esconder o real e abrir um jogo XInput. Anotar se o jogo vê só o virtual e a latência percebida.
- Linux: um controle `uinput` com `EVIOCGRAB` no real, num jogo pelo Proton e num nativo (SDL). Conferir se o
  Proton expõe o virtual como XInput.
- Medir a CPU do supervisor com o laço de 4 ms durante o jogo (meta do PRD: memória total ≤ 40 MB; a CPU entra na
  ADR).

---

## Fontes

- ViGEmBus aposentado: <https://docs.nefarius.at/projects/ViGEm/End-of-Life/> e <https://github.com/nefarius/ViGEmBus>
- HidHide (API e limitações): <https://docs.nefarius.at/projects/HidHide/API-Documentation/>
- Pause Games do Decky Loader (`SIGSTOP`/`SIGCONT` no Steam Deck): <https://github.com/wynn1212/SDH-PauseGames>
- Suspender processos no Windows (`NtSuspendProcess`, PsSuspend): <https://j00ru.vexillium.org/2009/08/suspending-processes-in-windows/>
  e <https://scivision.dev/windows-suspend-program>
- Residência de memória da GPU no WDDM 2.0: <https://learn.microsoft.com/en-us/windows-hardware/drivers/display/wddm-2-0-and-windows-10>
