# ADR-0030: API local de integração (EXT-002)

- Status: Aceita
- Data: 2026-10-18

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

### 2. Autenticação e Armazenamento do Token
- Cada instalação do Tako gera localmente um token de autenticação seguro (32 caracteres hexadecimais aleatórios
  gerados a partir de UUID v4 criptográfico).
- O token é armazenado no banco local de configurações (`LocalApiToken` na tabela `settings`).
- Toda requisição HTTP (exceto pre-flight OPTIONS) deve enviar o cabeçalho:
  ```http
  Authorization: Bearer <token>
  ```
- Requisições sem cabeçalho `Authorization`, com formato incorreto ou com token inválido recebem `401 Unauthorized`
  com corpo JSON `{"error": "Unauthorized"}`.
- O usuário pode visualizar o token gerado e regenerá-lo a qualquer momento em Configurações > Sistema > Integrações > API local.
  A regeneração invalida imediatamente o token antigo.

### 3. Versionamento da API
- Rotas prefixadas por versão: `/api/v1/...`.
- Alterações futuras que quebrem compatibilidade utilizarão `/api/v2/...`.

### 4. Escopo dos Endpoints (v1)

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

- `GET /api/v1/state`:
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
  - Jogo encontrado: `200 OK` `{"success": true, "id": "steam:570"}`.
  - Jogo não encontrado: `404 Not Found` `{"error": "Game not found"}`.

- `POST /api/v1/games/:id/play`:
  Inicia o fluxo para jogar o jogo, invocando a mesma rota e confirmação que o link `tako://play` exige hoje
  (`details::primary_action` via `open_link(Link::Game { id, play: true })`). Se o jogo exigir confirmações
  (como aprovação de comandos pré/pós-execução ou seleção de instalador/loja), o diálogo de confirmação é exibido
  na tela do Tako exatamente como na navegação interativa.
  - Jogo encontrado: `200 OK` `{"success": true, "id": "steam:570"}`.
  - Jogo não encontrado: `404 Not Found` `{"error": "Game not found"}`.

### 5. Ponto de Ligação para Eventos de Sessão (EXT-001)
- O PR #132 (EXT-001) introduz `tako_core::session::events` com os eventos `GameStarting`, `GameStarted` e `GameExited`.
- O design da API local isola o estado em um modelo desacoplado (`ApiHandler`), permitindo que a chegada de eventos
  do EXT-001 atualize o estado de sessão de forma reativa ou exponha endpoints SSE / WebSocket futuros sem refatoração estrutural.

## Consequências

- **Positivas:**
  - Usuários e integrações externas ganham uma interface local estável, segura e documentada.
  - Nenhuma dependência externa adicionada ao workspace: usa `std::net::TcpListener` e `serde_json`.
  - A segurança é garantida por padrão (opt-in explícito, localhost restrito, Bearer token obrigatório).
  - Ações respeitam a política de consentimento do usuário.
- **Negativas:**
  - Quem usa portas diferentes precisa configurar manualmente caso a porta 47774 já esteja em uso por outro serviço local.
