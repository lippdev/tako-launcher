# Desempenho do Tako (PERF-001)

Configurações › Sistema › Desempenho do Tako abre um painel, atualizado a cada segundo enquanto está aberto:

- **Interface**: memória residente (working set no Windows), CPU em % da máquina inteira (a diferença entre
  duas leituras; a primeira mostra "Medindo…") e o tempo até a primeira tela desta abertura, do `main` do
  processo da interface até o primeiro quadro desenhado. Esse tempo também vai para o log (`primeira tela em …`).
- **Supervisor** (quando a interface roda sob ele): memória e CPU do processo que fica aberto durante o jogo.
  No macOS só a memória: os tempos de CPU de outro processo vêm em unidades do relógio da máquina.

Leituras pelo SO, só leitura: `GetProcessMemoryInfo` e `GetProcessTimes` no Windows, `/proc/<pid>` no Linux,
`proc_pidinfo` e `getrusage` no macOS. Processo que não deu para ler mostra "Indisponível".

## Teste manual pendente

- Abrir o painel na versão instalada: memória e CPU da interface e do supervisor mudam a cada segundo; navegar
  na biblioteca com o painel fechado e voltar mostra o pico da CPU passando.
- Comparar com o Gerenciador de Tarefas (Windows) ou `top` (Linux): mesma ordem de grandeza.
- Tempo até a primeira tela: abrir o Tako de novo e comparar com o que se vê; com `--no-supervisor` a seção do
  supervisor some.
- Linux e macOS: o painel abre e mostra números (o código desses SOs só é compilado no CI do GitHub).
