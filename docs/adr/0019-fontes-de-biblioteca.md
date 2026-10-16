# ADR-0019: Fontes de biblioteca (Steam, Epic Games, GOG, Xbox, Ubisoft Connect, EA app, Battle.net, Amazon Games e Playnite)

- Status: Proposta
- Data: 2026-10-15

## Contexto

Até aqui a biblioteca juntava a Steam (lida direto dos arquivos da Steam), o Xbox (#40, `XboxLibraryService` com
liga/desliga próprio) e os jogos adicionados pelo usuário, cada loja ligada à mão na composição do app
(`App.CreateLocalLibrary`), no "Procurar jogos instalados" e no construtor do `ShellViewModel`. O PRD pede Epic Games
(STORE-002), GOG (STORE-003) e depois Ubisoft, EA, Battle.net e Amazon; também pede jogos
repetidos sem perder a loja (STORE-010), abrir pelos clientes oficiais quando exigido (STORE-011) e avisar quando o
cliente falta (STORE-012). Cada loja nova não pode mexer na composição nem na biblioteca.

## Decisão

1. **Contrato** em `Launcher.Core/Library/Sources`:
   - `ILibrarySource { GameStore Store; LibraryScan Scan(); }`. `Scan` lê os jogos **instalados** do disco ou do
     registro, sem a loja aberta e sem rede, e devolve `LibraryScan(Games, ClientInstalled, Notice)`: os jogos já como
     `Game` (id `"<loja>:<id da loja>"`), se o cliente oficial está no PC e um aviso curto para as Configurações.
   - `IRegistryReader` (subchaves e valores de texto): o registro fica atrás de interface; os testes usam um registro
     em memória e os parsers são funções puras sobre texto (testáveis no Mac/Linux).
   - `LibrarySources` junta as fontes: lê só as lojas ligadas, isola a falha de cada uma (IO, permissão, JSON
     inválido → a loja fica vazia, com aviso, e as outras seguem), ignora ids repetidos e guarda um
     `LibrarySourceReport` por loja para as Configurações. `LocalGameLibrary` recebe `LibrarySources.Discover`.
   - Uma loja nova = uma classe `ILibrarySource` + uma linha na composição (`App.CreateLibrarySources`) + uma linha
     nas Configurações. A interface tem dois membros de propósito, para as próximas lojas se registrarem igual.
   - "Procurar jogos instalados" resume todas as lojas ligadas (`LibraryText.ScanSummary`: "12 jogos da Steam, 2 da
     Epic Games e 3 do Xbox encontrados").
2. **Epic Games** (`Library/Epic`):
   - Jogos: manifests `*.item` (JSON) em `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests` (ou o `AppDataPath`
     gravado pela Epic em `HKLM\SOFTWARE\WOW6432Node\Epic Games\EpicGamesLauncher`). Ficam de fora instalação
     incompleta (`bIsIncompleteInstall`), DLC (`MainGameAppName` diferente de `AppName`) e itens sem a categoria
     `games` (plugins e o Unreal Engine). Jogo cuja pasta sumiu é procurado em `LauncherInstalled.dat`
     (`%ProgramData%\Epic\UnrealEngineLauncher`), que tem a pasta atual quando a Epic move o jogo; sem pasta, fica de fora.
   - Abrir: **sempre** pelo protocolo oficial
     `com.epicgames.launcher://apps/<CatalogNamespace>%3A<CatalogItemId>%3A<AppName>?action=launch&silent=true`, nunca o
     executável direto: a Epic faz login, DRM, nuvem e atualização. O fim da sessão vem dos processos dentro da pasta do
     jogo (estratégia "pasta" já existente, MVP-026).
   - Cliente: o protocolo registrado em `HKCR\com.epicgames.launcher\shell\open\command`. Sem ele os jogos aparecem,
     mas as Configurações avisam "Epic Games Launcher não encontrado: instale-o para abrir estes jogos", e a falha ao
     abrir mostra a mesma frase.
3. **GOG** (`Library/Gog`):
   - Jogos: `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<id>` (`gameName`, `path`, `exe`, `launchParam`, `workingDir`,
     `launchCommand`, `dependsOn`), gravado tanto pelo GOG Galaxy quanto pelos instaladores offline, mais o
     `goggame-<id>.info` da pasta (tarefa principal `isPrimary`/`FileTask`). DLC (`dependsOn` ou `rootGameId` diferente)
     e entradas cuja pasta sumiu ficam de fora.
   - Abrir: pelo GOG Galaxy quando instalado
     (`GalaxyClient.exe /command=runGame /gameId=<id> /path="<pasta>"`, mantém nuvem e conquistas); sem Galaxy, pelo
     próprio jogo (registro `exe`, senão a tarefa do `.info`, senão `launchCommand`), já que jogos do GOG não têm DRM.
     O fim da sessão vem dos processos na pasta do jogo.
   - Cliente: `HKLM\SOFTWARE\WOW6432Node\GOG.com\GalaxyClient\paths` (`client`) + `clientExecutable`. Sem Galaxy o aviso
     é só informativo ("Sem GOG Galaxy: os jogos abrem direto pelo executável").
4. **Xbox / Microsoft Store** (STORE-004, #40): `XboxLibrarySource` envolve o `XboxLibraryScanner` sem mudar a
   leitura (pastas `.GamingRoot`, pacotes MSIX/AppX que são jogos, abrir pelo AUMID). "Cliente instalado" = app Xbox
   e Serviços de Jogos presentes; sem eles, o aviso de `XboxGameRules.Notice` ("App Xbox não encontrado…", "Serviços
   de Jogos não encontrados…") aparece primeiro na linha da loja. O `XboxLibraryService` e a configuração
   `XboxLibraryEnabled` saem: o liga/desliga vai para `DisabledStores` (um `XboxLibraryEnabled=false` já salvo vira
   Xbox desligado).
5. **Mesmo jogo em duas lojas (STORE-010): itens separados e ligados**, não unificados. Cada cópia continua um item com
   a própria loja (selo e cor), o próprio jeito de abrir, tempo de jogo e favorito; `StoreDuplicates.Link` compara os
   títulos normalizados (mesma limpeza de `TitleMatcher`: sem ™/®, acentos, pontuação e palavras de edição) e preenche
   `Game.AlsoInstalledIn`, mostrado nos detalhes ("Também em") e em Opções. A cópia sem appid da Steam herda o appid e as
   artes da cópia da Steam (capa certa sem esperar a busca). Motivo: unificar exigiria escolher por qual loja abrir,
   somar tempos de lojas diferentes e decidir o que fazer quando uma loja é desligada; com itens ligados nada disso
   muda e o usuário vê de onde cada um vem. Unificar de verdade (um item com várias "formas de jogar") fica para quando
   houver tela para escolher a loja.
6. **Metadados**: jogos da Epic, do GOG e do Xbox passam pelo agente existente (casam pelo nome na Steam Store, como os
   adicionados). "Corrigir correspondência" passa a valer para todo jogo que não é da Steam.
7. **Configurações › Lojas**: uma linha por loja (Steam, Epic Games, GOG, Xbox) que liga/desliga com A, com o resultado da
   última leitura ("3 jogos instalados", "Epic Games Launcher não encontrado neste PC", "Desligada…"). A escolha fica
   no `tako.db` (`DisabledStores`); desligar some com os jogos da loja na hora, sem apagar favoritos nem tempo.
8. **Cores**: `store-epic` `#2A2A2A` e `store-gog` `#7A3FD1` do DESIGN.md (o selo do GOG estava `#86328A`).

### Rodada 4: Ubisoft Connect, EA app, Battle.net, Amazon Games e Playnite (STORE-005 a 009, 011, 012)

Mesmo padrão das lojas acima: uma classe `ILibrarySource` por loja, uma linha em `App.CreateLibrarySources`, uma linha
em Configurações › Biblioteca e lojas (liga/desliga com A, guardado em `DisabledStores`) e um item na barra lateral que
some sem jogos. Jogos lidos **sem login, sem rede e sem o cliente aberto**; abrir **sempre** pelo protocolo ou pelo
cliente oficial (nunca o executável do jogo, para não contornar login, DRM, nuvem ou atualização); fim da sessão pelos
processos na pasta do jogo (`InstallPath`). As chaves de desinstalação do Windows (`Uninstall`, 64 e 32 bits, máquina
e usuário) ficam em `UninstallEntries`; caminhos do Windows são tratados como texto (`WindowsPath`) para os testes
rodarem no Mac.

9. **Ubisoft Connect** (`Library/Ubisoft`, STORE-005):
   - Jogos: `HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\<id>` (`InstallDir`, às vezes com `/`; a chave sem
     `WOW6432Node` é plano B). Id não numérico ou pasta inexistente ficam de fora. Nome: `DisplayName` da chave de
     desinstalação `Uplay Install <id>`; sem ela, o nome da pasta (o cache `configurations` do cliente é binário e muda
     de formato; não foi usado).
   - Abrir: `uplay://launch/<id>/0`. Cliente: protocolo `HKCR\uplay` ou `UbisoftConnect.exe`/`upc.exe` na pasta
     `InstallDir` de `HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher`.
10. **EA app** (`Library/Ea`, STORE-006):
    - Jogos: pastas de `HKLM\SOFTWARE\WOW6432Node\EA Games\*` e `...\Electronic Arts\*` (`Install Dir`) e das chaves
      de desinstalação com editor "Electronic Arts" (`InstallLocation`). Só entra a pasta com
      `__Installer\installerdata.xml`, que dá os ids de conteúdo (`contentIDs/contentID`) e o nome (`gameTitle` ou
      `localeInfo/title`, pt_BR, depois en_US). O `C:\ProgramData\EA Desktop\...\IS` é criptografado e não foi usado.
    - Abrir: `origin2://game/launch?offerIds=<ids>&autoDownload=1`. **Incerteza documentada:** o EA app herdou o
      protocolo do Origin e a comunidade (Playnite, atalhos manuais) usa os ids de conteúdo do `installerdata.xml`
      nele; o `link2ea://launchgame/<id>` que a Steam usa espera o id de **oferta** da loja, que não está no disco. Se
      o EA app deixar de aceitar `origin2` com ids de conteúdo, só esta fonte muda. Validar no checklist §26.
    - Cliente: comando de `HKCR\origin2` ou `HKCR\link2ea` apontando para o EA app (`EADesktop`/`EALauncher`/
      `EALaunchHelper`).
11. **Battle.net** (`Library/BattleNet`, STORE-007):
    - Jogos: `%ProgramData%\Battle.net\Agent\product.db` (protobuf) lido por um decodificador mínimo próprio, sem
      pacote: só `product_installs` → `uid`, `product_code`, `settings.install_path` e
      `cached_product_state.base_product_state.installed`; campos desconhecidos são pulados. Arquivo ausente ou
      ilegível → chaves de desinstalação com `--uid=<uid>` no comando (sem "Test"/"Beta").
    - O disco só tem o uid ("prometheus"); o código de abrir ("Pro") e o nome vêm de `BattleNetProducts` (tabela
      adaptada das extensões do Playnite, MIT). O uid casa exato ou com sufixo de idioma ("diablo3_enus"); variações
      ("wow_classic", "wow_beta") não herdam o código para não abrir o jogo errado.
    - Abrir: `Battle.net.exe --exec="launch <código>"`; jogo fora da tabela abre só o Battle.net, para o usuário
      escolher (STORE-011). Cliente: chave de desinstalação `--uid=battle.net` (`InstallLocation`) ou
      `%ProgramFiles(x86)%\Battle.net\Battle.net.exe`. Sem cliente, o alvo vira `battlenet://<código>`, que falha ao
      abrir e mostra o aviso.
12. **Amazon Games** (`Library/Amazon`, STORE-008): SQLite
    `%LOCALAPPDATA%\Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite`, tabela `DbSet` (`Id`, `ProductTitle`,
    `InstallDirectory`, `Installed = 1`), aberto com `Mode=ReadOnly` pelo `Microsoft.Data.Sqlite` que o Core já usa.
    Banco ilegível vira `IOException` (loja vazia com aviso). Abrir: `amazon-games://play/<id>`. Cliente: protocolo
    `HKCR\amazon-games` ou `Amazon Games\App\Amazon Games.exe`.
13. **Cliente ausente ou desatualizado** (STORE-011/012): ao falhar a abertura de um jogo de loja que exige cliente
    (Epic, Ubisoft, EA, Battle.net, Amazon), o aviso da última leitura aparece com o nome do jogo
    (`LibraryText.ClientMissing`); "Abrindo … pelo EA app" diz por onde abriu (`LibraryText.LaunchVia`). **Desatualizado**
    só é detectado onde há um critério confiável: o EA app, quando o protocolo `origin2` pertence só ao **Origin**
    (desligado pela EA em 2025) → "Só o Origin está instalado e ele não abre mais os jogos: instale o EA app". Nenhuma
    das outras lojas publica uma versão mínima do cliente para o protocolo usado, então para elas o aviso é só de
    ausência (ler a versão do exe sem um mínimo conhecido não diria nada útil).
14. **Importar do Playnite** (`Library/Playnite`, STORE-009), em Configurações › Biblioteca e lojas:
    - **Consentimento**: a linha abre um diálogo que diz a pasta, o que será lido e que nada muda no Playnite; só "Ler
      biblioteca" lê. Depois vem a **prévia** (novos, completados, coleções criadas, ignorados) e só "Importar" grava.
      Importação única: nada fica ligado ao Playnite depois.
    - **Onde**: `DatabasePath` do `%APPDATA%\Playnite\config.json` (sem `{PlayniteDir}`), senão
      `%APPDATA%\Playnite\library`. Arquivos `games.db`, `platforms.db`, `categories.db`, `sources.db`.
    - **Formato sem pacote novo**: o Playnite usa o LiteDB **4** (arquivo "v7", páginas de 4 KB). `LiteDbV7Reader` lê os
      documentos pelo índice `_id` (página de coleção → páginas de índice → blocos de dados e páginas de extensão) e
      um leitor BSON mínimo, adaptados do leitor de upgrade do LiteDB 5 que o próprio Playnite embute (MIT, NOTICE.md).
      O arquivo é aberto com `FileShare.ReadWrite` e copiado para a memória: nunca é escrito nem bloqueado. Ler com o
      pacote LiteDB 5 exigiria `Upgrade=true`, que reescreve o arquivo (só numa cópia), e uma dependência nova para ler
      poucos campos; o leitor próprio tem ~300 linhas e testes com arquivos gerados no formato v7. LiteDB 5 ou outro
      formato (Playnite futuro) → "formato não reconhecido", sem tocar no arquivo.
    - **Sem duplicar**: jogo de plugin de loja (ids fixos dos plugins oficiais) casa pelo id da loja (`steam:<appid>`,
      `epic:<AppName>`, `gog:<id>`, `ubisoft:<id>`, `amazon:<id>`; Xbox/Battle.net/EA pelo `StoreProductId`), depois
      pela pasta de instalação e, entre os "Adicionados", pelo nome. Jogo de loja que o Tako lê mas não achou (não
      instalado ou loja desligada) **não** é importado: aparece pela própria loja quando instalado.
    - **O que entra**: jogos instalados com ação de arquivo ou link (`{InstallDir}` resolvido; outra variável do
      Playnite ou script = ignorado) viram "Adicionados" com id fixo `manual:playnite-<guid>` (importar de novo não
      duplica); jogos de emulador entram como ROM com o sistema pelo `SpecificationId` da plataforma (abrem quando a
      emulação chegar). Favorito (ou), tempo jogado e "jogado por último" (o maior; o tempo da Steam não muda, ela soma
      o próprio) valem também para os jogos que o Tako já tinha. Categorias viram coleções (criadas se não existem).
15. **Cores**: `store-ubisoft` `#2E3192`, `store-ea` `#D9302B`, `store-battlenet` `#0B74DE`, `store-amazon` `#B35C00`
    (contraste com texto branco ≥ 4,5:1). Ícones das lojas: Simple Icons (CC0).

## Consequências

- Epic e GOG aparecem sem login, sem rede e sem os clientes abertos. Jogos **não instalados** das contas (biblioteca
  completa) não aparecem: exigiria login nas APIs das lojas, fora do escopo.
- STORE-012 cobre **ausência** do cliente em todas as lojas e **desatualizado** só no EA app (Origin no lugar do EA
  app); as demais lojas não publicam uma versão mínima local confiável.
- STORE-011: Epic, Ubisoft, EA, Battle.net e Amazon sempre abrem pelo cliente; GOG só pelo Galaxy quando há Galaxy;
  Battle.net abre o próprio cliente quando não conhece o jogo. Instalar jogos pelas lojas continua fora.
- O formato dos manifests da Epic, as chaves do GOG, da Ubisoft e da EA, o `product.db` do Battle.net, o banco do
  Amazon Games e o arquivo do Playnite não são documentados publicamente; mudanças quebram só a fonte delas (falha
  isolada, aviso nas Configurações, log). Validação real no checklist de hardware §15 (Xbox), §17 (Epic e GOG) e §25
  (Ubisoft, EA, Battle.net, Amazon e Playnite).
