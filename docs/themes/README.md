# Temas do Tako: guia e referência da API

[English](README.en-US.md)

Um tema muda a **aparência** do Tako: cores, fontes, disposição das telas e animações. Os dados (jogos, capas,
textos já traduzidos, foco) e a navegação continuam com o Tako. O tema recebe tudo pronto por propriedades e
avisa cliques e teclas por callbacks. Esta página descreve a **API de temas versão 1**.

- Tema padrão (referência completa): [`crates/tako-ui/theme/`](../../crates/tako-ui/theme/)
- Tema de exemplo que só troca cores: [`crates/tako-ui/theme/extras/tako-alto-contraste/`](../../crates/tako-ui/theme/extras/tako-alto-contraste/)
- Tema oficial Fita (console moderno): [`crates/tako-ui/theme/extras/tako-fita/`](../../crates/tako-ui/theme/extras/tako-fita/) e [`fita.md`](fita.md)
- Linguagem dos arquivos de tela: [Slint](https://slint.dev/docs) (o Tako usa o interpretador do Slint 1.x)

## 1. Começo rápido

O jeito mais curto de fazer um tema é reaproveitar o padrão e trocar a paleta:

```
meu-tema/
├── theme.toml
├── main.slint
└── tokens.toml
```

`theme.toml`:

```toml
id = "autor.meu-tema"
name = "Meu tema"
version = "1.0.0"
author = "Seu nome"
api = 1
description = "Fundo azul-marinho."
```

`main.slint`:

```slint
export { App, Ui, Hint, PanelRow } from "@tako/main.slint";
```

`tokens.toml`:

```toml
canvas = "#0B1A33"
surface-1 = "#FFFFFF24"
```

Confira e instale:

```
tako theme check meu-tema
```

Para testar, copie a pasta para `<dados do Tako>/themes/` e escolha o tema em **Configurações › Aparência › Tema**.
Escolher um tema mostra uma prévia: ele aparece inteiro na tela, mas só vira o tema do Tako se você confirmar com
**Aplicar**. **Voltar** ou 15 segundos sem resposta devolvem o tema anterior; se o tema não carregar, o anterior
continua.
Para distribuir, compacte a pasta como `.takotheme` (veja a [seção 7](#7-pacote-e-distribuição)).

## 2. O que um tema pode e não pode fazer

- **Pode:** desenhar todas as telas do jeito que quiser, usar imagens e fontes próprias e animar.
- **Não pode:** acessar a rede, o disco ou outros programas. O Slint não oferece isso, e o Tako só carrega
  arquivos de dentro do pacote. Um tema também não muda o que os botões fazem: a navegação é do Tako.
- **Se quebrar:** o Tako volta ao tema padrão. Três falhas em 10 segundos ao carregar fazem a volta ser
  automática, sem loop de falha.

## 3. `theme.toml`

| Campo | Obrigatório | Regra |
|---|---|---|
| `id` | sim | Letras, números, `.`, `-` e `_`, até 128 caracteres. Não pode começar ou terminar com `.`, nem ser um nome reservado do Windows (`CON`, `NUL`, `COM1`…). `default` é do Tako. Use `autor.tema` para não colidir. |
| `name` | sim | Nome mostrado na lista de temas. |
| `version` | sim | Texto livre, mostrado ao lado do autor (sugestão: SemVer). |
| `author` | sim | Quem fez. |
| `api` | sim | Versão da API: `1` ou `"1.x"`. Só a versão maior conta. Outra versão maior é recusada. |
| `entry` | não | Arquivo `.slint` principal, relativo à raiz. Padrão: `main.slint`. Não pode sair da pasta do tema. |
| `description` | não | Uma frase, mostrada na lista. |

O manifesto tem no máximo 64 KB. Campos desconhecidos são ignorados.

## 4. O componente `App`

O arquivo de entrada precisa **exportar um componente `App`** (normalmente `inherits Window`). O Tako preenche as
propriedades `in` dele pelo nome e liga os callbacks. **Propriedade que o tema não declara é simplesmente
ignorada**: um tema pode começar pequeno e crescer. As unidades de tamanho são do tema. O padrão usa um palco de
1080 linhas (`Ui.u = altura / 1080`).

Para não repetir tipos, importe-os do padrão:

```slint
import { Ui, Hint, PanelRow } from "@tako/main.slint";
import { Tile, HomeRow, NavItem, Hero, Stat, Details, Overlay, Key } from "@tako/main.slint";
```

### 4.1 Gerais

| Propriedade | Tipo | O que é |
|---|---|---|
| `page` | `string` | Página atual: `home`, `library`, `details` ou `settings`. |
| `backdrop` | `image` | Arte de fundo do jogo em foco (pode vir vazia). |
| `clock` | `string` | Hora, já formatada (`21:30`). |
| `input-label`, `input-icon` | `string` | Entrada em uso (nome e ícone: `gamepad` ou `keyboard`). |
| `pad-battery` | `string` | Bateria do controle sem fio (`"80%"`), ou vazio. |
| `toast` | `string` | Aviso curto do momento, ou vazio. |
| `hints` | `[Hint]` | Dicas da barra de baixo (glifo do botão e texto), em ordem. |
| `transparent-background` | `bool` | A janela está por cima do jogo: desenhe só o necessário, com fundo transparente. |
| `leaving` | `bool` | Saindo para o jogo: o Tako fecha depois de `Ui.leave-time`. Ideal para um fade. |
| `intro` | `bool` | Entrada do app: verdadeiro por cerca de 1 s só na abertura, com a primeira tela já pronta por baixo. Cubra-a com uma animação curta e apague ao virar falso. Qualquer tecla, botão ou `clicked` (de qualquer zona) pula a entrada; o tema pode usar a zona `intro` para a área do véu. Nunca fica verdadeiro com "Reduzir animações" nem num tema que não declare a propriedade. |

### 4.2 Barra lateral

| Propriedade | Tipo | O que é |
|---|---|---|
| `sidebar` | `[NavItem]` | Itens (Início, Biblioteca, coleções, Configurações…). |
| `sidebar-focus` | `int` | Item com foco; `-1` = a barra não tem foco. |
| `sidebar-visible` | `bool` | Mostrar a barra. |

### 4.3 Início

| Propriedade | Tipo | O que é |
|---|---|---|
| `hero` | `Hero` | Destaque do topo (jogo em foco ou de continuar). |
| `hero-focus` | `int` | Botão do destaque com foco; `-1` = nenhum. |
| `home-rows` | `[HomeRow]` | Fileiras de capas. |
| `home-row-focus`, `home-col-focus` | `int` | Fileira e coluna com foco (`-1` = fora das fileiras). |
| `home-empty` | `bool` | Biblioteca vazia: mostre o estado vazio. |
| `home-empty-title`, `home-empty-text` | `string` | Textos do estado vazio. |
| `home-empty-actions` | `[PanelRow]` | Botões do estado vazio. |
| `see-all-label` | `string` | Texto de "Ver tudo". |

### 4.4 Biblioteca

| Propriedade | Tipo | O que é |
|---|---|---|
| `library-title`, `library-count` | `string` | Título e contagem. |
| `library-chips` | `[PanelRow]` | Filtros e ordenação (`kind` = `choice` ou `action`). |
| `library-chip-focus` | `int` | Chip com foco; `-1` = foco na grade. |
| `library-tiles` | `[Tile]` | Capas da grade. |
| `library-focus` | `int` | Capa com foco. |
| `library-empty-title`, `library-empty-text` | `string` | Grade vazia. |
| `library-columns` | `out int` | **Saída:** quantas colunas cabem. O Tako usa para mover o foco para cima e para baixo. |

### 4.5 Detalhes

| Propriedade | Tipo | O que é |
|---|---|---|
| `details` | `Details` | Título, textos e artes do jogo. |
| `details-stats` | `[Stat]` | Números (tempo jogado, última vez…). |
| `details-actions` | `[PanelRow]` | Botões (Jogar, Favoritar, Opções…). |
| `details-focus` | `int` | Botão com foco. |

### 4.6 Configurações

| Propriedade | Tipo | O que é |
|---|---|---|
| `settings-title` | `string` | Título da página. |
| `settings-categories` | `[NavItem]` | Categorias. |
| `settings-category-focus` | `int` | Categoria com foco. |
| `settings-heading`, `settings-subtitle` | `string` | Título e descrição da categoria. |
| `settings-rows` | `[PanelRow]` | Linhas da categoria. |
| `settings-row-focus` | `int` | Linha com foco; `-1` = foco nas categorias. |

### 4.7 Central de Controle

| Propriedade | Tipo | O que é |
|---|---|---|
| `control-center` | `bool` | A Central está aberta. |
| `cc-icons` | `[PanelRow]` | Ícones da barra da Central. |
| `cc-icon-focus` | `int` | Ícone com foco. |
| `cc-panel-for` | `int` | Ícone cujo painel está aberto; `-1` = nenhum. |
| `cc-panel` | `[PanelRow]` | Linhas do painel aberto. |
| `cc-panel-focus` | `int` | Linha com foco no painel. |
| `cc-game-title`, `cc-game-meta` | `string` | Jogo em andamento. |

Com `control-center` e `transparent-background` ligados, a Central está **por cima do jogo**: mostre só a barra
dela.

### 4.8 Camadas (diálogos, painéis, teclado)

| Propriedade | Tipo | O que é |
|---|---|---|
| `overlays` | `[Overlay]` | Camadas abertas, de baixo para cima. A última recebe o foco e os cliques. |

`Overlay.kind`:

| `kind` | O que é | Campos usados |
|---|---|---|
| `dialog` | Diálogo central | `title`, `overline`, `text`, `rows`, `sel`, `wide`, `image` |
| `panel` | Painel lateral | `title`, `overline`, `text`, `rows`, `sel` |
| `keyboard` | Teclado virtual | `title`, `input`, `input-hint`, `secret`, `keys`, `key-cols`, `sel` |
| `art` | Seletor de arte | `tabs`, `tab`, `tabs2`, `tab2`, `thumbs`, `thumb-cols`, `preview`, `preview-text`, `sel` |
| `launch` | "Abrindo…" | `title`, `text`, `image` |
| `progress` | Tarefa em andamento | `title`, `text` |
| `recovery` | Tela de recuperação | `title`, `text`, `rows`, `sel` |

`sel = -1` quer dizer "sem linha selecionável" (só leitura).

Desde o THEME-002, um `kind` vazio (ou que o tema não conhece) é uma camada sem desenho, como a prévia de tema: desenhe nada e deixe a
barra de `hints` falar. Nunca cubra a tela com um véu para um `kind` desconhecido.

### 4.9 Callbacks

| Callback | Quando chamar |
|---|---|
| `clicked(zona: string, índice: int)` | Clique ou toque num item. |
| `hovered(zona: string, índice: int)` | Ponteiro entrou num item (o foco segue o mouse). |
| `hint-clicked(índice: int)` | Clique numa dica da barra de baixo (índice em `hints`). |
| `key(texto, repetição, ctrl, shift, alt) -> bool` | Tecla física (`KeyEvent` do Slint). Devolva o resultado: `true` = o Tako tratou. |
| `key-released(texto)` | Tecla solta. |

Zonas que o Tako entende:

| Onde | Zona | Índice |
|---|---|---|
| Barra lateral | `sidebar` | item |
| Início | `hero`, `row-N` (N = fileira), `empty` | botão, coluna, ação |
| Biblioteca | `chip`, `grid` | chip, capa |
| Detalhes | `action` | botão |
| Configurações | `category`, `row` | categoria, linha |
| Central | `cc-icon`, `cc-panel` | ícone, linha |
| Camada do topo | `overlay-row`, `overlay-key`, `overlay-tab`, `overlay-tab2`, `overlay-thumb`, `overlay-outside` | linha, tecla, aba, miniatura, (fora da camada: `0`) |

Para o controle e o teclado funcionarem, o `App` precisa ter um `FocusScope` com foco que repasse
`key-pressed`/`key-released` aos callbacks, como faz o tema padrão (procure `fs.focus()` e `FocusScope` em
`main.slint`). O controle não passa pelo tema.

## 5. Tipos

```slint
struct Hint { glyph: string, label: string }

// kind: toggle, choice, action, info, header, radio, check, slider, text, button
// style: "", "primary" ou "danger"
struct PanelRow {
    key: string, kind: string, title: string, description: string, value: string,
    on: bool, enabled: bool, emphasized: bool, icon: string, style: string, image: image,
}

struct Tile {
    id: string, title: string, meta: string, cover: image, accent: color,
    installed: bool, favorite: bool, running: bool,
}

struct HomeRow { title: string, count: string, tiles: [Tile], see-all: bool }
struct NavItem { id: string, label: string, icon: string, count: string, section: string, selected: bool }
struct Hero {
    has-game: bool, title: string, overline: string, meta: string, description: string,
    art: image, logo: image, play-label: string, running: bool,
}
struct Stat { label: string, value: string }
struct Details {
    title: string, overline: string, meta: string, description: string,
    art: image, cover: image, logo: image, hint: string,
}
struct Key { label: string, row: int, col: int, span: int }
```

Os textos chegam **já traduzidos** no idioma escolhido. O tema não precisa de catálogo próprio.

`icon` (em `PanelRow` e `NavItem`) é um nome do conjunto do tema padrão (`Icons.stroke(nome)` e
`Icons.fill(nome)` em `@tako/icons.slint`): `alert`, `battery`, `camera`, `check`, `chevron`, `close`, `filter`,
`folder`, `gamepad`, `gauge`, `gear`, `grid`, `home`, `image`, `info`, `keyboard`, `limit`, `monitor`, `more`,
`palette`, `play`, `pointer`, `power`, `search`, `sort`, `star`, `starfill`, `tv`, `volume` e os logos
`store-amazon`, `store-battlenet`, `store-ea`, `store-epic`, `store-gog`, `store-manual`, `store-steam`,
`store-ubisoft`, `store-xbox`. Um tema pode desenhar os ícones como quiser a partir do nome.

## 6. O global `Ui` e `tokens.toml`

`Ui` (em `@tako/tokens.slint`) guarda os tokens do tema padrão. O Tako atualiza estes campos, e um tema próprio
pode ler os mesmos se exportar o `Ui` do padrão:

| Campo | O que o Tako põe |
|---|---|
| `accent`, `accent-deep` | Cor do jogo em foco e a versão escura (muda a cada foco). |
| `reduce-motion` | Acessibilidade: sem animações. Respeite (as durações do padrão já viram `0ms`). |
| `text-scale` | Tamanho do texto escolhido (1 = normal). |
| `pointer` | O último foco veio do mouse: destaque, mas não role listas até o item. |

Num tema que reaproveita o padrão, **`tokens.toml`** troca as cores `in-out`, fontes e fundo do `Ui` sem escrever Slint: `nome =
"#RRGGBB"` ou `"#RRGGBBAA"` (exceto `canvas`, que exige `"#RRGGBB"` de 6 dígitos sem canal alfa, com contraste >= 4.5:1 com texto branco). Cores disponíveis: `accent`, `canvas`, `on-accent`, `on-accent-muted`, `on-accent-subtle`,
`surface-1`, `surface-2`, `surface-sidebar`, `surface-solid`, `on-surface-solid`, `on-surface-solid-muted`,
`control-track-off`, `control-off`, `control-on-on-solid`, `control-off-on-solid`, `control-track-on-solid`,
`value-pill-on-solid`, `scrim`, `success`, `warning`, `error`. Também aceita fontes (`font`, `font-title`, `font-text`, `font-heading`, `font-desk`, `font-wordmark`) e imagem de fundo (`custom-background`, `background-image`). Pelo próprio Tako (Configurações › Aparência), a pessoa usuária pode personalizar o tema atual, gravando uma sobreposição sem modificar os arquivos originais. Ao carregar, o Tako ignora (com log) a chave ou o valor inválido. O `tako theme check` aponta os dois.


### Música de fundo e sons de navegação

Um tema pode trazer música de fundo e sons personalizados para a navegação do Tako. As declarações podem ser feitas no `theme.toml` ou no `tokens.toml`:

```toml
[audio]
music = "audio/bgm.wav"
move = "audio/move.wav"
accept = "audio/accept.wav"
back = "audio/back.wav"
```

Também são aceitas as chaves em português (`musica`, `mover`, `confirmar`, `voltar`) e declarações no nível raiz do arquivo.

- **Formato suportado:** `.wav` (PCM). Como o mecanismo de áudio do Tako usa a biblioteca nativa SDL3 para reprodução direta sem decodificadores externos pesados, formatos como `.ogg` e `.opus` não são decodificados nativamente pela biblioteca e são recusados na validação.
- **Validação de caminho:** Os arquivos devem ficar dentro da pasta do tema, sem caminhos absolutos e sem `..` (não é permitido sair da pasta do tema).
- **Limites de tamanho e duração:** Música até 32 MB (`MAX_THEME_MUSIC_BYTES`); sons de navegação com duração de até 2 segundos (`MAX_NAV_SOUND_DURATION_SECS`).
- **Comportamento da música:**
  - Toca exclusivamente na tela cheia (Big Picture), em loop contínuo e com fade suave (fade-in ao iniciar/retomar e fade-out ao pausar).
  - Pausa com fade ao abrir um jogo, permanece completamente parada durante todo o jogo e recomeça do início ao encerrar o jogo.
  - Pausa também quando a janela do Tako perde o foco ou é minimizada, retornando quando o foco volta.
  - No modo desktop, a música do tema não toca.
- **Configurações › Som:**
  - **Volume da música do tema:** controle de 0 a 100%, separado do volume dos sons de navegação. Em 0%, a música fica muda e não é reproduzida.
  - **Música de fundo do tema:** opção para ligar ou desligar a música a qualquer momento.
  - **Volume dos sons de navegação:** volume próprio (0 a 100%) para os efeitos ao mover, confirmar e voltar.

### A biblioteca `@tako`

`@tako/...` aponta para os arquivos do tema padrão da versão do Tako instalada:

- `@tako/main.slint`: o `App` completo e os tipos.
- `@tako/tokens.slint`: o global `Ui`.
- `@tako/widgets.slint`: `RowView`, `HintBar`, `Pill`, `Toggle`, `Icon`, `Glyph`.
- `@tako/icons.slint`: o global `Icons`.

Reaproveitar peças do padrão (`import { RowView } from "@tako/widgets.slint";`) é a forma recomendada de começar.
Saiba, porém, que os componentes internos do padrão podem mudar entre versões menores do Tako. **O contrato
estável é o da seção 4 (o `App`) e da seção 5 (os tipos).**

## 7. Pacote e distribuição

Um tema é uma **pasta** com `theme.toml` ou um **`.takotheme`**: a mesma pasta compactada em zip, com o
`theme.toml` na raiz do zip ou dentro de uma única pasta.

Limites (dados de terceiros são conferidos antes de qualquer arquivo ser gravado):

| Limite | Valor |
|---|---|
| Pacote em disco | 200 MB |
| Total descompactado | 200 MB |
| Um arquivo | 64 MB |
| Arquivos e pastas | 5.000 |
| Caminho | 240 caracteres, sem `..`, sem caminho absoluto, sem nomes reservados do Windows |
| Tipos de arquivo | `.slint`, `.toml`, `.png`, `.jpg`, `.jpeg`, `.svg`, `.webp`, `.ttf`, `.otf`, `.txt`, `.md`, `.wav` |

Imagens e fontes entram pelo próprio Slint (`@image-url("fundo.png")`, `import "fonts/Minha.ttf";`), com caminho
relativo ao arquivo `.slint`.

Para instalar:

- **Arquivo:** Configurações › Aparência › Tema › **Instalar tema…** e escolha o `.takotheme`.
- **Pasta (durante o desenvolvimento):** copie para `<dados do Tako>/themes/<id>/`; o tema aparece na lista.
- **Endereço:** publique uma *release* no GitHub com o `.takotheme` anexado. Quem usa o tema digita o endereço do
  repositório em **Instalar por endereço…**. Com **Atualizar temas**, o Tako baixa a release nova quando houver.

Os temas ficam em `<dados do Tako>/themes/<id>/`. Instalar um tema com o mesmo `id` substitui o anterior.

| Sistema | `<dados do Tako>` |
|---|---|
| Windows (instalado) | `%LOCALAPPDATA%\Tako` |
| Windows (portátil) | `Tako_Data` ao lado do `tako.exe` |
| Linux | `$XDG_DATA_HOME/tako` (ou `~/.local/share/tako`) |
| macOS | `~/Library/Application Support/Tako` |

## 8. `tako theme check`

```
tako theme check <pasta-do-tema>
tako theme check meu-tema.takotheme
```

Aplica as mesmas regras de instalar (manifesto, versão da API, limites, caminhos, tipos de arquivo), compila o
`.slint` com a biblioteca `@tako` desta versão do Tako e confere o `tokens.toml`. Tudo acontece numa pasta
temporária e **nada é instalado**.

- Saída `0`: tema válido (`Meu tema 1.0.0 (autor.meu-tema): tema válido, API 1.`).
- Saída `1`: problemas, um por linha, com o arquivo e a linha do erro do Slint.
- Saída `2`: uso errado.

## 9. Versões da API

A versão maior (`api = 1`) só sobe quando o contrato quebra: uma propriedade ou callback das seções 4 e 5 sai, muda
de tipo ou de significado. Nesse caso, temas da versão anterior são recusados com uma mensagem clara e voltam ao
padrão. Propriedades **novas** entram sem mudar a versão. Por isso o Tako ignora o que o tema não declara, e um
tema antigo continua funcionando, só sem a novidade.

## 10. Checklist antes de publicar

- [ ] `tako theme check` sem problemas.
- [ ] Controle: todas as páginas, a Central e os diálogos têm o foco visível.
- [ ] Mouse: `clicked` e `hovered` nas zonas da [seção 4.9](#49-callbacks).
- [ ] `Ui.reduce-motion` desliga as animações; `Ui.text-scale` em 1,5 não corta textos.
- [ ] Janela pequena (640×360) e grande (4K).
- [ ] Por cima do jogo (`transparent-background`): só a barra da Central aparece.
- [ ] Os dois idiomas (pt-BR e en-US): textos mais longos cabem.
- [ ] Licença das fontes e imagens incluídas (um `LICENSE.txt` ou `.md` no pacote).
