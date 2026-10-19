# ADR-0033: API local de integração (EXT-002)

- Status: Proposta
- Data: 2026-10-18 (atualizado em 2026-10-19)

## Contexto

O Tako precisa permitir que integrações locais — como atalhos de teclado, Stream Deck, Home Assistant,
scripts de automação e o Console Mode (no sentido inverso, conforme PRD v0.2 §3 e ADR-0025) — possam
consultar o estado da biblioteca, saber se um jogo está em execução e acionar ações básicas como abrir a tela
de um jogo ou iniciá-lo.

Princípios do Tako envolvidos (PRD §0, §7 e `regras.md`):
1. **Privacidade e segurança:** nada sai da máquina sem consentimento; nenhuma porta externa pode ser exposta;
   qualquer API local exige autenticação explícita.
2. **Opt-in estrito:** a API deve vir **desligada por padrão** e só iniciar se o usuário ativá-la nas configurações.
3. **Paridade com links externos:** ações de jogo disparadas pela API devem passar exatamente pelas mesmas checagens
   e confirmações de segurança que o protocolo `tako://play` exige hoje (não executar comandos arbitrários nem
   burlar aprovações do usuário).
4. **Sem dependências desnecessárias:** o Tako já possui `serde`, `serde_json` e `std::net` no workspace. Não há
   necessidade de crates pesadas de web server para atender a uma API local focada.

## Decisão

Implementar a API local de integração com a seguinte especificação:

### 1. Transporte e Rede
- Protocolo: HTTP/1.1 sobre TCP.
- Endereço de escuta: **estritamente em loopback local (`127.0.0.1`)**. Qualquer tentativa de bind em endereços
  externos (`0.0.0.0` ou IPs de interfaces de rede) é expressamente rejeitada.
- Porta: padrão `47774` (configurável pelo usuário via configuração `LocalApiPort`).
- Estado padrão: **desligada por padrão** (`LocalApiEnabled = false`). Só sobe a thread de escuta quando habilitada.
- Limites e proteção contra exaustão de recursos:
  - Limite de conexões concorrentes ativas (máximo de 16 conexões simultâneas).
  - Timeouts de leitura e escrita curtos (5 segundos).
  - Limites de tamanho de buffer: linha de requisição limitada a 8 KiB e cabeçalhos limitados a 16 KiB.
  - Percent-encoding inválido em identificadores na URL é rejeitado com `400 Bad Request` (sem truncamento silencioso).

### 2. Autenticação, Armazenamento do Token e Ciclo de Vida do Servidor
- Cada instalação do Tako gera localmente um token de autenticação seguro (32 caracteres hexadecimais aleatórios
  gerados a partir de UUID v4 criptográfico).
- Validação estrita do token: tokens fora do formato esperado (32 caracteres hexadecimais) são invalidados e regenerados.
- O token é armazenado no banco local de configurações (`LocalApiToken` na tabela `settings`).
- Toda requisição HTTP deve enviar o cabeçalho:
  ```http
  Authorization: Bearer <token>
  ```
- A comparação do token recebido com o token esperado é feita estritamente em tempo constante (constant-time, via XOR acumulado com validação prévia de tamanho), mitigando ataques de temporização (timing attacks).
- Requisições sem cabeçalho `Authorization`, com formato incorreto ou com token inválido recebem `401 Unauthorized`
  com corpo JSON `{"error": "Unauthorized"}`.
- **Proteção contra DNS Rebinding:** O cabeçalho `Host` é rigorosamente validado e deve corresponder a `127.0.0.1:<porta>` ou `localhost:<porta>`. Requisições com qualquer outro `Host` ou sem o cabeçalho são rejeitadas imediatamente com `421 Misdirected Request`.
- **CORS e pre-flight desativados:** A API local não libera origens de navegadores (`Access-Control-Allow-Origin: *` removido) e requisições HTTP `OPTIONS` respondem `405 Method Not Allowed`. Ferramentas e integrações locais comunicam-se via HTTP direto sem preflight de browser.
- **Proteção do banco `tako.db`:** No Unix, ao ligar a API local, o arquivo de banco de dados `tako.db` tem suas permissões restritas a `0600` (leitura e gravação exclusivas do usuário proprietário).
  - *Risco residual registrado:* Em sistemas de arquivos sem suporte a atributos POSIX (como partições FAT32/exFAT) ou em sistemas não-Unix (Windows), permissões numéricas não são aplicadas pelo SO desta forma, exigindo que o controle de acesso seja garantido pelas permissões de pasta do perfil do usuário.
- O usuário pode visualizar o token gerado (via diálogo com botão de cópia rápida) e regenerá-lo a qualquer momento em Configurações > Sistema > Integrações > API local (com confirmação em diálogo de perigo).
- O servidor só incrementa a geração quando o token efetivamente mudar (`set_token`). Quando o token mudar ou ao desligar o servidor, conexões já aceitas são encerradas de imediato (`stream.shutdown`) e o despacho de ações revalida a geração/sessão ativa, impedindo que workers remanescentes executem ações desatualizadas. O desligamento não bloqueia a interface do usuário.

### 3. Privacidade, Backup e Restauração (Integração com CLOUD-001)
- O token (`LocalApiToken`) é um segredo local específico do dispositivo: **não pode entrar em backup nem em exportação**.
- O consentimento de ativação (`LocalApiEnabled`) **não pode ser importado** ao restaurar um backup em outra máquina. O opt-in deve ser feito de forma consciente pelo usuário em cada instalação.
- Essas chaves são explicitamente marcadas em `tako-core` para exclusão pelo módulo de backup (CLOUD-001 / PR #137).

### 4. Ciclo de Vida da Interface e Eventos de Sessão
- A API é executada no processo da interface (`tako-ui`).
- **Disponibilidade:** a API **só responde enquanto a interface estiver aberta**.
  - No **modo desktop**, a interface permanece aberta ou minimizada durante o jogo, portanto a API continua disponível e informando o estado da sessão.
  - No **modo de tela cheia (Big Picture)**, por regra arquitetural (ADR-0026 e ADR-0027), a interface fecha durante a execução do jogo para liberar recursos da GPU e restaura a supervisão ao supervisor; a API local fecha junto com a interface e só volta a responder quando a interface reabrir após o término da sessão.
- **Sincronização de Estado via EXT-001:** o estado de jogos em execução (`running_game`) é sincronizado de forma reativa ouvindo os eventos de sessão de `tako_core::session::events` (`GameStarting`, `GameStarted` e `GameExited`).
  - O campo `started_at` reflete o timestamp real de início da sessão (`DateTime<Utc>`) recebido no evento, sem resetar a cada chamada ou consulta. Quando o início não for conhecido, o campo é emitido como `null`.

### 5. Escopo dos Endpoints (v1)

#### Leitura
- `GET /api/v1/games` (ou `/api/v1/library`):
  Retorna a lista de jogos conhecidos pelo Tako com seus dados essenciais (`id`, `title`, `store`, `installed`).
  Resposta `200 OK`:
  ```json
  [
    {
      "id": "steam:570",
      "title": "Dota 2",
      "store": "Steam",
      "installed": true
    }
  ]
  ```

- `GET /api/v1/state` (ou `/api/v1/status`):
  Retorna o estado atual de execução do Tako: se há um jogo aberto, qual é o jogo, título e quando iniciou.
  Resposta `200 OK`:
  ```json
  {
    "is_game_running": true,
    "running_game": {
      "id": "steam:570",
      "title": "Dota 2",
      "pid": 12345,
      "started_at": "2026-10-18T20:30:00Z"
    }
  }
  ```
  Se nenhum jogo estiver rodando:
  ```json
  {
    "is_game_running": false,
    "running_game": null
  }
  ```

#### Ações
- `POST /api/v1/games/:id/open`:
  Abre os detalhes do jogo na interface do Tako e traz a janela para a frente (`raise_current`).
  - Se a interface estiver inacessível (ex: loop de eventos fechado): `503 Service Unavailable`.
  - Jogo encontrado: `200 OK` `{"success": true, "id": "steam:570"}`.
  - Jogo não encontrado: `404 Not Found` `{"error": "Game not found"}`.

- `POST /api/v1/games/:id/play`:
  Inicia o fluxo para jogar o jogo, invocando a mesma rota e confirmação que o link `tako://play` exige hoje
  (`details::primary_action` via `open_link(Link::Game { id, play: true })`). Se o jogo exigir confirmações
  (como aprovação de comandos pré/pós-execução ou seleção de instalador/loja), o diálogo de confirmação é exibido
  na tela do Tako exatamente como na navegação interativa.
  - Se a interface estiver inacessível: `503 Service Unavailable`.
  - Jogo encontrado: `200 OK` `{"success": true, "id": "steam:570"}`.
  - Jogo não encontrado: `404 Not Found` `{"error": "Game not found"}`.

## Consequências

- **Positivas:**
  - Usuários e integrações externas ganham uma interface local estável, segura e documentada.
  - Nenhuma dependência externa adicionada ao workspace: usa `std::net::TcpListener` e `serde_json`.
  - A segurança é garantida por padrão (opt-in explícito, localhost restrito, Bearer token obrigatório).
  - Ações respeitam a política de consentimento do usuário.
  - Proteção contra DoS local (limite de conexões, timeouts e limites de bytes).
  - Exclusão do token e de consentimento prévio de backups e exportações.
- **Negativas:**
  - Em modo tela cheia exclusivo, a API fecha enquanto o jogo roda (pois a interface fecha). Caso seja desejada disponibilidade ininterrupta mesmo durante tela cheia, a API precisaria migrar futuramente para o supervisor do Tako.
