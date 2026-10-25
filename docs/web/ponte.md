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
| CEF (WEB-02a, WEB-02b) | função V8 registrada no processo de renderização → mensagem de processo para o principal do filho `--web-ui` → linha JSON no stdout do filho → núcleo (`tako --web`) → `bridge::handle_message` | resposta e eventos em ordem, em linhas JSON no stdin do filho → `ExecuteJavaScript("window.__takoDeliver(…)")` com o texto como string JS escapada. Detalhes em `docs/web/hospedeiro.md` |
| WebView2 (MIG-W01) | `chrome.webview.postMessage` (o script injetado adapta) | `PostWebMessageAsString` + `chrome.webview.addEventListener("message")` → `__takoDeliver` |
| Falso (navegador) | `installFakeHost()` em `fake.ts` | `setTimeout` com dados de exemplo |

O `AppService` roda na thread do núcleo. Chamada demorada (ler uma loja, baixar arte) responde logo com o
estado atual e avisa o resto por evento; o hospedeiro nunca trava a janela esperando o núcleo.

**Pedidos ao hospedeiro.** O que só o hospedeiro sabe fazer (mexer na janela, abrir um jogo pela sessão) passa
por `HostRequest`: a chamada (`window.action`, `game.play`…) manda o pedido pelo canal de `set_host_sink` e
responde assim que ele foi aceito; o resultado chega depois por evento. Assim o `AppService` não conhece janela
nem supervisor.

## 4. Esquemas próprios

Nenhuma porta de rede. Tudo que a página carrega vem de dois esquemas, registrados como seguros e com CSP:

**`tako-app://app/<caminho>`**: o build do `web/` (`web/dist/`). Caminho vazio ou rota desconhecida do React →
`index.html`. `schemes::resolve_app_path` recusa `..`, `\`, caminho absoluto, `%00`, codificação inválida e
link simbólico para fora da pasta.

**`tako-img://<tipo>?id=<id>&w=<largura>&h=<altura>`**: imagens do cache do núcleo, decodificadas no tamanho
pedido (pixels físicos). Tipos: `cover` (2:3), `hero` (larga), `logo`, `icon`. O `id` é o ID do jogo
(`steam:570`, `heroic:legendary:Fortnite`) com `encodeURIComponent`. `schemes::parse_image_url` recusa o resto.
Sem imagem no cache → 404, e a tela mostra o estado sem arte do design system.

O núcleo publica a resolução de **id + tipo** por `tako_core::metadata::game_art::GameArt`. O filho CEF lê esse
índice atualizado em disco a cada pedido, sem snapshot da biblioteca, consulta pela ponte ou rede. Arquivos
locais escolhidos e arte do cache da Steam são copiados para o cache de arte antes da publicação. Índices têm
nomes derivados de hash, limite de leitura e apenas o nome do arquivo; caminhos absolutos, `..` e links para
fora do cache são recusados. O esquema nunca espera por download: ausência continua sendo 404.
Ícones ICO locais da Steam também são importados; a resposta do esquema é PNG, como para os demais formatos.

**Chegada de arte:** `{"name":"art.changed","payload":{"gameId":"steam:570"}}`. O evento sai depois da
publicação no cache; pode haver mais de um por jogo (capa, fundo, logo e ícone). O React deve incrementar uma
versão local das URLs desse jogo, acrescentando `&v=<versão>` (até 64 bytes). `v` é opaco, não muda a resolução
nem vira caminho. `web/src/lib/imageVersions.ts` guarda a versão por jogo, e `useImageVersion.ts` mantém uma
assinatura da ponte por página. Todos os helpers chamam `imageUrl`; componentes que recebem uma URL pronta usam
`versionedImageUrl`. Falhas pertencem à URL/versão que falhou: o próximo evento permite tentar de novo, inclusive
em imagens que responderam 404. Desmontar e remontar uma tela preserva a versão durante a vida da página.
Sem interface aberta o evento se perde; o cache e o índice permanecem e a próxima abertura já encontra a arte.

```ts
import { imageUrl } from "../lib/imageVersions";
const src = imageUrl("cover", game.id, w * devicePixelRatio, h * devicePixelRatio);
```

### Metadados no núcleo (WEB-32)

O núcleo compartilha o mesmo `MetadataAgent` entre “Fontes e IDs” e `MetadataWorker`. A fila usa os provedores,
cache, prioridades e espaçamento de 1,5 s do Slint, sem rede na thread da ponte. Na demonstração não cria a fila
nem lê o cofre. Chaves continuam nos clientes Rust; respostas/eventos não levam credenciais nem URLs de provedores.

`game.source_ids.set` grava a escolha e agenda uma atualização do jogo. ID explícito dispensa busca por título;
“Nenhum” dispensa inclusive a descoberta em lote daquela fonte; automático força a correspondência original.
Resultados levam internamente as escolhas usadas: se já mudaram, não são aplicados à biblioteca. Campos e artes
são reaplicados sobre os dados da loja, preservando preferências e imagens fixadas, e emitem `library.changed`.
As artes publicadas pela fila emitem `art.changed` depois da gravação do índice; arte removida limpa sua publicação.

`LibraryGame` acrescenta `genres` e `publisher` (nulos sem dados). `library.list` aceita `genres` (qualquer gênero
do grupo; combinado com os demais filtros) e acrescenta à página os gêneros dos resultados antes da paginação.
Categorias pede `limit: 0` para descobrir as fileiras e depois pagina cada gênero pelo núcleo. A busca geral usa
a publicadora na informação secundária da linha do jogo.

CSP da página: `default-src tako-app:; img-src tako-app: tako-img: data:; style-src tako-app: 'unsafe-inline';
script-src tako-app:; connect-src 'none'`. Em desenvolvimento, o servidor do Vite entra no lugar de `tako-app:`.

## 5. Desenvolvimento

- `TAKO_WEB_DEV_URL=http://localhost:5173 tako --web` (binário de `cargo build -p tako --features web`) faz o
  hospedeiro carregar o servidor do Vite (hot reload) em vez de `tako-app://`, com o
  DevTools liberado. Só em build de debug.
- `npm run dev` em `web/` sem o Tako usa o hospedeiro falso.

## 6. Interface descartável (D-10)

A interface pode ser fechada e reaberta a qualquer momento (jogo aberto, minimizado, bandeja). Por isso:
- Estado que precisa sobreviver (tela atual, jogo selecionado, filtros, rolagem) é guardado no núcleo com
  `ui.state.set` (JSON da própria interface, até 64 KiB) e lido ao abrir com `ui.state`.
- Evento emitido com a interface fechada se perde de propósito; ao abrir, a interface consulta tudo de novo.

## 7. Acrescentar uma chamada ou evento

1. Variante em `Call` e a resposta com o mesmo `#[serde(rename = "…")]` em `Reply` (ou variante em `Event`).
2. Tratamento em `AppService::handle` com testes (caso feliz, `not_found`, entrada estranha).
3. `cargo test -p tako-app` regenera `web/src/bridge/generated/`; commitar os `.ts` gerados junto.
4. Usar com `call("dominio.acao", payload)` na interface; o TypeScript confere payload e resposta.

Mudança que quebra uma tela já publicada (remover ou renomear campo) sobe o `BRIDGE_VERSION`.

## 8. Textos e idiomas

Todo texto visível vem dos catálogos `crates/tako-core/locales/<idioma>/*.toml` (pt-BR e en-US), os mesmos do app
atual. **Componente não escreve texto**: ele chama `t("chave")`. Chave nova entra nos dois `.toml` (o teste
`catalogs_match` reprova se faltar num deles); não existe catálogo no `web/`.

- **De onde vem.** `i18n.catalog` (sem argumentos) responde `{ lang, messages }`: o código do idioma atual
  (`pt-BR`) e o catálogo inteiro dele como mapa plano `chave completa → texto` (`settings.general.title`). Hoje são
  cerca de 120 KB, 12% do limite de 1 MiB; se um dia apertar, a chamada passa a receber uma lista de módulos.
- **Quando muda.** `settings.set` com `Language` diferente do atual emite `i18n.changed` (`{ lang }`); a interface
  pede o catálogo de novo e os componentes redesenham sozinhos. Igual ao anterior, inválido ou outra chave: sem evento.
- **Formato** (o de `tako_core::i18n`): parâmetro é `{nome}`; plural são duas chaves, `chave.one` e `chave.other`,
  e `one` vale só quando `n == 1`.

```tsx
import { useT } from "../i18n"; // em componentes: redesenha quando o idioma muda
// import { t } from "../i18n"; // fora deles (handler, aviso): lê o idioma da hora, sem redesenhar

const t = useT();
t("settings.general.title");                       // "Geral" / "General"
t("art.summary", { count: 3, total: 10 });         // "3 de 10 jogos para conferir"
t("common.games", { n: 1 });                       // chave sem texto próprio + n numérico → "1 jogo"
t("common.games", { n: 3 });                       // → "3 jogos" (o {n} também é preenchido)
```

- Chave sem texto aparece como a própria chave e, em desenvolvimento, avisa uma vez no console; nunca quebra a
  tela. Enquanto o primeiro catálogo não chega, `t` devolve a chave sem avisar.
- Valor de `{nome}` ausente em `vars` fica como está no texto. Nada de HTML: o texto vai como texto.
- O código está em `web/src/i18n/` (`core.ts` é puro e tem teste; `index.ts` liga a ponte e o React). O hospedeiro
  falso responde `i18n.catalog` com poucas chaves de exemplo: ao usar uma chave nova no navegador, acrescente-a em
  `SAMPLE_CATALOGS` de `fake.ts`.
- Os textos que ainda estão em `web/src/app/strings.ts` migram para o catálogo por tarefa própria; até lá os dois
  convivem.
