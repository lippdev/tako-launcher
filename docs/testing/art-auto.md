# Capas certas para a biblioteca inteira (ART-001)

As fontes ligadas casam cada jogo sozinhas (ids da loja primeiro, depois o nome) e agora cobrem os quatro tipos de
imagem, sem o usuário escolher:

- **Jogos da Steam** (ou casados com um app da Steam): capa, fundo, **logo e ícone** pelo appid. Se a Steam não
  tiver o logo, a tela mostra o nome, como antes.
- **Outros jogos**, com "Completar capas pelo SteamGridDB" ligado: o SteamGridDB entra quando falta qualquer um dos
  quatro tipos (antes, só sem capa) e traz o melhor logo e ícone estáticos também. A capa e o fundo de fontes de
  maior prioridade (IGDB, ScreenScraper) continuam valendo.
- Cada fonte anota se achou o jogo **só pelo nome** (`MatchedByTitle` no cache).

**Configurações › Metadados › Revisar capas** lista só o que vale conferir: jogos sem capa primeiro, depois os
achados só pelo nome com outro nome (o mais diferente primeiro), depois os sem fundo, logo ou ícone. Cada jogo
abre **Escolher imagens** (seletor), **Corrigir correspondência** ("Fontes e IDs") e, na dúvida, **Está certo**
(fixa o id achado; o jogo sai da lista). **Buscar de novo para a biblioteca inteira** refaz tudo em segundo plano.
Caches antigos não têm a anotação: a dúvida pelo nome aparece depois de buscar de novo.

## Teste manual pendente

- Biblioteca real com Steam e outras lojas: depois de "Buscar de novo", jogos da Steam mostram o logo na home e
  nos detalhes (ou o nome, se a Steam não tiver), e o ícone na Central.
- Com chave do SteamGridDB e "Completar capas" ligado: jogos de Epic/GOG/manuais ganham logo e ícone.
- Abrir "Revisar capas": a contagem e a ordem fazem sentido; "Está certo" tira o jogo da lista e ele não volta
  depois de buscar de novo; "Corrigir correspondência" e "Escolher imagens" abrem as telas de sempre.
