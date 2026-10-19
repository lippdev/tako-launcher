# ADR-0032: Protocolo para aplicação de capas com um clique do SteamGridDB (ART-002)

- Status: Proposta
- Data: 2026-10-18

## Contexto

O item **ART-002** do PRD (P2) prevê "aplicar uma imagem do site do SteamGridDB num jogo com um clique (protocolo próprio, como o SGDBoop)".

### Como funciona o fluxo SGDBoop

Nas preferências da conta do site [SteamGridDB](https://www.steamgriddb.com), os usuários podem habilitar a opção "SGDBoop". Ao ativá-la, cada imagem listada no catálogo (capas verticais, horizontais, planos de fundo/hero, logotipos e ícones) passa a exibir um botão "BOOP".

Ao clicar nesse botão, o navegador aciona uma URI registrada no sistema operacional no formato:
`sgdb://boop/<tipo>/<número>` (ou no formato nativo do Tako: `tako://steamgriddb/<tipo>/<número>`).

O PR #94 implementou esse mecanismo no Tako (parsers de URI, tratamento de links, download validado via HTTPS, aplicação automática para jogos Steam e tela de seleção com correspondência por proximidade de título para jogos de outras lojas). Entretanto, o PR foi fechado devido a um impasse na camada de autenticação externa e no modelo de credenciais.

### Correção de referências: a API v2 pública no repositório

A versão preliminar deste documento mencionava erroneamente a "ADR-0023 / PR #91". Retifica-se:
- A ADR-0023 trata do Slint Gate e o PR #91 trata de perfis de controle (INPUT-001).
- O seletor manual de capas com a API v2 pública do SteamGridDB foi implementado nos **PR #51 (`feat/covers-steamgriddb`)** e **PR #60 (`feat/sgdb-picker-filters`)**, onde cada usuário configura sua chave pessoal nas Configurações do Tako (conforme item ART-001 do PRD).

No entanto, a API v2 pública pesquisa ativos por ID de jogo (`/games/id/...`, `/heroes/game/...`), mas **não possui um endpoint público documentado para consultar diretamente uma imagem pelo seu ID numérico** (o número recebido no link do botão BOOP).

---

## O impasse das credenciais e o risco de extração de segredos do binário

1. **A rota oficial do botão (`/api/sgdboop/`):** o aplicativo oficial SGDBoop (desenvolvido pela equipe do SteamGridDB) consulta o endpoint privado `https://www.steamgriddb.com/api/sgdboop/` utilizando um token de aplicativo interno embutido no código do próprio SGDBoop.
2. **Incerteza da chave de usuário:** se o Tako consultar `/api/sgdboop/` enviando a chave pessoal de usuário configurada no aplicativo, a requisição pode ser rejeitada com erro HTTP 401 ou 403 caso o endpoint restrinja chamadas exclusivamente ao token do aplicativo oficial.
3. **Impersonation e ética:** embutir no código do Tako o token pertencente ao SGDBoop oficial viola as boas práticas éticas de software livre e os termos do serviço, sujeitando o Tako a bloqueios arbitrários.
4. **Risco de extração de segredos embutidos no CI (Análogo à ADR-0022):**
   - Caso o SteamGridDB conceda formalmente um token de aplicativo exclusivo para o Tako Launcher, esse token poderia ser injetado durante o build de CI de release como segredo do repositório (mantendo o código-fonte limpo).
   - **Entretanto, como já alertado na ADR-0022 para as credenciais do ScreenScraper, qualquer token embutido no binário final compilado pode ser facilmente extraído por terceiros** através de inspeção de strings (`strings tako`), engenharia reversa ou descompilação. O projeto deve estar plenamente ciente de que "segredo embutido em binário cliente distribuído" é uma proteção puramente cosmética e não previne extração.

---

## Conflitos de associação do protocolo `sgdb://` entre plataformas

A intenção original de interceptar links `sgdb://` gerados pelo botão BOOP do site esbarra em diferenças fundamentais na forma como cada sistema operacional gerencia manipuladores de protocolos personalizados:

1. **Windows (Convivência pacífica possível):**
   - No Registro do Windows (`HKCU\Software\Classes\sgdb\shell\open\command`), o instalador Inno Setup (`tako.iss`) pode inspecionar se já existe uma associação ativa para `sgdb://`.
   - Se o SGDBoop oficial já estiver instalado, o Tako **não sobrescreve** a chave, preservando o aplicativo do usuário e registrando apenas o esquema nativo `tako://`. Caso o SGDBoop não esteja presente, o Tako pode assumir a manipulação de `sgdb://`.
2. **Linux (Conflito direto em nível de sistema):**
   - A associação de protocolos no Linux ocorre declarando `MimeType=x-scheme-handler/sgdb;` no arquivo `.desktop` e registrando no `mimeapps.list` do ambiente de desktop (XDG).
   - Se o SGDBoop e o Tako estiverem ambos instalados, há um **conflito direto**: o ambiente gráfico atribui prioridade a apenas um deles. Não há como coexistir de forma cortês;
   - **Com o SGDBoop instalado, o navegador abrirá o SGDBoop oficial.**
3. **macOS (Conflito direto no LaunchServices):**
   - No macOS, os esquemas são declarados na chave `CFBundleURLTypes` dentro do `Info.plist` do pacote `Tako.app`.
   - Quando duas aplicações declaram manipular o mesmo protocolo (`sgdb://`), o LaunchServices da Apple não possui fallback condicional, roteando a ativação para um dos binários conforme a versão e data de registro.
   - **Com o SGDBoop instalado, o sistema operacional abre o SGDBoop oficial.**
4. **Localização de código FFI e manipulação de SO:**
   - Todo código nativo para interceptação e escuta de eventos de URL do sistema operacional (como a captura de Apple Events via `NSAppleEventManager` no macOS, conforme viabilizado no EXT-005 / PR #130) reside **estritamente em `tako-platform`**.

---

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **A. Testar chave pessoal na `/api/sgdboop/` (Recomendada imediatamente)** | Respeita a privacidade e o modelo de chaves do usuário (como na ADR-0021 e ART-001); sem segredos no binário; aproveita o PR #94. | Exige teste manual prévio com chave real para validar se a rota aceita chamadas de usuários comuns. |
| **B. Solicitar token oficial de app ao SteamGridDB** | Solução oficial autorizada pelos mantenedores do serviço; dispensa chave pessoal para o botão. | Depende da boa vontade de terceiros; como na ADR-0022, o token injetado no CI acaba dentro do binário e pode ser extraído. |
| **C. Userscript comunitário (TakoBoop)** | Substitui os links da página diretamente para `tako://steamgriddb/...`, eliminando qualquer conflito de protocolo com o SGDBoop no Linux/macOS. | Exige instalação de extensão de navegador pelo usuário (Tampermonkey/Violentmonkey). |
| **D. Solicitar rota de busca por ID na API v2 pública** | Arquitetura mais limpa a longo prazo (`/api/v2/grids/:id`). | Depende inteiramente do ciclo de desenvolvimento e deploy da equipe do SteamGridDB. |
| **E. Embutir o token interno do SGDBoop oficial** | Funcionaria imediatamente. | **Rejeitada categoricamente:** violação ética, uso não autorizado de credenciais de terceiros e risco iminente de revogação. |

---

## Decisão proposta

1. **Validação Manual da Chave de Usuário:**
   - Realizar teste prático em hardware com uma chave pessoal real de usuário contra a rota `https://www.steamgriddb.com/api/sgdboop/` para verificar se ela é aceita com `Authorization: Bearer <chave_pessoal>`.
2. **Contato Oficial com o SteamGridDB:**
   - Definir responsabilidades claras para contatar a equipe do SteamGridDB (via canais de parceiros ou Discord oficial), consultando sobre a concessão de token ou abertura da rota para chaves de usuário.
   - Registrar expressamente o risco de que, se um token de aplicativo for concedido e injetado no CI, ele **poderá ser extraído do binário final compilado** (idêntico à ADR-0022).
3. **Política de Registro de Protocolos:**
   - Registrar incondicionalmente o protocolo nativo `tako://` em todos os sistemas operacionais.
   - **Windows:** registrar `sgdb://` apenas se o SGDBoop oficial não estiver associado no Registro.
   - **Linux e macOS:** reconhecer que o registro de `sgdb://` conflita diretamente com o SGDBoop oficial instalado e submeter ao mantenedor a decisão de registrá-lo ou restringir-se ao protocolo próprio `tako://`.
4. **Salvaguardas de Aplicação e Isolamento de FFI:**
   - Todo FFI e integração de SO (Apple Events no macOS) fica isolado em **`tako-platform`**.
   - Validação estrita de CDN: apenas imagens originadas de `https://cdn2.steamgriddb.com/` são baixadas.
   - Rejeição de formatos incompatíveis: aviso informativo caso o usuário tente aplicar capa horizontal na grade estritamente vertical (600×900) da biblioteca.
   - Desambiguação de jogos por distância Levenshtein quando a correspondência não for imediata.

---

## Consequências

- O recurso permanece desativado na branch principal até que o teste da chave pessoal seja realizado e as tratativas com o SteamGridDB sejam consolidadas.
- Caso o usuário tenha o SGDBoop oficial instalado, no Linux e macOS o botão BOOP acionará o SGDBoop, mantendo a integridade da ferramenta pré-instalada.
- O código do Tako permanece transparente e auditável, sem embutimento não autorizado de credenciais de terceiros.

---

## Perguntas para o mantenedor

1. **Registro do `sgdb://` no Linux e no macOS:** Diante do conflito direto com o SGDBoop oficial no Linux (`.desktop` / `mimeapps.list`) e no macOS (`CFBundleURLTypes`), o projeto deve registrar o protocolo `sgdb://` nesses sistemas (assumindo o conflito caso o SGDBoop esteja instalado) ou registrar apenas o protocolo proprietário `tako://`?
2. **Teste da chave pessoal e contato oficial:** Quem testará a chave pessoal na rota `/api/sgdboop/` (o mantenedor dispõe de chave ativa para validação imediata) e quem fará o contato formal com a equipe do SteamGridDB para tratar do token dedicado ou rota pública?
3. **Aceite do risco de extração do token no CI:** Caso o SteamGridDB forneça um token de aplicativo exclusivo para o Tako, o projeto aceita o risco conhecido (idêntico ao da ADR-0022 para o ScreenScraper) de que o segredo injetado durante o build de CI acabará embutido no binário executável final e poderá ser extraído por engenharia reversa?
4. **Disponibilização de Userscript Comunitário:** Vale a pena disponibilizar um userscript comunitário no repositório (`tools/takoboop.user.js`) para usuários que queiram usar o botão com o protocolo `tako://` diretamente, contornando qualquer conflito de sistema?
