# API Local de Integração (EXT-002)

O Tako oferece uma API HTTP local para integração com Stream Deck, automações locais, Home Assistant, scripts e o Console Mode.

## Segurança e Privacidade

- **Apenas conexões locais:** O servidor escuta **estritamente em `127.0.0.1`** (loopback). Nenhuma interface de rede externa tem acesso.
- **Opt-in explícito:** A API vem desativada por padrão.
- **Autenticação obrigatória:** Todas as requisições exigem um token enviado no cabeçalho `Authorization: Bearer <token>`.
- O token é gerado localmente na primeira ativação e pode ser consultado ou regenerado a qualquer momento em:
  **Configurações > Sistema > Integrações > API local**.

## Configuração

| Configuração | Padrão | Descrição |
|---|---|---|
| Ativar API local | `false` (desligado) | Inicia o servidor HTTP ao abrir o Tako |
| Porta | `47774` | Porta TCP local para conexões |
| Token | Gerado aleatoriamente | Token de 32 caracteres hexadecimais |

---

## Exemplos com `curl`

Substitua `SEU_TOKEN_AQUI` pelo token obtido em Configurações.

### 1. Listar jogos da biblioteca

Retorna todos os jogos conhecidos com identificador, título, loja e status de instalação.

```bash
curl -X GET http://127.0.0.1:47774/api/v1/games \
  -H "Authorization: Bearer SEU_TOKEN_AQUI"
```

**Resposta de exemplo (`200 OK`):**
```json
[
  {
    "id": "steam:570",
    "title": "Dota 2",
    "store": "Steam",
    "installed": true
  },
  {
    "id": "manual:hades",
    "title": "Hades",
    "store": "Manual",
    "installed": true
  }
]
```

Também é aceito no caminho `/api/v1/library`.

---

### 2. Consultar o estado atual (jogo aberto ou não)

Informa se existe algum jogo aberto no momento, qual é o jogo e quando a sessão começou.

```bash
curl -X GET http://127.0.0.1:47774/api/v1/state \
  -H "Authorization: Bearer SEU_TOKEN_AQUI"
```

**Resposta quando há jogo rodando (`200 OK`):**
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

**Resposta quando nenhum jogo está aberto (`200 OK`):**
```json
{
  "is_game_running": false,
  "running_game": null
}
```

---

### 3. Abrir a tela de um jogo no Tako

Abre a página de detalhes do jogo na interface do Tako e traz a janela para a frente.

```bash
curl -X POST http://127.0.0.1:47774/api/v1/games/steam:570/open \
  -H "Authorization: Bearer SEU_TOKEN_AQUI"
```

**Resposta (`200 OK`):**
```json
{
  "success": true,
  "id": "steam:570"
}
```

Se o ID não existir na biblioteca:
```json
// Status: 404 Not Found
{
  "error": "Game not found"
}
```

---

### 4. Jogar um jogo

Dispara o comando de jogar. Passa pelas mesmas checagens e confirmações que o link `tako://play` exige (comandos de antes/depois, verificação de arquivos, etc.). Se confirmação for necessária, o diálogo surge na tela do Tako.

```bash
curl -X POST http://127.0.0.1:47774/api/v1/games/steam:570/play \
  -H "Authorization: Bearer SEU_TOKEN_AQUI"
```

**Resposta (`200 OK`):**
```json
{
  "success": true,
  "id": "steam:570"
}
```

Se o ID não existir na biblioteca:
```json
// Status: 404 Not Found
{
  "error": "Game not found"
}
```

---

## Erros comuns

- **`401 Unauthorized`:** Token ausente ou inválido no cabeçalho `Authorization: Bearer`.
  ```json
  {
    "error": "Unauthorized"
  }
  ```
- **`404 Not Found`:** Rota desconhecida ou jogo não encontrado na biblioteca.
  ```json
  {
    "error": "Not found"
  }
  ```
- **`405 Method Not Allowed`:** Método HTTP incorreto para a rota (ex.: `POST` em `/api/v1/games`).
  ```json
  {
    "error": "Method not allowed"
  }
  ```
