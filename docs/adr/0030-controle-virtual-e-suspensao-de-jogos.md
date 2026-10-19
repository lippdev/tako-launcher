# ADR-0030: Controle virtual nos jogos (INPUT-002) e suspensão de processos para retomada rápida (SESSION-001)

- Status: Proposta
- Data: 2026-10-18

## Contexto

Dois itens do PRD tocam diretamente na forma como o supervisor do Tako interage com o processo do jogo enquanto ele está em execução (ADR-0026):

1. **INPUT-002:** remapear controles para os jogos com perfis por jogo, zonas mortas, curvas de resposta de analógicos, gatilhos e emulação de teclado/mouse, de forma análoga à Entrada Steam.
2. **SESSION-001:** retomada rápida (estilo Quick Resume do Xbox Series), permitindo suspender um jogo em execução na memória sem fechá-lo para abrir outro, e retomar exatamente de onde parou.

Ambos foram objeto de levantamento técnico no spike `docs/spikes/SESSION-001-INPUT-002.md`. Este documento consolida a arquitetura técnica, as restrições por sistema operacional e as decisões de segurança e produto necessárias antes de qualquer implementação.

### Contradição explícita com o PRD

O PRD declara textualmente para o item INPUT-002 (§368):
> *"no Windows o ViGEmBus foi descontinuado, então é preciso escolher outro driver"*.

Esta ADR contradiz formalmente essa declaração ao propor a adoção do ViGEmBus como dependência externa opcional recomendada no Windows (enquanto o sucessor oficial anunciado, o VirtualPad, não atinge maturidade). **Caso esta ADR seja aceita pelo mantenedor, o texto correspondente do PRD (§INPUT-002) deverá ser retificado.**

### Restrições fundamentais do projeto

- **Proibição de injeção de código:** o Tako nunca injeta DLLs ou código na memória de processos de jogos (CONTRIBUTING §6.7). Qualquer remapeamento precisa ocorrer por meio de dispositivos virtuais expostos ao sistema operacional.
- **Localização de código FFI e chamadas de baixo nível:** todo código inseguro (`unsafe`), chamadas FFI de plataforma (como `NtSuspendProcess`, `DeviceIoControl` do ViGEmBus) e interações diretas com o kernel ficam **estritamente restritos ao crate `tako-platform`**, com justificativas obrigatórias de segurança via blocos `// SAFETY:`. O `tako-core` contém exclusivamente contratos e lógica pura.
- **Isolamento de privilégios e segurança:** código executado em nível de kernel (drivers) não pode ser embutido sem assinatura válida e infraestrutura de suporte corporativa. Drivers de terceiros devem ser estritamente opcionais e instalados pelo próprio usuário.
- **Proteção contra banimentos:** a suspensão de processos jamais pode ser aplicada a jogos com anti-cheat ou partidas online, sob risco de banimento de contas ou congelamento de drivers de proteção do sistema.

---

## Parte 1: Controle virtual para jogos (INPUT-002)

### Desafios por plataforma

Para que um jogo receba comandos remapeados, ele precisa se comunicar com um controle virtual gerado pelo Tako, enquanto a leitura do controle físico original é isolada para evitar envio duplo de comandos (*double input*).

1. **Windows:**
   - Não existe API padrão de usuário no Windows para criar controladores virtuais XInput ou HID.
   - **ViGEmBus e seus riscos críticos:**
     - O driver de barramento padrão adotado por projetos como DS4Windows foi o **ViGEmBus** (Nefarius). Contudo, o projeto foi **descontinuado e arquivado em 02/11/2023** por questões de marca e manutenção.
     - **Sem correções de segurança:** o driver não recebe mais patches de vulnerabilidade ou atualizações de compatibilidade com novas compilações do Windows.
     - **Risco de bloqueio pelo sistema operacional:** drivers de kernel descontinuados e sem correções correm o risco real de entrar na lista de bloqueio de drivers vulneráveis da Microsoft (*Microsoft Recommended Driver Blocklist* / HVCI - Memory Integrity), o que impediria sua inicialização em versões futuras ou máquinas corporativas com integridade de memória ativada.
     - **Rejeição por anti-cheats:** sistemas de proteção e anti-cheat modernos frequentemente monitoram ou rejeitam controles virtuais gerados por drivers conhecidos (ou bloqueiam a emulação de XInput para combater assistência automatizada e macros).
     - Seu sucessor planejado pelo mesmo autor, **VirtualPad**, ainda não possui lançamento estável disponível.
     - O desenvolvimento de um driver próprio VHF/UMDF exigiria certificado EV (Extended Validation) corporativo anual e atestação WHQL da Microsoft, inviável para os custos e recursos atuais do projeto.
   - **Não depender do crate `vigem-client`:**
     - O crate `vigem-client` no crates.io encontra-se estagnado e sem manutenção desde 2022. O Tako não adicionará essa dependência externa abandonada; a comunicação com o driver do ViGEmBus será implementada diretamente em `tako-platform` via FFI nativo / envio de IOCTLs com `DeviceIoControl` no descritor `\\.\ViGEmBus`.
   - **Isolamento com HidHide:**
     - O isolamento do controle físico no Windows para evitar envio duplo (*double input*) depende do driver de filtro **HidHide** (já homologado no INPUT-005 / PR #97). O Tako solicita ao HidHide que oculte o controle real dos jogos, permitindo que apenas o Tako o leia e injete no controle virtual do ViGEmBus.
   - **Fallback para teclado e mouse via `SendInput`:**
     - Para jogos que não suportam controle (mapeando botões e analógicos para ações de teclado e mouse), utiliza-se `SendInput` da API Win32 nativa (já adotado no `PointerInjector`), sem necessidade de qualquer driver de kernel adicional.
2. **Linux:**
   - O subsistema `uinput` nativo do kernel permite criar controles virtuais (como controle de Xbox 360 ou DualShock) sem necessidade de drivers de terceiros.
   - O controle físico é isolado de forma limpa via chamada `EVIOCGRAB` (tornando o Tako o único leitor do dispositivo de entrada).
   - O Tako já possui infraestrutura de `uinput` com a crate `evdev` em `tako-platform/src/linux/input.rs` para o ponteiro do mouse virtual. Requer apenas permissão em `/dev/uinput` via regra udev (já documentada no empacotamento Linux).
3. **macOS:**
   - A criação de dispositivos virtuais HID exige a extensão de sistema DriverKit com a entitlement privada `com.apple.developer.driverkit.family.hid.virtual.device`, concedida manualmente pela Apple em processo restrito.
   - Sem essa permissão, o macOS não permite emular controle para controle; apenas emulação de teclado e mouse via `CGEvent` (já existente no `PointerInjector`).
4. **Jogos da Steam em qualquer SO:**
   - A Steam já possui seu próprio driver e camada de remapeamento de alto nível (Steam Input). Para títulos da Steam ou adicionados como atalho na Steam, acionar o link `steam://controllerconfig/<appid>` entrega a experiência nativa sem duplicar infraestrutura.

### Impacto no supervisor e consumo de CPU

Hoje o supervisor lê o controle a cada 2 segundos apenas para capturar o atalho da Central (ADR-0026). Ao assumir o papel de remapeador em tempo real, o supervisor precisará de um laço de leitura e despacho de 1 ms a 4 ms (250 Hz a 1000 Hz) para manter latência imperceptível.

Esse laço exigirá medições rigorosas de uso de CPU em hardware real durante o jogo. Não é prudente prometer números a priori; a sobrecarga real do laço de 1 a 4 ms deverá ser comprovada por experimento prático e constar como nova meta de CPU durante a execução de jogos (revisando os limites descritos no PRD §8.1).

---

## Parte 2: Suspensão de processos para retomada rápida (SESSION-001)

### Mecanismos de suspensão por SO

1. **Windows:** chamada de sistema `NtSuspendProcess` / `NtResumeProcess` (disponível via `ntdll.dll`).
   - Fica isolada exclusivamente em `tako-platform` com justificativa `// SAFETY:` documentando a validade do handle da árvore de processos.
   - **Alternativa `SuspendThread` descartada:** a API Win32 documentada `SuspendThread` (com `THREAD_SUSPEND_RESUME`) exige iterar thread por thread via snapshots do Toolhelp32 ou chamadas sucessivas. Isso introduz uma condição de corrida (*race condition*) inerente e perigosa: novas threads criadas concorrentemente pelo jogo durante a varredura não são suspensas, deixando o processo em estado parcialmente ativo e instável. Em contrapartida, `NtSuspendProcess` congela atomicamente todas as threads da árvore no kernel NT.
   - **Alternativa `DebugActiveProcess` descartada:** anexa um depurador ao processo do jogo, o que dispara imediatamente alertas em qualquer mecanismo de integridade.
2. **Linux:** uso do freezer de cgroup v2 (`cgroup.freeze`), aplicável quando o jogo é iniciado em um escopo dedicado via `systemd-run --scope`. Caso contrário, fallback para envio de sinal `SIGSTOP` / `SIGCONT` na árvore de processos (mesmo método adotado pelo plugin *Pause Games* do Decky Loader no Steam Deck).
3. **macOS:** envio de `SIGSTOP` / `SIGCONT` na árvore de processos.

### Riscos críticos levantados no spike e mitigações

1. **Anti-cheat e integridade (risco de banimento de contas):**
   - Softwares como Easy Anti-Cheat, BattlEye, Vanguard e Ricochet monitoram batimentos cardíacos de threads, contadores de tempo e integridade de rede. Suspender o processo do jogo congela as threads do anti-cheat, provocando desconexão imediata ou banimento automático da conta por suspeita de manipulação de memória.
   - **Diferenciação do Denuvo:** o Denuvo Anti-Tamper é um mecanismo de proteção contra cópia / DRM em nível de usuário (*anti-tamper*), e não um anti-cheat com driver de kernel. Ainda assim, pausas bruscas em checagens periódicas de DRM recomendam prudência.
   - **Mitigação e detecção:** identificação automática baseada na presença de arquivos e diretórios conhecidos na instalação do jogo (como `EasyAntiCheat/`, `BattlEye/`, `*_BE.exe`) e pelas tags de metadados da loja (a Steam marca na loja jogos que usam anti-cheat). Jogos com anti-cheat detectado têm a suspensão **terminantemente desabilitada**.
2. **Jogos online e multiplayer:**
   - Jogos com conexão ativa de rede perderão pacotes e sofrerão timeout do servidor após poucos segundos de congelamento, corrompendo a sessão.
   - **Mitigação:** exclusão de títulos classificados como online/multiplayer e opção manual nas propriedades do jogo para bloquear a suspensão.
3. **Áudio WASAPI exclusivo preso:**
   - No Windows, se o jogo estiver utilizando a interface de áudio em modo WASAPI exclusivo, a suspensão do processo não libera o dispositivo, mantendo o canal de som travado e impedindo que o novo jogo ou o próprio Tako reproduzam áudio.
4. **Salto de relógio (*clock jump* / *time delta*):**
   - Ao retomar um jogo congelado após minutos ou horas, os timers do motor (`QueryPerformanceCounter`, `gettimeofday`) detectam um salto gigantesco de tempo. Embora motores modernos limitem o passo máximo por quadro (*frame delta clamping*), alguns jogos acumulam timers internos, executam rajadas descontroladas de lógicas ou tratam o salto temporal como travamento fatal.
5. **Tela cheia exclusiva DX9/DX11 antigos:**
   - Jogos legados renderizados em tela cheia exclusiva (*exclusive fullscreen*) podem falhar na recuperação do dispositivo Direct3D ao serem retomados caso outro jogo tenha alterado o modo de exibição da tela. O risco é substancialmente menor em jogos modernos em modo janela sem borda ou *flip model* (DirectX 12 e Vulkan).
6. **Controle físico aberto:**
   - O processo do jogo suspenso mantém aberto o descritor/handle para o controle físico original, sem liberar o dispositivo no subsistema de entrada.
7. **Consumo de memória RAM e desalocação de VRAM:**
   - O jogo suspenso permanece alocado na memória RAM.
   - No subsistema gráfico (WDDM 2.0 no Windows e Mesa/DRM no Linux), a VRAM do jogo suspenso pode ser desalocada para a RAM do sistema quando o segundo jogo exigir recursos gráficos. Se a RAM total for insuficiente, o computador sofrerá congelamento geral por *thrashing* de disco.
   - **Mitigação:** limite estrito de **no máximo 1 jogo suspenso** simultaneamente. A suspensão só é autorizada se houver margem de memória livre suficiente (verificação do *working set* do jogo mais margem de segurança de RAM livre).
8. **Recuperação de falhas (Crash Recovery):**
   - Caso o Tako ou o supervisor sofram encerramento abrupto enquanto um jogo estiver congelado, o processo ficaria eternamente suspenso na memória.
   - **Mitigação:** registro de estado em diário persistente no disco (`session_state.json`). Na inicialização seguinte do Tako, o supervisor verifica processos congelados pendentes e dispara o degelo (`resume` / `SIGCONT`) ou finalização do processo órfão.

---

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **A. ViGEmBus opcional (Windows) + uinput (Linux) + Steam Input (Recomendada para INPUT-002)** | Funcional hoje nos sistemas mais usados; sem injeção de DLL; uinput nativo no Linux; atalho para Steam aproveita infraestrutura existente. | Contradiz o PRD original; ViGEmBus está arquivado/sem patches futuros e pode entrar na lista de bloqueio da Microsoft; no macOS fica restrito a teclado/mouse. |
| **B. Aguardar o VirtualPad (Nefarius)** | Arquitetura moderna e suporte oficial futuro do mesmo autor do ViGEmBus. | Projeto ainda sem release pública estável ou data confirmada; bloqueia a entrega do INPUT-002 por tempo indeterminado. |
| **C. Driver próprio de kernel no Windows (VHF/UMDF)** | Independência total de ferramentas de terceiros; controle total da experiência. | Custo financeiro e burocrático inviável (certificado EV anual, testes de certificação WHQL); risco de segurança em caso de bugs no driver. |
| **D. Suspensão em memória desligada por padrão e opt-in por jogo (Recomendada para SESSION-001)** | Retomada instantânea em jogos offline compatíveis; sem escrita pesada em disco; experiência de console no PC; segurança para jogos sensíveis. | Fica na RAM (não sobrevive a reboot); risco de perda de estado se o jogo falhar ao restaurar contexto DirectX/Vulkan ou WASAPI exclusivo. |
| **E. Hibernação completa em disco (Quick Resume total)** | Sobreviveria a reinicializações do sistema. | Inviável no PC sem suporte específico do SO e dos drivers de GPU; salvar 16-32 GB de RAM no SSD levaria dezenas de segundos e degradaria a vida útil do disco; contexto de VRAM é perdido. |
| **F. Não suspender processos** | Zero risco de banimento ou problemas de áudio/gráficos; implementação simples. | Não entrega o diferencial de experiência rápida planejado no PRD §8 (SESSION-001). |

---

## Decisão proposta

1. **INPUT-002 (Controle virtual):**
   - Definir no crate `tako-core` o contrato puro de remapeamento e perfis de controle (`devices::VirtualGamepad`), testável com implementações em memória e sem hardware.
   - **Linux:** implementação nativa via `uinput` + `EVIOCGRAB` para isolamento do dispositivo físico em `tako-platform`.
   - **Windows:** suporte opcional via **ViGEmBus** gerenciado diretamente por FFI / IOCTLs em `tako-platform` (sem depender da crate abandonada `vigem-client`). O Tako não embutirá o driver de kernel, direcionando o usuário para a página oficial do projeto caso deseje ativar remapeamento XInput. Migração planejada para o VirtualPad assim que disponível.
   - **Isolamento e teclado/mouse no Windows:** integração com o driver **HidHide** (INPUT-005) para ocultar o controle físico do jogo e evitar entrada dupla (*double input*); uso de **`SendInput`** nativo para perfis de controle → teclado/mouse sem necessidade de driver.
   - **Jogos da Steam:** priorizar o redirecionamento para `steam://controllerconfig/<appid>` através de ação rápida na interface ("Configurar controle na Steam").
   - **macOS:** suporte a remapeamento apenas para teclado e mouse virtuais via `CGEvent` em `tako-platform`.
2. **SESSION-001 (Retomada rápida):**
   - Contrato puro `devices::ProcessFreezer` no `tako-core` com operações `freeze(pid_tree)`, `thaw(pid_tree)` e rotina de recuperação `recover_after_crash`.
   - Implementação em `tako-platform`:
     - Windows via `NtSuspendProcess` / `NtResumeProcess` com `// SAFETY:`.
     - Linux via freezer de cgroup v2 (`cgroup.freeze`) associado ao `systemd-run --scope`, com fallback para `SIGSTOP`.
     - macOS via `SIGSTOP`.
   - **Recomendação formal:** a retomada rápida deve vir **desligada por padrão** e ser ativada **estritamente por jogo** (*opt-in* nas opções do jogo), garantindo total consentimento do usuário ciente dos riscos de jogos online e modos exclusivos.
   - Limite estrito de **um único jogo congelado** por vez.
   - Verificação mandatória de memória: recusar a suspensão se a memória RAM física disponível for inferior ao tamanho do processo atual somado a uma margem de segurança de RAM livre.
   - **Lista de exclusão obrigatória:** desativação automática para jogos que utilizem anti-cheats conhecidos (detecção por pastas e arquivos locais como `EasyAntiCheat/`, `BattlEye/`, `*_BE.exe` e metadados de loja) e jogos online.
   - Diário de recuperação persistente em disco para evitar processos zumbis suspensos após falhas do supervisor.
3. **Localização de FFI:**
   - Todo e qualquer código FFI (`NtSuspendProcess`, IOCTLs do ViGEmBus, etc.) reside **exclusivamente em `tako-platform`**.

---

## Consequências

- O supervisor precisará gerenciar múltiplos jogos (o jogo ativo em foco e o jogo congelado em segundo plano), mantendo os contextos de sessão isolados.
- No Linux, a suspensão atômica via cgroup v2 exige que o supervisor inicie o processo do jogo em um escopo próprio via `systemd-run --scope`.
- O laço de leitura de controle em tempo real (1 a 4 ms) durante o jogo exigirá medições de CPU em hardware para definir formalmente a nova meta de consumo de CPU do supervisor (PRD §8.1).
- O PRD deverá ser atualizado para autorizar expressamente a convivência temporária com o ViGEmBus como dependência opcional no Windows.
- A interface gráfica precisará de novos estados e ações ("Pausar e abrir outro", "Retomar <jogo>", aviso de WASAPI exclusivo ou tela preta com opção "Forçar encerramento e reiniciar jogo").
- No Windows, usuários sem o ViGEmBus instalado poderão usufruir de remapeamento para teclado/mouse via `SendInput` ou integração Steam, mas não de controle virtual XInput direto.

---

## Perguntas para o mantenedor

1. **Laço de leitura e meta de CPU:** O projeto aceita o laço de leitura de 1 ms a 4 ms no supervisor durante a execução dos jogos e a definição de uma nova meta de uso de CPU a ser validada por experimento em hardware?
2. **Adoção do ViGEmBus e retificação do PRD:** O projeto aceita adotar o ViGEmBus como dependência externa opcional recomendada no Windows (com comunicação direta por IOCTL em `tako-platform`, sem o crate `vigem-client`), atualizando o PRD (§INPUT-002), mesmo ciente dos riscos de o driver ser arquivado e poder vir a entrar na lista de bloqueio da Microsoft?
3. **Prioridade da Entrada Steam:** Para jogos originários da Steam, o Tako deve priorizar o botão "Configurar na Steam" (`steam://controllerconfig/<appid>`) em vez de aplicar o remapeador interno do Tako?
4. **Política de ativação da Retomada Rápida:** Confirma a recomendação de que a retomada rápida (SESSION-001) venha desligada por padrão, sendo estritamente *opt-in* por jogo com aviso de incompatibilidade para jogos online e com áudio exclusivo?
5. **Critério de corte de memória para suspensão:** Uma margem fixa de 4 GB de RAM livre restante é adequada como política padrão para autorizar o congelamento, ou devemos exigir uma porcentagem da memória total do sistema (ex.: 25% livre)?
6. **Recuperação de congelamento na Central:** Caso um jogo apresente tela preta ou perca o dispositivo de áudio após ser descongelado (devido a limitações de motores gráficos legados ou WASAPI exclusivo), o Tako deve oferecer na Central uma opção explícita de "Forçar encerramento e reiniciar jogo"?
