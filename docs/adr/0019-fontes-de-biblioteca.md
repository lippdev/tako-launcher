# ADR-0019: Fontes de biblioteca (Steam, Epic Games, GOG, Xbox e as próximas lojas)

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

## Consequências

- Epic e GOG aparecem sem login, sem rede e sem os clientes abertos. Jogos **não instalados** das contas (biblioteca
  completa) não aparecem: exigiria login nas APIs das lojas, fora do escopo.
- STORE-012 cobre **ausência** do cliente; "desatualizado" não é detectado (nem Epic nem GOG publicam uma versão mínima
  local confiável).
- STORE-011: Epic sempre abre pelo cliente; GOG só pelo Galaxy quando há Galaxy. Instalar jogos pelas lojas continua
  fora.
- O formato dos manifests da Epic e as chaves do GOG não são documentados publicamente; mudanças das lojas quebram só
  a fonte delas (falha isolada, aviso nas Configurações, log). Validação real no checklist de hardware §15 (Xbox) e §17 (Epic e GOG).
