# Metadados e artes dos jogos

O Tako preenche as informações dos jogos sozinho, como os "agentes" do Plex fazem com filmes.

## De onde vêm os jogos
- **Steam**: o Tako localiza a Steam pelo registro, lê `steamapps/libraryfolders.vdf` (todas as pastas de biblioteca) e os `appmanifest_*.acf`. O tempo jogado e a última vez vêm de `userdata/<conta>/config/localconfig.vdf`. Não precisa da Steam aberta nem de login.
- **Xbox / PC Game Pass** (STORE-004, ligável em Configurações › Lojas): pastas de jogos do app Xbox em cada unidade (`.GamingRoot` → `<pasta>\<jogo>\Content\MicrosoftGame.config`) e pacotes MSIX/AppX do usuário que são jogos (`MicrosoftGame.config`, `xboxservices.config` ou registrados pelos Serviços de Jogos). Lê só arquivos e o registro (sem WinRT nem administrador); o jogo abre pelo AUMID (`shell:AppsFolder\<família>!<app>`). Nome e logos do manifesto do pacote ficam como plano B: os dados e artes vêm do agente, casados pelo nome, como nos adicionados.
- **Epic Games**: manifests `*.item` em `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests` (e `LauncherInstalled.dat` para jogos movidos). Abre sempre pelo launcher da Epic ([ADR-0019](adr/0019-fontes-de-biblioteca.md)).
- **GOG**: registro `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games` e `goggame-<id>.info` na pasta do jogo (Galaxy ou instalador offline). Abre pelo GOG Galaxy quando instalado, senão pelo próprio jogo.
- Cada loja pode ser desligada em Configurações; um jogo instalado em duas lojas aparece duas vezes, cada item com a sua loja, ligados ("Também em").
- **Adicionados**: qualquer executável, atalho (`.lnk`/`.url`) ou script, pelo botão "Adicionar jogo" (Início vazia, Biblioteca ou Configurações).
- O estado do usuário (favoritos, última vez aberto pelo Tako, jogos adicionados) fica no banco SQLite `%LOCALAPPDATA%\Tako\tako.db` (MVP-005), junto com as configurações; um `library.json` antigo é importado na primeira abertura e renomeado para `library.json.importado`. Logs de diagnóstico ficam em `logs\tako-AAAAMMDD.log` (7 dias, MVP-033).

## Provedores (`IMetadataProvider`)
| Provedor | Chave | Traz | Status |
|---|---|---|---|
| Steam Store (`SteamStoreProvider`) | não precisa | descrição em português, gêneros, lançamento, desenvolvedora, publicadora, nota do Metacritic; artes 2x (capa 600×900, fundo 3840×1240) | ativo |
| IGDB (`IgdbProvider`, [ADR-0021](adr/0021-metadados-igdb.md)) | Client ID + Client Secret da Twitch **do próprio usuário** | jogos de qualquer loja e de consoles: resumo (inglês), gêneros, temas, modos, lançamento, desenvolvedora, publicadora, nota da crítica, capa, arte de fundo, franquia; ids em outras lojas | ativo quando configurado |
| ScreenScraper | conta do usuário | ROMs por hash, caixa 2D/3D, wheel, marquee, vídeo | planejado |
| SteamGridDB | chave gratuita | capas, fundos e logos da comunidade para qualquer jogo | planejado |

O Metacritic não tem API pública, e raspar o site viola os termos de uso. Por isso a nota vem pela Steam Store, que a publica com autorização.

## IGDB com a sua chave da Twitch
O Tako é open source, então não traz chave nenhuma: cada pessoa cria a sua, de graça, em Configurações › **Dados dos jogos (IGDB)**:
1. Entrar em [dev.twitch.tv/console](https://dev.twitch.tv/console) (a conta precisa da verificação em duas etapas).
2. "Registrar seu aplicativo": qualquer nome, URL de redirecionamento OAuth `http://localhost`, categoria "Application Integration", cliente Confidencial.
3. Em "Gerenciar", copiar o **Client ID**; em "Novo segredo", copiar o **Client Secret**. Colar nos campos (botão "Colar" funciona com o controle), **Testar** e **Salvar**.

A chave fica cifrada com DPAPI em `%LOCALAPPDATA%\Tako\igdb.dat` e nunca volta para a tela. Sem chave (ou desligada), a IGDB não é consultada e tudo segue só com a Steam. Detalhes, limites e a alternativa descartada (proxy) na [ADR-0021](adr/0021-metadados-igdb.md).

## Como funciona
1. **Correspondência**: primeiro por id, sem ambiguidade — jogos da Steam pelo appid; com a IGDB ligada, jogos da Steam, do GOG (id do jogo), da Epic (CatalogItemId) e do Xbox (StoreId) pelos `external_games` da IGDB, e a IGDB informa o appid da Steam do mesmo jogo, que é consultada por ele. Sem id, a busca é pelo título (nome do produto no executável ou nome do arquivo), filtrada pela plataforma na IGDB (PC ou o console do emulador), e o `TitleMatcher` escolhe o resultado mais parecido (ignora acentos, ™/®, "Edition", "GOTY" e algarismos romanos). Abaixo de 70% de semelhança, nada é aplicado (o usuário pode corrigir).
2. **Combinação** (`MetadataMerge`), campo a campo, pela ordem do tipo de jogo: jogos da Steam usam a Steam e completam com a IGDB (temas, modos, série, nota da crítica); jogos de outras lojas e adicionados usam a IGDB primeiro e completam com a Steam; jogos de emulador usarão o ScreenScraper, depois a IGDB. A descrição prefere português (a IGDB só tem inglês); a arte de jogos de PC vem da Steam quando há appid (maior), senão da IGDB.
3. **Cache**: `%LOCALAPPDATA%\Tako\cache\metadata\<jogo>.json`, com a parte de cada fonte. É aplicado na hora em que o jogo aparece e atualizado em segundo plano depois de 30 dias (7 dias quando não houve correspondência), um jogo a cada 1,5 s, por causa do limite da Steam. Ligar a IGDB consulta só a IGDB para os jogos que ainda não passaram por ela. As respostas da IGDB ficam em `cache\igdb` (30 dias); a biblioteca inteira é resolvida em poucas consultas em lote, no máximo 4 por segundo, esperando e tentando de novo quando a IGDB pede (429).
4. **Artes**: `steam-art://<appid>/cover|hero` resolve, nesta ordem, os arquivos 2x informados pela API, o caminho antigo da CDN e o cache local da Steam (`appcache/librarycache`). Artes da IGDB são https (`images.igdb.com`, capa `t_cover_big`, fundo `t_1080p` ou `t_screenshot_huge`). O resultado vai para `%LOCALAPPDATA%\Tako\cache\art`. A capa escolhida pelo usuário nunca é trocada.
5. **Corrigir correspondência** (Detalhes › Opções): mostra até 4 resultados de cada fonte ligada, com a fonte, o ano e a plataforma ("Hades (2020) · IGDB"); ✓ marca o casado pelo id da loja. A escolha fica fixada (e traz os dados das outras fontes pelos ids) e não é trocada nas atualizações automáticas.
6. **Aviso**: com a IGDB desligada, um jogo de fora da Steam sem descrição mostra nos detalhes uma linha discreta sugerindo ligá-la.
