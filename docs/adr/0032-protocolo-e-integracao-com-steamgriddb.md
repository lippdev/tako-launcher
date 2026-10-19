# ADR-0032: Protocolo para aplicação de capas com um clique do SteamGridDB (ART-002)

- Status: Proposta
- Data: 2026-10-18

## Contexto

O item **ART-002** do PRD (P2) prevê "aplicar uma imagem do site do SteamGridDB num jogo com um clique (protocolo próprio, como o SGDBoop)".

### Como funciona o fluxo SGDBoop

Nas preferências da conta do site [SteamGridDB](https://www.steamgriddb.com), os usuários podem habilitar a opção "SGDBoop". Ao ativá-la, cada imagem listada no catálogo (capas verticais, horizontais, planos de fundo/hero, logotipos e ícones) passa a exibir um botão "BOOP".

Ao clicar nesse botão, o navegador aciona uma URI registrada no sistema operacional no formato:
`sgdb://boop/<tipo>/<número>` (ou no formato do próprio Tako: `tako://steamgriddb/<tipo>/<número>`).

O PR #94 implementou esse mecanismo no Tako (parsers de URI, tratamento de links, download validado via HTTPS, aplicação automática para jogos Steam e tela de seleção com correspondência por proximidade de título para jogos de outras lojas). Entretanto, o PR foi fechado pelo mantenedor devido a um impedimento arquitetural e ético na camada de autenticação externa.

### O problema do Token de Aplicativo × Chave Pessoal do Usuário

1. **A rota oficial do botão (`/api/sgdboop/`):** o aplicativo oficial SGDBoop (desenvolvido pela equipe do SteamGridDB) consulta o endpoint privado `https://www.steamgriddb.com/api/sgdboop/` utilizando um **token de aplicativo interno embutido no código-fonte do próprio SGDBoop**. Não há documentação oficial afirmando se esse endpoint aceita chaves de API pessoais de usuários.
2. **A API pública v2 (`/api/v2/`):** o Tako já suporta a API v2 pública do SteamGridDB no seletor manual de capas (ADR-0023 / PR #91), onde cada usuário informa sua chave de API pessoal nas Configurações. No entanto, a API v2 pública pesquisa ativos por ID de jogo (`/games/id/...`, `/heroes/game/...`), mas **não possui um endpoint público para consultar diretamente uma imagem pelo seu ID numérico** (o número recebido no link do botão BOOP).
3. **Impasses:**
   - **Impersonation:** embutir no código do Tako o token de aplicativo pertencente ao SGDBoop oficial viola as boas práticas éticas de software livre e termos de serviço, correndo o risco de revogação a qualquer momento.
   - **Incerteza da chave de usuário:** se o Tako consultar `/api/sgdboop/` enviando a chave pessoal do usuário configurada no Tako, a requisição pode ser rejeitada com erro HTTP 401 ou 403 caso o endpoint restrinja chamadas exclusivamente ao token do aplicativo SGDBoop.
   - **Dependência externa:** se chaves pessoais não forem aceitas, a funcionalidade requer que o SteamGridDB conceda um token de aplicativo próprio para o Tako Launcher.

---

## Aspectos técnicos adicionais

### 1. Associação de protocolos por SO

- **Windows:** o instalador Inno Setup (`tako.iss`) registra a associação para o protocolo `sgdb://` apenas se nenhum outro aplicativo (como o SGDBoop oficial) estiver previamente associado, ou se a associação existente já pertencer ao Tako. O protocolo próprio `tako://` é sempre registrado. A desinstalação só remove a associação se esta ainda apontar para o binário do Tako.
- **Linux:** o arquivo `.desktop` declara manipulação dos tipos MIME `x-scheme-handler/sgdb` e `x-scheme-handler/tako`.
- **macOS:** o pacote de aplicação `Tako.app` declara os esquemas em `CFBundleURLTypes` e a captura ocorre via Apple Event (`kAEGetURL` no `NSAppleEventManager`), conforme arquitetura viabilizada no EXT-005 (PR #130).

### 2. Validação de segurança e integridade

- **Filtro de URLs de download:** o Tako aceita única e exclusivamente URLs HTTPS com destino aos domínios da CDN oficial do SteamGridDB (`cdn2.steamgriddb.com`). Qualquer tentativa de redirecionamento para URLs HTTP, domínios de terceiros, portas não padrão ou parâmetros maliciosos deve ser terminantemente descartada.
- **Formato visual:** a grade da biblioteca do Tako é estritamente vertical (proporção 600×900). Se o usuário acionar o botão BOOP em uma capa horizontal, o Tako deve emitir um aviso informativo orientando o usuário a selecionar a capa em formato vertical, sem aplicar a imagem horizontal de forma distorcida.
- **Desambiguação inteligente:** caso a imagem pertença a um jogo fora da Steam (ou com ID não casado de imediato), a interface abre o modal "Aplicar imagem em...", ordenando a lista da biblioteca pela proximidade de nome (distância Levenshtein) para agilizar a confirmação em um único toque.

---

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **A. Solicitar token oficial de aplicativo ao SteamGridDB (Recomendada a médio prazo)** | Solução 100% legítima e suportada pelos mantenedores do serviço; funciona de forma transparente para todos os usuários sem exigir chave pessoal para o botão. | Depende da aprovação e tempo de resposta de terceiros; exige gerenciamento seguro de segredos de build no CI. |
| **B. Testar e permitir chave pessoal do usuário na `/api/sgdboop/` (Recomendada imediatamente)** | Mantém a filosofia do Tako de chaves fornecidas pelo próprio usuário (como Twitch/IGDB na ADR-0021); não impersona outro aplicativo; código já pronto no PR #94. | Requer teste manual em hardware com chave real para validar se a API do botão não bloqueia tokens de usuário comum. |
| **C. Solicitar endpoint de busca por asset ID na API v2 pública** | Solução mais limpa arquiteturalmente, unificando tudo na API pública oficial. | Depende de alteração de código e deploy no backend do SteamGridDB pela equipe deles. |
| **D. Userscript / Extensão de navegador comunitária (TakoBoop)** | Dispensa qualquer consulta à API do botão: o userscript na página lê os dados visíveis e aciona `tako://steamgriddb/apply?url=...&name=...`; funciona imediatamente. | Exige que o usuário instale uma extensão de navegador (Tampermonkey/Violentmonkey) em vez de usar apenas o botão nativo do site. |
| **E. Embutir o token do SGDBoop oficial** | Funciona imediatamente sem depender de ninguém. | **Rejeitada categoricamente:** violação ética, uso não autorizado de credenciais de outro projeto e vulnerabilidade a bloqueio arbitrário. |

---

## Decisão proposta

1. **Ação imediata de teste:** realizar teste manual em hardware com uma chave pessoal real de usuário contra o endpoint `https://www.steamgriddb.com/api/sgdboop/` para atestar se ela é aceita.
2. **Contato com a equipe do SteamGridDB:** abrir contato formal via canais de desenvolvedores da plataforma solicitando:
   - Um token de aplicativo oficial dedicado ao Tako Launcher; ou
   - A documentação e autorização do uso de chaves pessoais no endpoint `/api/sgdboop/`; ou
   - A criação de um endpoint na API pública v2 (`GET /api/v2/grids/id/:id`) permitindo recuperar metadados de uma arte por seu ID com chave comum.
3. **Adoção do código do PR #94 com salvaguardas:**
   - Aproveitar a implementação testada do PR #94 quando a questão de credenciais for sanada.
   - O Tako utilizará primariamente a chave de API cadastrada pelo próprio usuário nas Configurações. Caso a requisição retorne HTTP 401/403 com indicação de token de aplicativo obrigatório, o Tako exibirá aviso claro explicando a limitação e o motivo pelo qual a imagem não pôde ser resgatada.
4. **Associação de protocolos:**
   - Registrar `tako://` como protocolo primário e `sgdb://` como opcional e cortês (não sobrescrevendo o SGDBoop oficial se já estiver instalado na máquina).
   - No macOS, utilizar o manipulador de Apple Events estabelecido no EXT-005.

---

## Consequências

- O recurso permanece desativado na branch principal até que a validação da chave pessoal seja concluída ou as credenciais oficiais sejam fornecidas.
- Se a chave pessoal for aceita pela API do botão, o Tako entrega o recurso aos usuários sem necessidade de intermediários ou servidores próprios.
- A biblioteca ganha uma experiência de enriquecimento visual extremamente rápida e ergonômica sem comprometer a estabilidade do layout de capas (rejeição de capas horizontais e validação rigorosa de CDN).

---

## Perguntas para o mantenedor

1. **Teste da chave pessoal na API do botão:** O mantenedor possui uma chave de API pessoal do SteamGridDB disponível para testar se a rota `https://www.steamgriddb.com/api/sgdboop/` aceita o header `Authorization: Bearer <chave_pessoal>` retornando os dados da imagem com sucesso?
2. **Canal de contato oficial com o SteamGridDB:** O mantenedor prefere abrir contato formal com a equipe do SteamGridDB (via Discord oficial ou e-mail de parcerias) solicitando um token de aplicativo dedicado para o Tako Launcher?
3. **Distribuição de token de app caso concedido:** Caso o SteamGridDB forneça um token de aplicativo exclusivo para o Tako, o projeto aceita injetá-lo durante o build de CI de release como segredo (idêntico ao modelo adotado para as credenciais do ScreenScraper na ADR-0022), mantendo o código-fonte livre de segredos expostos?
4. **Userscript alternativo como paliativo:** Vale a pena disponibilizar um userscript comunitário no repositório (`tools/takoboop.user.js`) para usuários avançados que queiram aplicar capas com um clique via link direto `tako://` antes da conclusão das tratativas com o SteamGridDB?
