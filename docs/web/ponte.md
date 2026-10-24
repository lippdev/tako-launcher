# Ponte entre a interface web e o núcleo Rust

Contrato do WEB-00 (PRD D-09, D-10; ADR-0035). Vale para o hospedeiro CEF, para a variante WebView2 e para o
hospedeiro falso do navegador.

## 1. Peças

| Peça | Onde | Papel |
|---|---|---|
| Protocolo | `crates/tako-app/src/protocol.rs` | Tipos das mensagens; fonte dos tipos TypeScript |
| Serviço | `crates/tako-app/src/lib.rs` (`AppService`) | Atende cada chamada com o núcleo; emite eventos |
| Ponte Rust | `crates/tako-web/src/bridge.rs` | Texto JSON → `AppService` → texto JSON; valida tamanho, formato e versão |
| Esquemas | `crates/tako-web/src/schemes.rs` | Valida `tako-app://` e `tako-img://` |
| Cliente TS | `web/src/bridge/index.ts` | `call()` e `on()` tipados |
| Tipos TS | `web/src/bridge/generated/` | Gerados por `cargo test -p tako-app` (ts-rs); **não editar** |
| Hospedeiro falso | `web/src/bridge/fake.ts` | Interface no navegador sem o Tako |

## 2. Mensagens

Todas são JSON em texto, com `v` = versão do contrato (`BRIDGE_VERSION`, hoje 1).

**Pedido** (interface → núcleo):
```json
{ "v": 1, "id": 7, "name": "app.info", "payload": { } }
```
`payload` some quando a chamada não tem argumentos. `id` é um contador da interface (`u32`, dá a volta).

**Resposta** (núcleo → interface), uma por pedido, com o mesmo `id`:
```json
{ "v": 1, "id": 7, "ok": true,  "reply": { "name": "app.info", "data": { "version": "0.3.0", "bridgeVersion": 1 } } }
{ "v": 1, "id": 8, "ok": false, "error": { "code": "not_found", "message": "jogo steam:570" } }
```

**Evento** (núcleo → interface, sem pedido):
```json
{ "v": 1, "event": { "name": "library.changed" } }
```

Regras:
- Nomes em `dominio.acao`. Consulta com substantivo (`library.list`, `game.details`); comando com verbo
  (`game.play`, `settings.set`).
- Cada `Call` tem uma `Reply` com o **mesmo nome**. Comando sem retorno responde com `data` vazio do tipo certo.
- Campos em `camelCase` no JSON (`#[serde(rename_all = "camelCase")]` nos structs).
- `error.message` é só para log. A tela escolhe o texto pelo `error.code` nos catálogos de `locales/`.
- Mensagem acima de 1 MiB é recusada. Listas grandes (biblioteca) são paginadas ou filtradas no núcleo.

| `code` | Quando |
|---|---|
| `bad_request` | Não é JSON, formato errado, grande demais |
| `unknown_call` | Nome que este núcleo não conhece |
| `version_mismatch` | `v` diferente; a interface mostra "atualize o Tako" |
| `not_found` | O alvo não existe |
| `conflict` | Válido, mas não agora (jogo já aberto…) |
| `internal` | Falha do núcleo; detalhes no log |

## 3. Transporte (cada motor)

O hospedeiro injeta, **antes** de a página carregar:

```ts
window.takoHost = { post(message: string): void }
```

e entrega cada resposta e evento chamando `window.__takoDeliver(message)`, que o cliente (`index.ts`) registra.

| Motor | `post` | Entrega |
|---|---|---|
| CEF (WEB-02a) | função V8 registrada no processo de renderização → mensagem de processo para o principal → `bridge::handle_message` | `ExecuteJavaScript("window.__takoDeliver(…)")` ou mensagem de processo de volta, com o texto como string JS escapada |
| WebView2 (MIG-W01) | `chrome.webview.postMessage` (o script injetado adapta) | `PostWebMessageAsString` + `chrome.webview.addEventListener("message")` → `__takoDeliver` |
| Falso (navegador) | `installFakeHost()` em `fake.ts` | `setTimeout` com dados de exemplo |

O `AppService` roda na thread do núcleo. Chamada demorada (ler uma loja, baixar arte) responde logo com o
estado atual e avisa o resto por evento; o hospedeiro nunca trava a janela esperando o núcleo.

## 4. Esquemas próprios

Nenhuma porta de rede. Tudo que a página carrega vem de dois esquemas, registrados como seguros e com CSP:

**`tako-app://app/<caminho>`**: o build do `web/` (`web/dist/`). Caminho vazio ou rota desconhecida do React →
`index.html`. `schemes::resolve_app_path` recusa `..`, `\`, caminho absoluto, `%00`, codificação inválida e
link simbólico para fora da pasta.

**`tako-img://<tipo>?id=<id>&w=<largura>&h=<altura>`**: imagens do cache do núcleo, decodificadas no tamanho
pedido (pixels físicos). Tipos: `cover` (2:3), `hero` (larga), `logo`, `icon`. O `id` é o ID do jogo
(`steam:570`, `heroic:legendary:Fortnite`) com `encodeURIComponent`. `schemes::parse_image_url` recusa o resto.
Sem imagem no cache → 404, e a tela mostra o estado sem arte do design system.

```ts
const src = `tako-img://cover?id=${encodeURIComponent(game.id)}&w=${w * devicePixelRatio}&h=${h * devicePixelRatio}`;
```

CSP da página: `default-src tako-app:; img-src tako-app: tako-img: data:; style-src tako-app: 'unsafe-inline';
script-src tako-app:; connect-src 'none'`. Em desenvolvimento, o servidor do Vite entra no lugar de `tako-app:`.

## 5. Desenvolvimento

- `TAKO_WEB_DEV_URL=http://localhost:5173 tako --web` faz o hospedeiro carregar o servidor do Vite (hot reload)
  em vez de `tako-app://`, com o DevTools liberado. Só em build de debug.
- `npm run dev` em `web/` sem o Tako usa o hospedeiro falso.

## 6. Interface descartável (D-10)

A interface pode ser fechada e reaberta a qualquer momento (jogo aberto, minimizado, bandeja). Por isso:
- Estado que precisa sobreviver (tela atual, jogo selecionado, filtros, rolagem) é guardado no núcleo por uma
  chamada (`ui.state.set`, a criar no WEB-02a) e lido ao abrir (`ui.state`).
- Evento emitido com a interface fechada se perde de propósito; ao abrir, a interface consulta tudo de novo.

## 7. Acrescentar uma chamada ou evento

1. Variante em `Call` e a resposta com o mesmo `#[serde(rename = "…")]` em `Reply` (ou variante em `Event`).
2. Tratamento em `AppService::handle` com testes (caso feliz, `not_found`, entrada estranha).
3. `cargo test -p tako-app` regenera `web/src/bridge/generated/`; commitar os `.ts` gerados junto.
4. Usar com `call("dominio.acao", payload)` na interface; o TypeScript confere payload e resposta.

Mudança que quebra uma tela já publicada (remover ou renomear campo) sobe o `BRIDGE_VERSION`.
