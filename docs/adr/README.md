# Architecture Decision Records

Ordem de verdade (PRD v0.2 §0): o PRD, depois as ADRs a partir da 0023. As ADRs 0011 a 0022 descrevem a implementação C#/WPF e ficam como histórico. Para uma nova, copie o [template](0000-template.md).

- [ADR-0011](0011-runtime-contracts-naming-and-targets.md): nome e frameworks-alvo do runtime compartilhado (histórico, implementação C#).
- [ADR-0012](0012-supervisor-e-ipc.md): supervisor da UI no próprio `Tako.exe` e IPC versionado por Named Pipes (histórico, implementação C#).
- [ADR-0013](0013-estado-de-monitores-e-audio.md): estado de monitores e áudio da sessão, contratos próprios e implementação Windows (histórico, implementação C#).
- [ADR-0014](0014-licenca-agpl.md): licença AGPL-3.0-only e reuso do Console Mode (Aceita).
- [ADR-0015](0015-monitores-do-console-mode-e-tela-de-jogo.md): monitores vindos do Console Mode e "Tela de jogo" (histórico, implementação C#).
- [ADR-0016](0016-audio-steam-e-tv-na-sessao.md): saída de áudio, Steam (`RunningAppID` e bandeja) e TV (ADB pela rede) durante a sessão de jogo (histórico, implementação C#).
- [ADR-0017](0017-contador-de-fps-com-presentmon.md): contador de FPS próprio pelo PresentMon, sem RTSS e sem injeção; permissão pelo grupo "Usuários de log de desempenho" (histórico, implementação C#).
- [ADR-0018](0018-limite-de-fps-pelo-driver.md): limite de FPS pelo driver de vídeo (NVAPI, ADLX, IGCL), sem RTSS nem elevação (histórico, implementação C#).
- [ADR-0019](0019-fontes-de-biblioteca.md): fontes de biblioteca (Steam, Epic Games, GOG, Xbox, Ubisoft Connect, EA app, Battle.net e Amazon Games) por uma interface comum, mais a importação única do Playnite; jogo em duas lojas fica em itens separados e ligados (histórico, implementação C#).
- [ADR-0020](0020-organizacao-das-configuracoes.md): Configurações em categorias, menu Modo Console com cartões e assistente, e layout que não corta (rolagem e barra de dicas) (histórico, implementação C#).
- [ADR-0021](0021-metadados-igdb.md): metadados pela IGDB com a chave da Twitch de cada usuário (sem proxy) e combinação por campo entre N provedores pela prioridade do tipo de jogo (histórico, implementação C#).
- [ADR-0022](0022-metadados-screenscraper.md): ScreenScraper para ROMs, com a conta de cada usuário e as credenciais do software só no build de release (segredos do CI); hash da ROM, mídia sem credenciais no disco e ROM no "Adicionar jogo" sem abrir (histórico, implementação C#).
- [ADR-0023](0023-slint-gate.md): Slint na interface e resultado do gate do MIG-001 (Aceita, números de referência pendentes).
- [ADR-0024](0024-matriz-de-plataforma.md): matriz real de suporte das funções de plataforma por SO (MIG-002).
- [ADR-0025](0025-fronteira-com-o-console-mode.md): fronteira entre o Tako e o Console Mode e contrato da futura API (Aceita).
- [ADR-0026](0026-supervisor-e-processo-durante-o-jogo.md): supervisor e processo do Tako durante o jogo (Aceita).
- [ADR-0027](0027-modo-desktop-e-big-picture.md): desktop principal, Big Picture e janela minimizada durante o jogo (Aceita).
- [ADR-0028](0028-downloads-por-torrent.md): downloads por torrent de fontes `.json` do usuário (DL-001): formato e validação das fontes, motor `librqbit`, vínculo ao jogo, fim do download sem executar nada, privacidade e avisos legais; instalações das lojas no mesmo painel (Proposta).
