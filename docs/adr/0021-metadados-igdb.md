# ADR-0021: Metadados pela IGDB com a chave da Twitch de cada usuário e combinação de N provedores

- Status: Proposta
- Data: 2026-10-15

## Contexto

Os dados e as artes dos jogos vinham só da loja Steam (`SteamStoreProvider`, sem chave), casados pelo appid ou pelo
nome. Jogos da Epic, do GOG, do Xbox, adicionados à mão e, em breve, de emuladores (EMU-007) ficam sem dados quando não
existem na Steam. O PRD pede provedores licenciados (SOURCE-017). A IGDB (api.igdb.com v4, da Twitch) cobre jogos de
qualquer loja e de consoles, com gêneros, temas, modos, empresas, notas, capa, artes, franquia e ids em outras lojas.

A IGDB exige um app da Twitch: Client ID + Client Secret → token OAuth `client_credentials` em
`https://id.twitch.tv/oauth2/token`. O Tako é open source (AGPL): um segredo no repositório ou no executável seria
público. Limite da API: 4 consultas por segundo (429 acima disso), até 500 itens por consulta.

Depois da IGDB virá o ScreenScraper (ROMs por hash, mídia de emulação). A combinação e a escolha de fonte precisam
servir para N provedores.

## Decisão

1. **Autenticação: cada usuário usa a própria chave da Twitch.** Client ID e Client Secret são informados em
   Configurações › Dados dos jogos (IGDB) e guardados em `%LOCALAPPDATA%\Tako\igdb.dat`, o arquivo inteiro cifrado com
   DPAPI (`IgdbSettingsStore` + `ISecretProtector`, como as chaves das TVs). O `TwitchAppAuthenticator` obtém o token,
   guarda só na memória e renova 5 min antes de vencer, quando a chave muda ou quando a IGDB responde 401. A
   autenticação fica atrás de `IIgdbAuthenticator` (testes e um modo futuro sem mexer no cliente). Sem chave, ou com a
   IGDB desligada, o provedor fica desativado e tudo segue só com a Steam, sem erro e sem nenhuma consulta.
2. **Alternativa descartada: proxy do Tako.** Um servidor dos mantenedores guardaria o segredo e aceitaria as mesmas
   consultas (`POST {base}/v4/{endpoint}` com o corpo Apicalypse, cabeçalho de versão do Tako, cache e 4 req/s no
   servidor). Descartado pelo mantenedor: exige hospedagem e custo contínuos, concentra o limite de 4 req/s de todos os
   usuários numa chave só, cria um ponto único de falha e de abuso e põe os mantenedores no caminho dos termos da
   Twitch para terceiros. Se voltar a ser necessário, entra como outra implementação de `IIgdbAuthenticator` (o cliente
   já separa autenticação de consulta).
3. **Provedores genéricos.** `IMetadataProvider` passa a ser independente da Steam: `Name`, `IsEnabled`,
   `SearchAsync(MetadataLookup)`, `FindExactAsync(MetadataLookup)` (casamento por id, sem ambiguidade) e
   `FetchAsync(string id)`. `MetadataLookup` leva título, plataforma (nula = PC) e ids em lojas (`Steam`, `GOG`,
   `Epic` = CatalogItemId, `Microsoft` = StoreId). `MetadataMatch` diz a fonte. Provedores que resolvem muitos jogos de
   uma vez implementam `IBatchMetadataProvider.PrefetchAsync`.
4. **Agente (`MetadataAgent`, Core, testado).** Para cada jogo, na ordem do tipo: id já conhecido → `FindExactAsync`
   (ids da loja e ids que outra fonte informou: a IGDB devolve o appid da Steam em `external_games`, e a Steam é
   consultada pelo appid, sem busca por nome) → busca pelo título (`TitleMatcher`, ≥ 70%). Cada parte é guardada no
   cache do jogo (`Parts`) com as fontes consultadas (`Consulted`): ligar a IGDB depois consulta só a IGDB, aos poucos,
   sem refazer a Steam. Falha passageira (429, fora do ar) não marca a fonte como consultada. Cache antigo (só Steam)
   vira uma combinação de uma parte.
5. **Prioridade por tipo de jogo, para N fontes (`MetadataPriority`)**: jogo da Steam → Steam, IGDB, ScreenScraper;
   outro jogo de PC (Epic, GOG, Xbox, adicionado) → IGDB, Steam, ScreenScraper; jogo de emulador (com `Game.Platform`)
   → ScreenScraper, IGDB, Steam. Provedores fora da lista vão para o fim. Exceções por campo: **descrição** prefere
   português em qualquer fonte (a IGDB não traduz resumos; a Steam traduz quando o jogo tem português); **arte** de
   jogo de PC vem da Steam quando há appid (capa 600×900 e fundo 3840×1240, maiores que os da IGDB), senão da IGDB.
   Capa e fundo vêm sempre da mesma fonte. A capa escolhida pelo usuário nunca é trocada.
6. **Consultas (IgdbQueries/IgdbParser, puros e testados com respostas escritas à mão).** Busca com
   `search "…"; where version_parent = null & platforms = (…)`, filtrada pela plataforma (PC = 6; consoles mapeados em
   `IgdbPlatforms`). Campos expandidos numa consulta só: nome, resumo, gêneros, temas, modos, lançamento,
   `involved_companies` (desenvolvedora/publicadora), `aggregated_rating` (senão `total_rating`), `cover`,
   `artworks`/`screenshots`, franquia/série e `external_games`. Nome e capa da localização Brasil quando houver.
   Imagens: capa `t_cover_big`, fundo `t_1080p` (artworks) ou `t_screenshot_huge` (capturas). Gêneros, temas e modos
   são traduzidos para português por uma tabela fixa. Ids em lojas pelo `external_game_source` (1 Steam, 5 GOG,
   11 Microsoft, 26 Epic), que substituiu o `category`.
7. **Limites.** `IgdbClient`: uma consulta por vez (fila), no máximo 4 por segundo (`RateGate`), backoff exponencial
   no 429 (1 s, 2 s, 4 s ou o `Retry-After`, até 4 tentativas), troca de token no 401, cache de respostas em
   `cache\igdb` por 30 dias. Lote: antes de cada rodada, `external_games` de todos os jogos com id de loja (uma consulta
   por loja) e `games where id = (…)` a cada 500 jogos; a rodada jogo a jogo usa o que já está na memória. O espaço de
   1,5 s entre jogos continua, pelo limite da Steam.
8. **Configurações.** Uma linha isolada "Dados dos jogos (IGDB)" (estado: desligada / ligada · sua chave / falta a
   chave) que abre uma sobreposição própria (`MetadataViews.xaml`): liga/desliga, passo a passo curto (dev.twitch.tv
   › registrar app com redirecionamento `http://localhost` e categoria Application Integration › copiar Client ID ›
   novo segredo), botão para abrir o site, campos com "Colar" (para quem usa só o controle), "Testar" (token + consulta
   mínima, sem salvar) com mensagem clara por motivo, Salvar e Cancelar. O segredo salvo nunca volta para a tela. A
   reorganização das Configurações pode mover a linha para "Biblioteca e lojas" sem tocar na lógica. Nos detalhes, um
   jogo de fora da Steam sem descrição mostra um aviso discreto sugerindo ligar a IGDB; "Corrigir correspondência" lista
   resultados de todas as fontes ligadas (até 4 por fonte), com a fonte, o ano e a plataforma, e marca com ✓ os casados
   por id.
9. **Atribuição.** "Dados e imagens fornecidos pela IGDB.com" na sobreposição da IGDB e no `NOTICE.md`, como pedem os
   termos da IGDB/Twitch; o Tako não usa a marca da IGDB além do nome.

## Consequências

- Nenhum segredo no repositório nem no binário; cada usuário respeita o próprio limite de 4 req/s. Em troca, todo
  usuário que quiser a IGDB passa pelo cadastro na Twitch (conta com verificação em duas etapas) — por isso o passo a
  passo na tela e o botão "Testar".
- Sem IGDB, nada muda para quem só tem a Steam. Com IGDB, a primeira rodada de uma biblioteca grande custa poucas
  consultas em lote; o resto vem do cache.
- O ScreenScraper entra como mais um `IMetadataProvider` (e, se precisar, `IBatchMetadataProvider`), sem mexer na
  combinação: a ordem para ROMs já o coloca primeiro.
- A validar com conta real (checklist, seção 21): nomes atuais dos campos (`external_game_source`), o formato do uid
  da Epic e da Microsoft Store em `external_games`, o identificador da região Brasil em `game_localizations` e se
  `t_cover_big` (264×374) fica bom na TV — se não, trocar para `t_cover_big_2x` em `IgdbParser.CoverSize`.
