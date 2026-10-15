# Metadados e artes dos jogos

O Tako preenche as informações dos jogos sozinho, como os "agentes" do Plex fazem com filmes.

## De onde vêm os jogos
- **Steam**: o Tako localiza a Steam pelo registro, lê `steamapps/libraryfolders.vdf` (todas as pastas de biblioteca) e os `appmanifest_*.acf`. O tempo jogado e a última vez vêm de `userdata/<conta>/config/localconfig.vdf`. Não precisa da Steam aberta nem de login.
- **Adicionados**: qualquer executável, atalho (`.lnk`/`.url`) ou script, pelo botão "Adicionar jogo" (Início vazia, Biblioteca ou Configurações).
- O estado do usuário (favoritos, última vez aberto pelo Tako, jogos adicionados) fica em `%LOCALAPPDATA%\Tako\library.json`.

## Provedores (`IMetadataProvider`)
| Provedor | Chave | Traz | Status |
|---|---|---|---|
| Steam Store (`SteamStoreProvider`) | não precisa | descrição em português, gêneros, lançamento, desenvolvedora, publicadora, nota do Metacritic; artes 2x (capa 600×900, fundo 3840×1240) | ativo |
| IGDB (Twitch) | Client ID + secret | jogos fora da Steam, consoles, notas da crítica | planejado |
| SteamGridDB | chave gratuita | capas, fundos e logos da comunidade para qualquer jogo | planejado |

O Metacritic não tem API pública, e raspar o site viola os termos de uso. Por isso a nota vem pela Steam Store, que a publica com autorização.

## Como funciona
1. **Correspondência**: jogos da Steam usam o próprio appid. Jogos adicionados são buscados pelo título (nome do produto no executável ou nome do arquivo), e o `TitleMatcher` escolhe o resultado mais parecido (ignora acentos, ™/®, "Edition", "GOTY" e algarismos romanos). Abaixo de 70% de semelhança, nada é aplicado (o usuário pode corrigir).
2. **Cache**: `%LOCALAPPDATA%\Tako\cache\metadata\<jogo>.json`. É aplicado na hora em que o jogo aparece e atualizado em segundo plano depois de 30 dias (7 dias quando não houve correspondência), um jogo a cada 1,5 s, por causa do limite da API.
3. **Artes**: `steam-art://<appid>/cover|hero` resolve, nesta ordem, os arquivos 2x informados pela API, o caminho antigo da CDN e o cache local da Steam (`appcache/librarycache`). O resultado vai para `%LOCALAPPDATA%\Tako\cache\art`. A capa escolhida pelo usuário nunca é trocada.
4. **Corrigir correspondência** (Detalhes › Opções): mostra os 5 primeiros resultados da busca. A escolha fica fixada e não é trocada nas atualizações automáticas.
