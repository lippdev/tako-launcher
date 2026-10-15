# Architecture Decision Records

ADR-001 a ADR-010 estão propostas em [`../PRD.md` §14](../PRD.md). Ao aceitar uma, crie `NNNN-slug.md` a partir do template.
Dependem de spike: ADR-003 (FSE) e ADR-004 (compat Playnite).
- [ADR-0011](0011-runtime-contracts-naming-and-targets.md): nome e frameworks-alvo do runtime compartilhado (Proposta).
- [ADR-0012](0012-supervisor-e-ipc.md): supervisor da UI no próprio `Tako.exe` e IPC versionado por Named Pipes (Proposta).
- [ADR-0013](0013-estado-de-monitores-e-audio.md): estado de monitores e áudio da sessão, contratos próprios e implementação Windows (Proposta).
- [ADR-0014](0014-licenca-agpl.md): licença AGPL-3.0-only e reuso do Console Mode (Aceita).
- [ADR-0015](0015-monitores-do-console-mode-e-tela-de-jogo.md): monitores vindos do Console Mode e "Tela de jogo" (Proposta).
- [ADR-0016](0016-audio-steam-e-tv-na-sessao.md): saída de áudio, Steam (`RunningAppID` e bandeja) e TV (ADB pela rede) durante a sessão de jogo (Proposta).
- [ADR-0017](0017-contador-de-fps-com-presentmon.md): contador de FPS próprio pelo PresentMon, sem RTSS e sem injeção; permissão pelo grupo "Usuários de log de desempenho" (Proposta).
- [ADR-0018](0018-limite-de-fps-pelo-driver.md): limite de FPS pelo driver de vídeo (NVAPI, ADLX, IGCL), sem RTSS nem elevação (Proposta).
- [ADR-0019](0019-fontes-de-biblioteca.md): fontes de biblioteca (Steam, Epic Games, GOG, Xbox e as próximas lojas) por uma interface comum; jogo em duas lojas fica em itens separados e ligados (Proposta).
- [ADR-0021](0021-metadados-igdb.md): metadados pela IGDB com a chave da Twitch de cada usuário (sem proxy) e combinação por campo entre N provedores pela prioridade do tipo de jogo (Proposta).
- [ADR-0022](0022-metadados-screenscraper.md): ScreenScraper para ROMs, com a conta de cada usuário e as credenciais do software só no build de release (segredos do CI); hash da ROM, mídia sem credenciais no disco e ROM no "Adicionar jogo" sem abrir (Proposta).
