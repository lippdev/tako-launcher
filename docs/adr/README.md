# Architecture Decision Records

ADR-001 a ADR-010 estão propostas em [`../PRD.md` §14](../PRD.md). Ao aceitar uma, crie `NNNN-slug.md` a partir do template.
Dependem de spike: ADR-003 (FSE) e ADR-004 (compat Playnite).
- [ADR-0011](0011-runtime-contracts-naming-and-targets.md): nome e frameworks-alvo do runtime compartilhado (Proposta).
- [ADR-0012](0012-supervisor-e-ipc.md): supervisor da UI no próprio `Tako.exe` e IPC versionado por Named Pipes (Proposta).
- [ADR-0013](0013-estado-de-monitores-e-audio.md): estado de monitores e áudio da sessão, contratos próprios e implementação Windows (Proposta).
- [ADR-0014](0014-licenca-agpl.md): licença AGPL-3.0-only e reuso do Console Mode (Aceita).
- [ADR-0015](0015-monitores-do-console-mode-e-tela-de-jogo.md): monitores vindos do Console Mode e "Tela de jogo" (Proposta).
- [ADR-0016](0016-audio-steam-e-tv-na-sessao.md): saída de áudio, Steam (`RunningAppID` e bandeja) e TV (ADB pela rede) durante a sessão de jogo (Proposta).
