# Estudo AI-001 + THEME-010: assistente de IA para a biblioteca e conversor de temas do Playnite

- Status: estudo concluído; nenhuma das duas frentes vira código agora (recomendações no fim de cada parte)
- Data: 2026-10-17
- Branch: `docs/AI-001-THEME-010-study`
- Itens do PRD: AI-001 (épico VOICE e AI, P3) e THEME-010 (épico THEME, P2). As duas pedem estudo antes de qualquer
  código: o AI-001 por ser "estudo de um assistente", o THEME-010 porque "só segue se o estudo mostrar resultado útil"
  (D-05).

---

## Parte 1: AI-001, assistente de IA para organizar e gerenciar a biblioteca

### Pergunta

Um assistente de IA consegue organizar e gerenciar a biblioteca (coleções, limpeza, recomendações, configurações por
jogo) sem quebrar as regras do Tako (biblioteca 100% offline, nada de executar o que vem de fora, uso com controle,
LGPD) e com ganho real sobre o que o Tako já faz sem IA?

### O que o Tako já resolve sem IA

| Pedido típico | Função existente | Falta só a "tradução" da frase |
|---|---|---|
| "meus jogos de corrida instalados" | filtros e coleções automáticas (LIB-003): gênero, fonte, status, tags, tempo | sim |
| "o que eu larguei pela metade" | status (LIB-001), tempo jogado e última vez (SESSION-002) | sim |
| "limpar a biblioteca" | jogos quebrados (LIB-004), armazenamento (LIB-005), revisão de capas (ART-001) | não: já são listas prontas |
| "capas erradas" | Revisar capas (ART-001), Fontes e IDs, seletor de imagens | não |
| "jogar algo curto hoje" | tempo jogado; falta duração estimada (IDEA-AI-05, HowLongToBeat) | dado novo, não IA |
| "configurar este jogo" | tela, áudio, argumentos, variáveis e comandos por jogo (SESSION-004) | sim, mas é a área de maior risco |

Conclusão: a maior parte do valor está em **traduzir uma frase em filtros e ações que já existem**, não em o modelo
"saber" algo. As listas de limpeza já são determinísticas e não precisam de modelo.

### Onde o modelo roda

| Opção | Prós | Contras |
|---|---|---|
| **A. Sem modelo** (regras e listas prontas) | Offline, previsível, zero custo, testável | Não entende frase livre |
| **B. Modelo local do usuário** (servidor compatível com OpenAI em `localhost`: Ollama, LM Studio, llama.cpp; IDEA-AI-12) | Nada sai da máquina; o usuário escolhe e baixa o modelo | 2 a 8 GB de disco, de 4 a 8 GB de RAM ou VRAM; disputa a GPU com o jogo; qualidade varia muito com modelos pequenos |
| **C. API na nuvem com a chave do usuário** (Claude, OpenAI etc.) | Melhor qualidade, sem custo de hardware | Títulos, gêneros e hábitos saem da máquina (LGPD, consentimento explícito); custo por uso; depende de rede |
| D. Modelo embutido no Tako | Funciona sem instalar nada | Pacote centenas de MB maior, atualização e licença do modelo, desempenho ruim em PCs modestos. Descartada |

Embutir uma biblioteca de inferência (llama.cpp ligado ao binário) foi descartado pelo mesmo motivo de D: tamanho,
compilação nos três SOs e GPU. Falar com um servidor local por HTTP mantém o Tako pequeno e deixa a escolha com o
usuário.

### Desenho seguro (se for adiante)

1. **O modelo propõe, o Tako executa.** A resposta é **JSON com um esquema fechado** (tool calls), nunca texto
   livre que vira ação. Ações permitidas na primeira fase, todas reversíveis e já existentes:
   - `filter` / `create_smart_collection` (filtros do LIB-003);
   - `set_status`, `add_tags` (vários jogos de uma vez);
   - `open_list` (quebrados, armazenamento, revisar capas).

   Fora da lista: jogar, instalar, baixar (o épico DL fica com as fontes do próprio usuário e nunca pelo assistente),
   comandos antes e depois do jogo, variáveis de ambiente, apagar jogos ou pastas.
2. **Validação contra a biblioteca.** Ids e valores inventados pelo modelo são descartados: só passam ids que existem,
   gêneros e tags conhecidos e status do enum.
3. **Prévia e confirmação.** Toda proposta aparece como lista ("12 jogos ficam com a tag Corrida") com A para aplicar
   e B para cancelar; nada é aplicado sozinho. Desfazer aproveita o mesmo caminho.
4. **Injeção de prompt.** Descrições e nomes vêm de fontes externas (IGDB, lojas) e podem conter instruções. Elas vão
   ao modelo como dados delimitados, e o esquema fechado mais a confirmação limitam o estrago a uma proposta errada.
5. **Privacidade.** Desligado por padrão. Na opção C, a tela mostra o que é enviado (títulos, gêneros, status; nunca
   caminhos, contas ou ids de loja), como pede o DATA-001. A chave fica no cofre do SO, como as das fontes de metadados.
6. **Nunca durante o jogo.** A UI fecha durante a sessão (ADR-0026); o assistente não tem como disputar a GPU.
7. **Controle.** Digitar no teclado virtual é lento. Sugestões prontas ("Largados pela metade", "Curtos para hoje",
   "Sem capa") resolvem a maior parte, e a frase livre fica para teclado ou voz (VOICE-001).

### Custo estimado

- Fase 0 (sem modelo): uma tela "Sugestões" com listas prontas, montadas pelos filtros existentes. De 2 a 3 dias.
- Fase 1 (frase → filtros e ações, opções B e C atrás de um trait `Assistant`, cliente HTTP `ureq` já existente,
  esquema e validação em `tako-core` testáveis com respostas gravadas): de 1 a 2 semanas, mais as decisões abaixo.

### Decisões que são do mantenedor

1. A opção C (nuvem) entra? Ela pede o mesmo cuidado de consentimento do DATA-001.
2. Qual servidor local é o "oficial" para a documentação: Ollama, LM Studio ou só "compatível com OpenAI"?
3. Vale a pena a Fase 1 antes do VOICE-001? Sem voz, a frase livre pelo controle rende pouco.

### Recomendação

**Não implementar o assistente com modelo agora.** Fazer primeiro a **Fase 0**, que entrega a maior parte do valor
sem nenhum dos riscos, e registrar a Fase 1 como item próprio quando as três decisões acima forem tomadas. O desenho
seguro acima vale como requisito para quando ela vier.

---

## Parte 2: THEME-010, conversor de temas do Playnite 10 para o formato do Tako

### Pergunta

Dá para converter um tema do Playnite (Fullscreen) para um tema do Tako com resultado útil? "Útil" aqui é: a pessoa
reconhece o tema convertido e ele funciona sem retoque.

### Os dois formatos

| | Playnite (Fullscreen, API 2.0) | Tako |
|---|---|---|
| Pacote | `.pthm` (zip): `theme.yaml`, XAML do WPF, imagens, fontes | pasta ou `.takotheme`: `theme.toml`, `main.slint`, `tokens.toml` opcional |
| Cores | `Constants.xaml`: `Color`, `SolidColorBrush`, gradientes | `tokens.toml`: cores do global `Ui` |
| Layout | `Views/*.xaml` (Main, GameDetails, menus) com `ControlTemplate`, `PART_*` e controles do Playnite (`ButtonEx`, `ListBoxEx`, `FadeImage`…) | `main.slint` com o componente `App` e as propriedades que o Tako preenche (`page`, `library-tiles`, `overlays`…) |
| Comportamento | `Trigger`, `DataTrigger`, `Storyboard`, bindings ao modelo do Playnite | estado vem do Rust; o tema só desenha (CONTRIBUTING §6.6) |

### Medição no tema Fullscreen padrão do Playnite

Tema em `src/Launcher.UI.Wpf/Themes/Playnite-Default` (MIT, o mesmo homologado no MVP-016/022), 42 arquivos:

- **Cores**: `Constants.xaml` tem 10 `Color`, 9 `SolidColorBrush` (todos apontando para essas cores) e 6 gradientes,
  alguns usando as próprias cores (`SelectionBrush` = `GlyphColor` → `GlyphLightColor`). É pouca coisa e com nomes
  estáveis entre temas derivados do padrão.
- **Layout**: 18 arquivos de view e estilo. Só o `Main.xaml` tem 231 linhas, 10 `ButtonEx`, `ListBoxEx`, `FadeImage`,
  `GameStatus` e partes `PART_*` que o código do Playnite liga ao modelo.
- **Comportamento**: 130 `Trigger`, 14 `DataTrigger`, 4 `MultiDataTrigger` e 26 `Storyboard` no tema inteiro, além de
  30 bindings ao modelo (`PowerStatus.Charge`, `Title`, `Icon`…), 4 `{ThemeFile}` e 4 `{Settings}`.
- **Fontes e ícones**: o tema usa fontes próprias como fonte de ícones (`FontIcoFont`, `FontXboxOneController`,
  `FontPlayStation`: 58 referências). O Tako desenha os ícones e os glifos dos botões pelo próprio tema, e as fontes do
  `Ui` são de saída (`font-title`, `font-text`), não tokens.

Temas populares da comunidade (PS5ish, Toggle, Revisited, no MVP-022) mudam justamente o layout e as animações, que
é a parte que não converte. Pelo que o MVP-016 mostrou, eles também dependem de extensões (`{PluginSettings}`), de
behaviors e de caminhos de `ExtraMetadata`.

### O que dá e o que não dá

| Parte | Converte? | Por quê |
|---|---|---|
| Cores sólidas | **Sim**, com mapeamento de nomes | `Color`/`SolidColorBrush` literais; atenção: o XAML usa `#AARRGGBB` e o `tokens.toml` usa `#RRGGBBAA` |
| Gradientes | Aproximado | o `tokens.toml` só aceita cor sólida; dá para usar a primeira parada ou a média |
| Imagem de fundo do tema | Não | o fundo do Tako é a arte do jogo em foco; uma imagem fixa pede mudança de layout |
| Fontes | Não (hoje) | fontes não são tokens; liberar fonte de tema é decisão de segurança e de licença |
| Layout, menus e detalhes | **Não** | XAML do WPF com controles e modelo do Playnite. Traduzir para Slint exigiria reimplementar o motor do Playnite (o mesmo trabalho do adaptador P10, que D-05 encerrou) |
| Animações e gatilhos | Não | `Storyboard` e `Trigger` não têm equivalente direto e dependem do modelo do Playnite |

Exemplo do mapeamento de cores no tema padrão:

```toml
# Gerado de Playnite_builtin_DefaultFullscreen (Constants.xaml). Só cores.
canvas = "#0B1027FF"           # fim do gradiente do fundo principal
on-accent = "#EBEBEBFF"        # TextColor
on-accent-muted = "#EBEBEBB8"  # TextColor com a transparência do tema padrão
on-accent-subtle = "#757575FF" # TextColorDark
surface-1 = "#2B354D80"        # ControlBackgroundColor
surface-2 = "#2B354DCC"
surface-sidebar = "#121830CC"  # ControlBackgroundColorDark
scrim = "#000000BB"            # OverlayColor (#BB000000 no XAML)
warning = "#FF6B6BFF"          # WarningBrush
```

O resultado é "o Tako com as cores do tema", não o tema. A cor que mais marca o Playnite padrão, o laranja do
`GlyphColor` na seleção, nem tem para onde ir: no Tako o destaque é a superfície sólida e o `accent` vem da arte do
jogo (cor dinâmica), não de um token.

### Segurança, se um importador de paleta for feito

Ler só XML (`roxmltree`, que já é dependência), sem instanciar XAML; aceitar só `Color`, `SolidColorBrush` e
`GradientStop` com valores literais ou `DynamicResource` do mesmo arquivo; limites de tamanho e de entradas no zip,
como no THEME-004; nada de imagens, fontes ou caminhos. O resultado é um `tokens.toml` validado pelo carregador de
sempre.

### Licença

Temas da comunidade têm licenças variadas. Converter localmente o tema que o próprio usuário baixou é uso dele.
O Tako não deve distribuir temas convertidos nem oferecer uma galeria deles sem a licença de cada um.

### Recomendação

**Não seguir com o conversor de temas.** O estudo não mostra resultado útil: o que identifica um tema do Playnite
(layout, animações, fontes) não converte, e só a paleta converte, com perda. Quem procura um tema do Playnite não
reconheceria o resultado.

Fica registrada uma alternativa barata para quando o THEME-006 (criar tema pelo Tako) existir: **"Importar cores de
um tema do Playnite"** como uma das origens de paleta do editor. São de 1 a 2 dias com o mapeamento acima e as
regras de segurança, e o valor depende do editor existir antes. Sugestão para o backlog: fechar o THEME-010 como
"estudado, não segue" e anotar a importação de paleta no THEME-006.
