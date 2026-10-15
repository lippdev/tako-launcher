// Ponto de entrada do Launcher.Host: escolha do modo (FSE/Fullscreen) e recuperação. Ver docs/PRD.md §4.1.
// A supervisão da UI (MVP-006) roda dentro do próprio Tako.exe em modo supervisor, para manter um executável único
// (ADR-0012); a lógica fica em Launcher.Core.Supervision e o IPC em ConsoleMode.Runtime.Ipc.
return 0;
