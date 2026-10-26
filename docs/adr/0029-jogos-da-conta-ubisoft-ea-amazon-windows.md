# ADR-0029: Jogos da conta não instalados da Ubisoft, da EA e da Amazon no Windows (SRC-005)

- Status: Aceita para a Amazon (entra) e a EA (não entra); a Ubisoft fica pendente do spike da seção 2
- Data: 2026-10-18

## Contexto

O SRC-005 (PRD §9) mostra os jogos da conta que não estão instalados, com "Instalar pela loja". Já entraram Steam,
Epic e GOG no Windows e Epic, GOG e Amazon no Linux/macOS pelo Heroic (#107). Faltavam Ubisoft Connect, EA app e
Amazon Games no Windows. O #107 deixou os três de fora; o orquestrador anotou que a Amazon cabia injetando o DPAPI
que o `tako-platform` já tem.

Regras que valem para qualquer leitura (ADR-0019, que já não usa o `IS` cifrado da EA e abre pelo cliente oficial para
não contornar login nem DRM; ADR-0028, que repete que o Tako não contorna DRM; e CONTRIBUTING §5):

- biblioteca 100% offline: sem login, sem rede, sem o cliente aberto. API privada da loja está fora;
- não contornar criptografia nem DRM da loja;
- formato com fonte citada e testável com fixture fictícia fiel ao formato; dado externo com limite de tamanho, sem
  pânico, entrada quebrada testada;
- `unsafe` só no `tako-platform`.

## Decisão

### 1. Amazon Games no Windows: entra

- **Arquivo:** `%LOCALAPPDATA%\Amazon Games\Data\Games\Sql\Entitlements.sqlite`, tabela `game_entitlements`, coluna
  `value`. Cada linha é um JSON com `ProductIdStr` (o mesmo `Id` do `GameInstallInfo.sqlite` dos instalados) e
  `ProductTitle`, cifrado pelo **DPAPI do usuário**, sem entropia. Fonte: plugin Amazon do GOG Galaxy,
  [`plugin.py`](https://github.com/Rall3n/galaxy-integration-amazon/blob/master/src/plugin.py) (`_get_owned_games`),
  [`client.py`](https://github.com/Rall3n/galaxy-integration-amazon/blob/master/src/client.py) (caminho) e
  [`utils.py`](https://github.com/Rall3n/galaxy-integration-amazon/blob/master/src/utils.py)
  (`CryptUnprotectData(..., None, ..., CRYPTPROTECT_UI_FORBIDDEN, ...)`).
- **Por que não é contornar criptografia:** o DPAPI do usuário é a função do próprio Windows para o usuário ler os
  próprios dados. Não há chave embutida nem derivada à mão: só decifra na conta do Windows que cifrou, e falha em
  outro usuário ou outro PC. É o mesmo que o Tako já faz com os segredos das alphas (`windows::dpapi::unprotect`).
- **Como:** `amazon::owned` é parser puro e recebe o decifrador como `fn(&[u8]) -> Option<Vec<u8>>`, igual ao
  `alpha_migration::migrate_secrets`. A composição (`tako-ui/src/sources.rs`) passa
  `tako_platform::windows::dpapi::unprotect`; o `unsafe` continua só no `tako-platform`. Os testes usam um decifrador
  falso e rodam em todos os SOs.
- **Limites:** banco até 256 MB, linha até 1 MB. Linha que não decifra, não é JSON ou não tem id e título é pulada; banco
  quebrado vira lista vazia com log. Só entram com o app da Amazon instalado (é ele que instala).
- **Instalar:** `amazon-games://install/<id>`, o mesmo do plugin Amazon do Playnite
  ([`AmazonGameController.cs`](https://github.com/JosefNemec/PlayniteExtensions/blob/master/source/Libraries/AmazonGamesLibrary/AmazonGameController.cs)).
  O id é `amazon:<id>`, igual ao instalado, então vira instalado no lugar.
- **Fora:** `GameProductInfo.sqlite` (tabela `DbSet`), marcado como legado no mesmo plugin.

### 2. Ubisoft Connect: não entra agora

- **O que existe no disco:** `cache\ownership\<userId>` (jogos da conta) e `cache\configuration\configurations`
  (nomes), na pasta do Ubisoft Connect. Não são cifrados, mas são binários sem especificação pública.
- **Por que não:** a única leitura conhecida é engenharia reversa do plugin Ubisoft do GOG Galaxy
  ([`local_file_parser.py`](https://github.com/FriendsOfGalaxy/galaxy-integration-uplay/blob/master/src/local_file_parser.py)):
  cabeçalho fixo de `0x108` bytes, registros lidos byte a byte procurando as tags `0x0a`/`0x08`/`0x10`, tamanhos
  corrigidos por uma fórmula empírica ("konrad's formula") e nomes num YAML dentro de cada registro, filtrado por
  `start_game`. O plugin não é mantido e quebra em versões novas do PyYAML. Sem um arquivo real, uma fixture fictícia
  só testaria a nossa própria suposição. O erro provável não é um arquivo recusado: é uma lista errada sem aviso, com
  jogo faltando, DLC ou pacote aparecendo como jogo, ou `uplay://install/<id>` abrindo o item errado.
- **Alternativas:**
  1. API da Ubisoft com login do usuário (o que o plugin faz por `public-ubiservices.ubi.com`): é privada, não
     documentada, exige guardar a sessão da conta e põe rede na biblioteca. Rejeitada.
  2. Ler os arquivos locais depois de conferir o formato com arquivos reais. É a recomendada (abaixo).
  3. Deixar a Ubisoft só com os instalados (como hoje).
- **Recomendação:** um `spike/SRC-005-ubisoft` num PC de mantenedor com Ubisoft Connect logado e alguns jogos da conta
  não instalados. O spike registra a estrutura dos dois arquivos (sem conteúdo da conta no repositório) e confere se
  são protobuf comum depois do cabeçalho. Se forem, o leitor reusa o decodificador mínimo de
  `battlenet/product_db.rs` e lê só `name`/`start_game` do YAML, sem dependência nova. Até lá, fica a opção 3.

### 3. EA app: não entra

- **O que existe no disco:** só os instalados. O `__Installer\installerdata.xml` de cada jogo, que a fonte da EA já
  lê, e `%ProgramData%\EA Desktop\…\IS`, com o estado de instalação, cifrado pela EA com uma chave que ela própria
  deriva. A lista da conta o EA app busca na conta, pela rede, a cada abertura; não grava um arquivo legível.
- **Por que não:** decifrar o `IS` exigiria reimplementar a derivação de chave da própria EA, o que é contornar a
  criptografia da loja (ADR-0019 e ADR-0028), e ele nem é a lista da conta. Não há outra fonte local. (Diferente do
  DPAPI da Amazon, que é a função do Windows para o usuário ler os próprios dados.)
- **Alternativas:**
  1. API da EA com login do usuário (o que fazem o EA app e launchers alternativos como o
     [Maxima](https://github.com/ArmchairDevelopers/Maxima)): privada, sem documentação oficial, exige guardar o token
     da conta e põe rede na biblioteca. Rejeitada pelas mesmas razões da Ubisoft.
  2. Jogos da EA comprados na Steam ou na Epic já aparecem como não instalados por essas lojas.
  3. O usuário instala pelo EA app; o jogo aparece no Tako quando instalado.
- **Recomendação:** a EA fica só com os instalados (2 e 3 já cobrem o caso comum). Revisar se a EA publicar uma API
  oficial com consentimento do usuário (OAuth) ou passar a gravar a lista num arquivo legível. Uma API com login só
  entraria com ADR própria e opt-in explícito, como a chave da Web API da Steam.

## Consequências

- Amazon no Windows mostra os jogos da conta não instalados, sem rede e sem login. Se a Amazon mudar o banco ou a
  cifra, só essa leitura fica vazia (com log); os instalados seguem.
- **Incertezas da Amazon, para o roteiro do Windows (`docs/testing/windows.md` §22):** o plugin não separa DLC nem
  conteúdo do Prime, então um deles pode aparecer como jogo, e não há arquivo real neste PC para conferir os campos do
  JSON além dos dois usados. **Risco aceito:** o roteiro pede os nomes das chaves do JSON (sem valores) para um filtro
  futuro. O id também é validado (`[A-Za-z0-9._-]+`) antes de entrar no endereço de instalar.
- Ubisoft e EA continuam só com os instalados. A Ubisoft volta com o spike; a EA, só com uma API oficial ou um arquivo
  legível.
