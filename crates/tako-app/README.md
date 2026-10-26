# Serviços da ponte

O contrato público está em `src/protocol.rs`; `cargo test -p tako-app` exporta os tipos para
`web/src/bridge/generated/`. O hospedeiro usa `parse_request`, `AppService::handle` e `Response`.

## Preferências e organização (WEB-01b)

- `settings.get`: payload `{}` lê as preferências públicas persistidas; `keys` limita as chaves retornadas.
  Chave permitida ainda sem valor não aparece no mapa `values`; o consumidor aplica seu padrão.
- `settings.set`: recebe `key` e `value` em texto, como no armazenamento do núcleo. A lista positiva de
  `settings.rs` permite preferências de apresentação, idioma, áudio, tela de jogo e comportamento.
  Chaves desconhecidas ou reservadas são `bad_request`. Segredos e o JSON interno `SmartCollections`
  não são acessíveis por estas chamadas. Booleanos usam `true`/`false`.
- `collections.list`: retorna manuais (`criteria: null`, `gameIds` persistidos) e automáticas
  (`criteria` preenchido, `gameIds` vazio). Para obter os jogos de qualquer coleção, consulte
  `library.list` com seu **id**, nunca com seu nome.
- `collections.create`, `collections.rename`, `collections.delete`: criam uma manual, renomeiam ou
  excluem. Renomear e excluir também aceitam ids de automáticas.
- `collections.add_games` e `collections.remove_games`: mudam a participação nas manuais, de forma
  idempotente, recebendo `collectionId` e `gameIds`.
- `collections.smart.set`: sem `collectionId` cria uma automática; com id substitui nome e critérios.
  `criteria.library` reutiliza os filtros da biblioteca. `genres`, `playTime`, `playedOnly`,
  `brokenOnly`, `favoritesOnly` e `collections` completam os critérios do núcleo.
  `filterStore` preserva o filtro de loja das coleções legadas e exige `library.filter: all`.
  Paginação não faz parte da coleção salva: `offset` deve ser zero e `limit` é ignorado.
- `tags.list`: tags em uso, com nome canônico e contagem de jogos.
- `game.tags.set`: `gameIds`, `tags` e `on`; aplica ou remove as tags indicadas, preservando as demais.
- `game.status.set`: `gameIds` e `status`; `null` remove o status.
- `game.favorite.set`: `gameIds` e `favorite`; define o valor, sem alterná-lo ao repetir o pedido.

As coleções automáticas são reavaliadas no núcleo em cada consulta, inclusive após mudanças em lote.
Filtros da consulta se intersectam com os salvos. A ordenação padrão da consulta usa a ordenação salva
quando `sort` é `recently_played` e `reversed` é falso. Id de coleção ausente em `library.list` continua
retornando página vazia, como no contrato anterior; comandos sobre alvos ausentes retornam `not_found`.

Limites: mensagem de 1 MiB; 200 ids por comando, com até 128 caracteres cada; 64 tags por comando;
nomes de coleção e tag com até 40 caracteres (regra do núcleo); 64 chaves por consulta de preferências e
4096 bytes por valor. Critérios mantêm os limites de `library.list`, mais 64 gêneros de até 128 caracteres
e 64 ids de coleções. Automáticas: até 256 coleções e 256 KiB no JSON persistido.

Os ids de jogos são conferidos antes de iniciar o lote. Cada comando de organização concluído emite
`library.changed`. Falhas de armazenamento retornam `internal`; os serviços existentes do núcleo não
garantem transação para o lote inteiro. Se houver mudança em memória antes da falha, o evento também é
emitido. A demonstração usa os mesmos serviços com armazenamento em memória, sem abrir o banco nem o cofre.
