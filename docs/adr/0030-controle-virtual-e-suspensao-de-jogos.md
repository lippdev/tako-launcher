# ADR-0030: Controle virtual nos jogos (INPUT-002) e suspensão de processos para retomada rápida (SESSION-001)

- Status: Proposta
- Data: 2026-10-18

## Contexto

Dois itens do PRD tocam diretamente na forma como o supervisor do Tako interage com o processo do jogo enquanto ele está em execução (ADR-0026):

1. **INPUT-002:** remapear controles para os jogos com perfis por jogo, zonas mortas, curvas de resposta de analógicos, gatilhos e emulação de teclado/mouse, de forma análoga à Entrada Steam.
2. **SESSION-001:** retomada rápida (estilo Quick Resume do Xbox Series), permitindo suspender um jogo em execução na memória sem fechá-lo para abrir outro, e retomar exatamente de onde parou.

Ambos foram objeto de levantamento técnico no spike `docs/spikes/SESSION-001-INPUT-002.md`. Este documento consolida a arquitetura técnica, as restrições por sistema operacional e as decisões de segurança e produto necessárias antes de qualquer implementação.

### Restrições fundamentais do projeto

- **Proibição de injeção de código:** o Tako nunca injeta DLLs ou código na memória de processos de jogos (CONTRIBUTING §6.7). Qualquer remapeamento precisa ocorrer por meio de dispositivos virtuais expostos ao sistema operacional.
- **Isolamento de privilégios e segurança:** código executado em nível de kernel (drivers) não pode ser embutido sem assinatura válida e infraestrutura de suporte. Drivers de terceiros devem ser estritamente opcionais.
- **Proteção contra banimentos:** a suspensão de processos jamais pode ser aplicada a jogos com anti-cheat ou partidas online, sob risco de banimento de contas ou congelamento de drivers de proteção do sistema.

---

## Parte 1: Controle virtual para jogos (INPUT-002)

### Desafios por plataforma

Para que um jogo receba comandos remapeados, ele precisa se comunicar com um controle virtual gerado pelo Tako, enquanto a leitura do controle físico original é isolada (para evitar envio duplo de comandos).

1. **Windows:**
   - Não existe API padrão de usuário para criar dispositivos virtuais XInput ou HID de controle.
   - O driver padrão da comunidade para essa finalidade foi o **ViGEmBus** (Nefarius). Entretanto, o projeto foi **descontinuado e arquivado em 02/11/2023**. O driver continua assinado, estável e funcional no Windows 10 e 11, mas não recebe novas atualizações. Seu sucessor anunciado, **VirtualPad**, ainda não possui data de lançamento estável.
   - O desenvolvimento de um driver próprio VHF/UMDF exigiria certificado EV (Extended Validation) corporativo e atestação WHQL da Microsoft, inviável para o escopo e orçamento atuais do projeto.
   - O isolamento do controle físico depende do driver de filtro **HidHide** (já tratado no INPUT-005 / PR #97), que oculta o dispositivo físico dos jogos mas permite que o Tako continue lendo-o.
2. **Linux:**
   - O subsistema `uinput` nativo do kernel permite criar controles virtuais (como controle de Xbox 360 ou DualShock) sem necessidade de drivers adicionais.
   - O controle físico pode ser isolado de forma limpa via chamada `EVIOCGRAB` (tornando o Tako o único leitor do dispositivo de entrada).
   - O Tako já possui infraestrutura de `uinput` com a crate `evdev` em `tako-platform/src/linux/input.rs` para o ponteiro do mouse virtual. Requer apenas permissão em `/dev/uinput` via regra udev (já documentada no empacotamento Linux).
3. **macOS:**
   - A criação de dispositivos virtuais HID exige a extensão de sistema DriverKit com a entitlement privada `com.apple.developer.driverkit.family.hid.virtual.device`, concedida manualmente pela Apple em processo restrito.
   - Sem essa permissão, o macOS não permite emular controle para controle; apenas emulação de teclado e mouse via `CGEvent` (já existente no `PointerInjector`).
4. **Jogos da Steam em qualquer SO:**
   - A Steam já possui seu próprio driver e camada de remapeamento de alto nível (Steam Input). Para títulos da Steam ou adicionados como atalho na Steam, abrir `steam://controllerconfig/<appid>` entrega a experiência nativa sem duplicar infraestrutura.

### Impacto no supervisor e CPU

Hoje o supervisor lê o controle a cada 2 segundos apenas para capturar o atalho da Central (ADR-0026). Ao assumir o papel de remapeador em tempo real, o supervisor precisará de um laço de leitura e despacho de 1 ms a 4 ms (250 Hz a 1000 Hz) para manter latência imperceptível, demandando controle rigoroso de uso de CPU durante o jogo (PRD §8.1).

---

## Parte 2: Suspensão de processos para retomada rápida (SESSION-001)

### Mecanismos de suspensão por SO

1. **Windows:** chamada de sistema não documentada `NtSuspendProcess` / `NtResumeProcess` (disponível via `ntdll.dll`, amplamente utilizada por ferramentas de sistema como `PsSuspend` do Sysinternals e Resource Monitor). Opera sobre toda a árvore de processos do jogo.
2. **Linux:** uso do freezer de cgroup v2 (`cgroup.freeze`), aplicável se o jogo for iniciado em um escopo dedicado via `systemd-run --scope`. Caso contrário, envio de sinal `SIGSTOP` / `SIGCONT` na árvore de processos (mesmo método adotado pelo plugin *Pause Games* do Decky Loader no Steam Deck).
3. **macOS:** envio de `SIGSTOP` / `SIGCONT` na árvore de processos.

### Riscos críticos e mitigações

1. **Anti-cheat (risco de banimento):**
   - Softwares como Easy Anti-Cheat, BattlEye, Vanguard, Ricochet e Denuvo monitoram a cadência de threads, pulso de rede e batimentos de integridade. Suspender o processo do jogo congela as threads do anti-cheat, provocando desconexão imediata, tela azul (BSOD) em drivers de kernel ou banimento automático da conta por suspeita de manipulação de memória.
   - **Mitigação:** identificação automática de executáveis, serviços e pastas de anti-cheat conhecidos. Jogos identificados com anti-cheat têm a função de congelamento **terminantemente desabilitada**.
2. **Jogos online e multiplayer:**
   - Qualquer jogo com conexão TCP/UDP ativa perderá pacotes e sofrerá timeout do servidor após poucos segundos de congelamento, corrompendo a sessão de jogo.
   - **Mitigação:** exclusão de jogos tagueados como online/multiplayer e opção manual nas propriedades do jogo para desativar a suspensão.
3. **Consumo de memória RAM e VRAM:**
   - Ao manter o jogo suspenso na memória enquanto outro jogo é aberto, a memória RAM ocupada permanece alocada.
   - No subsistema de vídeo (WDDM 2.0 no Windows e drivers Linux/Mesa), a VRAM do processo suspenso pode ser desalocada para a RAM do sistema quando o segundo jogo exigir recursos gráficos. Se a RAM total for insuficiente, o sistema entrará em *thrashing* (troca excessiva com disco), causando congelamento geral do computador.
   - **Mitigação:** limite rígido de **no máximo 1 jogo suspenso** simultaneamente. Antes de autorizar a suspensão para abrir um novo jogo, o Tako deve aferir a memória RAM livre do sistema e o *working set* do jogo atual, recusando a suspensão se a margem de segurança for inferior a 4 GB livres.
4. **Recuperação de falhas (Crash Recovery):**
   - Se o Tako ou o supervisor sofrerem um encerramento anormal enquanto um jogo estiver congelado, o processo ficará travado na memória consumindo recursos sem interface para retomada.
   - **Mitigação:** diário de estado persistido em disco (`session_state.json`). Na inicialização do Tako, o supervisor verifica se há processos congelados registrados e executa o degelo (`resume`/`SIGCONT`) ou finalização do processo órfão.

---

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **A. ViGEmBus opcional (Windows) + uinput (Linux) + Steam Input (Recomendada para INPUT-002)** | Funcional hoje nos sistemas mais usados; sem injeção de DLL; uinput nativo no Linux; atalho para Steam aproveita infraestrutura existente. | ViGEmBus está arquivado/descontinuado (sem patches futuros); requer instalação de driver externo no Windows; no macOS fica restrito a teclado/mouse. |
| **B. Aguardar o VirtualPad (Nefarius)** | Arquitetura moderna e suporte oficial futuro do mesmo autor do ViGEmBus. | Projeto ainda sem release pública estável ou data confirmada; bloqueia a entrega do INPUT-002 por tempo indeterminado. |
| **C. Driver próprio de kernel no Windows (VHF/UMDF)** | Independência total de ferramentas de terceiros; controle total da experiência. | Custo financeiro e burocrático alto (certificado EV anual, testes de certificação WHQL); risco de segurança em caso de bugs no driver. |
| **D. Suspensão em memória com guarda de RAM e anti-cheat (Recomendada para SESSION-001)** | Retomada instantânea em jogos offline compatíveis; sem escrita pesada em disco; experiência de console no PC. | Fica na RAM (não sobrevive a reboot); risco de perda de estado se o jogo falhar ao restaurar contexto gráfico DirectX/Vulkan. |
| **E. Hibernação completa em disco (Quick Resume total)** | Sobreviveria a reinicializações do sistema. | Inviável no PC sem suporte específico do SO e dos drivers de GPU; salvar 16-32 GB de RAM no SSD levaria dezenas de segundos e degradaria a vida útil do disco; contexto de VRAM é perdido. |
| **F. Não suspender processos** | Zero risco de banimento ou problemas de memória; implementação simples. | Não entrega o diferencial de experiência rápida planejado no PRD §8 (SESSION-001). |

---

## Decisão proposta

1. **INPUT-002 (Controle virtual):**
   - Definir no crate `tako-core` o contrato puro de remapeamento e perfis de controle (`devices::VirtualGamepad`), testável com implementações em memória e sem hardware.
   - **Linux:** implementação nativa via `uinput` + `EVIOCGRAB` para isolamento do dispositivo físico.
   - **Windows:** suporte opcional via **ViGEmBus** (com detecção de presença do driver). O Tako não embutirá nem forçará a instalação do driver de kernel, direcionando o usuário para a página oficial do projeto caso deseje ativar remapeamento virtual XInput. Migração planejada para o VirtualPad quando este atingir maturidade estável.
   - **Jogos da Steam:** priorizar o redirecionamento para `steam://controllerconfig/<appid>` através de ação rápida na interface ("Configurar controle na Steam").
   - **macOS:** suporte a remapeamento apenas para teclado e mouse virtuais via `CGEvent`.
2. **SESSION-001 (Retomada rápida):**
   - Contrato `devices::ProcessFreezer` no `tako-core` com operações `freeze(pid_tree)`, `thaw(pid_tree)` e rotina de recuperação `recover_after_crash`.
   - Limite estrito de **um único jogo congelado** por vez.
   - Verificação mandatória de memória: recusar a suspensão se a memória RAM física disponível for inferior ao tamanho do processo atual somado a uma margem de segurança de 4 GB.
   - **Lista de exclusão obrigatória:** desativação automática para jogos que utilizem anti-cheats conhecidos (EAC, BattlEye, Vanguard, Ricochet, PunkBuster, Denuvo Anti-Tamper) e jogos com funcionalidade multijogador online.
   - Opção granular nas configurações por jogo: o usuário pode forçar a desativação da suspensão para títulos específicos.
   - Diário de recuperação persistente em disco para evitar processos zumbis suspensos após falhas do supervisor.

---

## Consequências

- O supervisor precisará ser adaptado para gerenciar múltiplos jogos (o jogo em foco e o jogo congelado em segundo plano), mantendo os contextos de sessão isolados.
- O laço de leitura de controle em tempo real (1-4 ms) durante o jogo exigirá otimização no `tako-platform` para garantir que o consumo de CPU permaneça insignificante (< 0.5% em CPUs modernas).
- A interface gráfica precisará de novos estados e ações ("Pausar e abrir outro", "Retomar <jogo>", avisos de memória insuficiente e avisos de jogo incompatível com suspensão).
- No Windows, usuários sem o ViGEmBus instalado poderão usufruir de remapeamento para teclado/mouse ou integração Steam, mas não de controle virtual XInput direto.

---

## Perguntas para o mantenedor

1. **Driver ViGEmBus no Windows:** O projeto aceita adotar o ViGEmBus como dependência externa opcional recomendada no Windows (com link para download oficial no site do autor), com compromisso de migrar para o VirtualPad quando disponível, ou prefere adiar o remapeamento controle→controle no Windows até que o VirtualPad seja lançado?
2. **Prioridade da Entrada Steam:** Para jogos originários da Steam, o Tako deve priorizar o botão "Configurar na Steam" (`steam://controllerconfig/<appid>`) em vez de aplicar o remapeador interno do Tako?
3. **Ativação da Retomada Rápida:** A retomada rápida (SESSION-001) deve vir habilitada por padrão para todos os jogos offline compatíveis (com aviso no primeiro uso), ou deve ser estritamente *opt-in* (o usuário ativa nas opções de cada jogo)?
4. **Critério de corte de memória para suspensão:** Uma margem fixa de 4 GB de RAM livre restante é adequada como política padrão para autorizar o congelamento, ou devemos exigir uma porcentagem da memória total do sistema (ex.: 25% livre)?
5. **Comportamento em caso de jogo travado ao retomar:** Caso um jogo apresente tela preta ou perca o dispositivo de áudio após ser descongelado (limitação de alguns motores gráficos), o Tako deve oferecer na Central uma opção explícita de "Forçar encerramento e reiniciar jogo"?
