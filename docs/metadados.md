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
| IGDB (Twitch) | Client ID + secret | jogos fora da Steam, consoles, notas da crítica | planejado |
| SteamGridDB | chave gratuita | capas, fundos e logos da comunidade para qualquer jogo | planejado |

O Metacritic não tem API pública, e raspar o site viola os termos de uso. Por isso a nota vem pela Steam Store, que a publica com autorização.

## Como funciona
1. **Correspondência**: jogos da Steam usam o próprio appid. Jogos adicionados, do Xbox, da Epic e do GOG são buscados pelo título (os que também estão na Steam herdam o appid de lá) (nome do produto no executável ou nome do arquivo), e o `TitleMatcher` escolhe o resultado mais parecido (ignora acentos, ™/®, "Edition", "GOTY" e algarismos romanos). Abaixo de 70% de semelhança, nada é aplicado (o usuário pode corrigir).
2. **Cache**: `%LOCALAPPDATA%\Tako\cache\metadata\<jogo>.json`. É aplicado na hora em que o jogo aparece e atualizado em segundo plano depois de 30 dias (7 dias quando não houve correspondência), um jogo a cada 1,5 s, por causa do limite da API.
3. **Artes**: `steam-art://<appid>/cover|hero` resolve, nesta ordem, os arquivos 2x informados pela API, o caminho antigo da CDN e o cache local da Steam (`appcache/librarycache`). O resultado vai para `%LOCALAPPDATA%\Tako\cache\art`. A capa escolhida pelo usuário nunca é trocada.
4. **Corrigir correspondência** (Detalhes › Opções): mostra os 5 primeiros resultados da busca. A escolha fica fixada e não é trocada nas atualizações automáticas.
