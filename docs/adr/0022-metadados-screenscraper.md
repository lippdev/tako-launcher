# ADR-0022: ScreenScraper para jogos de emulador (ROMs), com a conta de cada usuário e as credenciais do software só no build de release

- Status: Histórico (implementação C#). A decisão vale para o Rust (MIG-028)
- Data: 2026-10-15

## Contexto

A [ADR-0021](0021-metadados-igdb.md) deixou a combinação de N provedores pronta, com o ScreenScraper reservado em
primeiro lugar para jogos de emulador (`Game.Platform` preenchido), mas ninguém preenchia a plataforma e não havia
provedor de ROMs. O ScreenScraper (api.screenscraper.fr, API v2) reconhece a ROM pelo hash do arquivo e tem a melhor
mídia de emulação (caixa 2D/3D, logo "wheel", marquee, capturas, fanart, vídeo), com nomes e datas por região e sinopse
em vários idiomas, incluindo português.

A API pede dois pares de credenciais em toda consulta:

- **do software** (`devid`, `devpassword`, `softname`), que o ScreenScraper dá ao projeto. Os mantenedores pediram e
  ainda não chegaram. O Tako é open source (AGPL): elas não podem ficar no repositório.
- **do usuário** (`ssid`, `sspassword`), a conta gratuita de cada pessoa em screenscraper.fr. A cota diária e o número
  de consultas simultâneas (`maxthreads`) são por conta.

Ainda não existe importador de ROMs (épico EMU) nem como abrir um jogo no emulador.

## Decisão

1. **Conta do usuário** em Configurações › Dados de ROMs (ScreenScraper), no mesmo padrão da IGDB: liga/desliga, passo a
   passo (criar conta em screenscraper.fr e confirmar o e-mail, escrever ou colar usuário e senha), "Colar" para quem
   usa só o controle, **Testar** (`ssuserInfos.php`, sem salvar) mostrando o nível da conta, quantas consultas restam
   hoje e quantas podem ir ao mesmo tempo, Salvar e Cancelar. Guardada em `%LOCALAPPDATA%\Tako\screenscraper.dat`, o
   arquivo inteiro cifrado com DPAPI (`ScreenScraperSettingsStore`). A senha salva nunca volta para a tela. É uma linha
   isolada que abre a própria sobreposição (`MetadataViews.xaml`), para a reorganização das Configurações (C1) movê-la
   para "Biblioteca e lojas".
2. **Credenciais do software só no build de release.** Ficam nos segredos do repositório no GitHub:
   `SCREENSCRAPER_DEVID`, `SCREENSCRAPER_DEVPASSWORD` e, opcional, `SCREENSCRAPER_SOFTNAME` (padrão "Tako"). Só o passo
   "Build portable + installer" do `release.yml` os recebe, como variáveis de ambiente. O `build\Publish-Tako.ps1` liga a
   gravação com `-p:TakoScreenScraper=true` (sem pôr os valores na linha de comando); o `.csproj` lê os valores do
   ambiente e os grava como `AssemblyMetadata` do `Tako.exe` (`ScreenScraperSoftware.FromAssembly`). Compilação local e
   CI de PR saem **sem** elas: o provedor fica desligado, a linha das Configurações diz "Indisponível nesta versão", a
   sobreposição mostra "Esta versão do Tako ainda não tem acesso ao ScreenScraper" e deixa salvar a conta para quando a
   versão chegar. Nada quebra. Aceitamos que credenciais embutidas num executável podem ser extraídas por quem
   quiser (é o modelo do ScreenScraper para todos os scrapers); se forem abusadas, os mantenedores pedem outras e só a
   próxima release muda. Valores com `;` ou `%` precisam ser escapados para o MSBuild (`%3B`, `%25`).

   Desde 2026-10-20, o usuário pode informar credenciais de software próprias quando o build não as tiver (guardadas cifradas como a conta). As credenciais do build continuam tendo prioridade quando presentes.
3. **Consultas** (`ScreenScraperClient`, `GET https://api.screenscraper.fr/api2/{endpoint}.php`, `output=json`):
   - por hash: `jeuInfos.php` com `systemeid`, `romtype` (`rom` ou `iso`), `romnom`, `romtaille` e os hashes calculados;
   - por nome (fallback e "Corrigir correspondência"): `jeuRecherche.php` com `systemeid` e `recherche`;
   - por id (correspondência escolhida): `jeuInfos.php?gameid=…`.

   Respostas em erro vêm como texto em francês, às vezes com código 200: são classificadas pelo código HTTP e pelo
   começo do texto (`ScreenScraperClient.Classify`, testado): 404/"non trouvée" = jogo desconhecido (não é erro);
   429/"threads" = limite de consultas simultâneas (espera 1 s, 2 s, 4 s e tenta de novo, até 4 vezes); 430/431/"quota"
   = cota do dia; 401/423 = API fechada ou sobrecarregada; 403 = credenciais do software; "identifiants utilisateur" =
   conta do usuário; 426/"blacklist" = versão bloqueada.
4. **Limites da conta.** No máximo `maxthreads` consultas ao mesmo tempo (1 até a primeira resposta; a conta vem em
   toda resposta, `ssuser`) e uma a cada 1,2 s por vaga. Cota do dia esgotada (`requeststoday ≥ maxrequestsperday`, ou
   430/431) **pausa** o cliente até a meia-noite de Paris (+5 min); servidor fechado pausa 15 min; sem rede, 2 min;
   conta ou software recusados, 12 h ou até a conta mudar. Pausado, o provedor implementa `IPausableMetadataProvider`:
   o agente não o consulta, **mantém** os dados que ele já trouxe e não marca as ROMs como "sem correspondência"
   (senão esperariam 7 dias). Respostas com sucesso ficam em `cache\screenscraper` por 30 dias; a chave do cache nunca
   inclui credenciais.
5. **Hash da ROM** (`RomHasher`, em streaming com buffer de 1 MB, sempre em `Task.Run`, fora da thread de UI) e cache
   por caminho + tamanho + data de modificação em `cache\rom-hashes.json` (`RomHashCache`):
   - até 64 MB (cartuchos): CRC32 + MD5 + SHA1;
   - acima (ISOs): só CRC32, que o ScreenScraper aceita junto com nome e tamanho, sem ler o arquivo três vezes;
   - imagens de disco compactadas (`.chd`, `.rvz`, `.cso`, `.wbfs`, `.pbp`, `.nsz`...) e `.7z`: só nome e tamanho (o
     hash do contêiner não identifica o jogo); a busca por nome cobre o resto;
   - `.zip` de sistema de cartucho com um arquivo só: hashes e nome **do arquivo de dentro** (é o que os dats
     registram); `.zip` do arcade/Neo Geo: o próprio `.zip` (o conjunto do MAME é o arquivo).
6. **Sistemas** (`ScreenScraperSystems`, tabela pura e testada): chave do Tako (a mesma de `IgdbPlatforms`), nome curto,
   `systemeid` e extensões típicas para NES (3), SNES (4), N64 (14), GameCube (13), Wii (16), Wii U (18), Switch (225),
   GB (9), GBC (10), GBA (12), DS (15), 3DS (17), Master System (2), Mega Drive (1), Sega CD (20), 32X (19), Game Gear
   (21), Saturn (22), Dreamcast (23), PS1 (57), PS2 (58), PS3 (59), PSP (61), PS Vita (62), Xbox (32), Xbox 360 (33),
   arcade/MAME (75), Neo Geo (142), PC Engine (31) e Atari 2600 (26).
7. **Campos** (`ScreenScraperParser`): nome por região (br → pt → us → wor → eu → ss → jp → o primeiro); sinopse
   pt → en; gêneros pt, senão en traduzido pela tabela da IGDB; lançamento por região (br → us → wor → eu → ss → jp,
   aceita "1994", "1994-08"); desenvolvedora; editora; jogadores ("1-2", novo campo `Players`); nota de 0 a 20 → 0 a
   100 (`CriticScore`). Mídias por região (br → us → wor → eu → ss → jp): caixa 2D vira a capa; fanart (senão a
   captura) vira o fundo; caixa 3D, logo (`wheel-hd`, senão `wheel`), marquee, captura, fanart e vídeo
   (`video-normalized`, senão `video`) vão para `GameMetadata.Media` / `Game.Media`, só como endereço.
8. **Mídia sem credenciais no disco.** As URLs de mídia da API trazem `devid`, `devpassword`, `ssid` e `sspassword`.
   O parser as guarda como `ssmedia://<host>/<caminho>?<sem credenciais>`, só de hosts `*.screenscraper.fr`. O
   `ArtService` baixa sob demanda (só o que aparece na tela) pelo `ScreenScraperClient.DownloadMediaAsync`, que põe as
   credenciais na hora e respeita o mesmo limite de consultas simultâneas; o arquivo fica no cache de artes, pela URL
   sem credenciais. O vídeo nunca é baixado sozinho: fica o endereço para quando houver onde tocá-lo.
9. **Combinação.** ROMs: ScreenScraper primeiro, IGDB como complemento (busca filtrada pela plataforma, temas, série) e
   Steam por último, como a ADR-0021 já previa; a arte e as outras mídias seguem a mesma ordem. O ScreenScraper **só
   atende ROMs** (`MetadataPriority.Applies`): jogos de PC nunca gastam a cota. "Corrigir correspondência" lista primeiro o
   casado pelo hash (✓) e depois a busca por nome (até 4 por fonte na tela), com o ano e o sistema.
10. **ROMs no "Adicionar jogo" (mínimo, até o épico EMU).** O diálogo aceita ROMs e imagens de disco. Arquivo com
    extensão de ROM abre "De qual sistema é este jogo?": os sistemas prováveis pela extensão primeiro (um `.zip` lista
    arcade, Neo Geo e os de cartucho), "Outro sistema" lista todos. O título vem do nome do arquivo sem as marcações dos
    dats ("Lanterna (USA) [!].sfc" → "Lanterna"). O sistema fica em `manual_games.platform` (migração 5) e vira
    `Game.Platform`; o arquivo vira `Game.RomPath`. **O jogo não abre**: o botão diz "Abrir no emulador (em breve)" e
    explica; abrir pelo emulador, pastas de ROMs e o cadastro de emuladores ficam para o épico EMU. Foi escolhido em vez
    de "só fixtures" porque é pequeno e permite validar o ScreenScraper de ponta a ponta com uma ROM real.
11. **Créditos.** "Dados e mídias fornecidos por ScreenScraper.fr" na sobreposição e no `NOTICE.md`, como o
    ScreenScraper pede.

## Consequências

- Nenhuma credencial no repositório; cada usuário gasta a própria cota. Até as credenciais do software chegarem, as
  releases mostram o ScreenScraper como indisponível e as ROMs ficam só com a IGDB.
- Para os mantenedores: criar os segredos `SCREENSCRAPER_DEVID` e `SCREENSCRAPER_DEVPASSWORD` (e
  `SCREENSCRAPER_SOFTNAME` se o nome registrado não for "Tako") em Settings › Secrets and variables › Actions. O log do
  `Publish-Tako.ps1` diz se a versão saiu com ou sem acesso.
- Capas de console nem sempre são 2:3 (caixas de SNES e Mega Drive são deitadas): a grade recorta como faz com as
  outras artes. Se ficar ruim, usar a caixa 3D ou o logo sobre a captura (fica para o épico EMU, que vai ter telas
  próprias para sistemas).
- A validar com conta real e credenciais do software (checklist, seção 22): textos exatos de erro do ScreenScraper,
  se `romtype=iso` com só CRC acha ISOs grandes, os tipos de mídia `screenmarquee`/`wheel-hd`, e se a mídia baixa pelo
  host `neoclone` com as credenciais acrescentadas.
