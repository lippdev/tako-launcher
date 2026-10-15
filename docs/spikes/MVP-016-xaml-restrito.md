# Spike MVP-016: carregador XAML restrito

- Status: implementado e homologado com temas reais (MVP-022); liberações da interface inteira (MVP-018/019) descritas abaixo; questão aberta 8 (caminhos montados por dados) fechada na frente H
- Data: 2026-10-14
- Branch: `spike/MVP-016-xaml-loader`
- Riscos do PRD: "XAML não confiável executar comportamentos indevidos" (crítico), CA-04, CA-10

## Pergunta

É possível carregar XAML de temas Playnite 10 com o `XamlReader` do WPF sem permitir que um tema execute código, acesse arquivos ou a rede?

## Premissa

No .NET moderno o `XamlReader` **não é sandbox**: não existe mais Code Access Security, e qualquer tipo resolvível pode ser instanciado. A proteção precisa acontecer **antes** do parser. Uma segunda camada (processo de UI com privilégios mínimos, PRD §4.3) continua necessária.

## Abordagem

Duas etapas, em `src/Launcher.Themes/Xaml/`:

1. **`XamlValidator`** lê o arquivo como XML puro (sem DTD, sem resolver externo, com limites de tamanho, profundidade e elementos) e compara cada nome com uma lista de permissões (`XamlSecurityPolicy`). Ele não instancia tipos nem carrega assemblies.
2. **`RestrictedXamlLoader`** só entrega ao `XamlReader.Load` o mesmo texto que foi validado, com `BaseUri` na pasta do tema, e exige `ResourceDictionary` como raiz.

Tudo o que não está na lista é rejeitado, com código, mensagem e linha/coluna (`XamlViolation`).

## Ameaças cobertas

| Ataque | Exemplo | Bloqueio |
|---|---|---|
| Executar métodos arbitrários | `ObjectDataProvider` + `Process.Start` / `File.WriteAllText` | tipo e namespace fora da lista |
| Código embutido ou fábricas | `x:Code`, `x:Class`, `x:FactoryMethod`, `x:Arguments`, `x:XData` | diretiva não permitida |
| Carregar assembly arbitrário | `xmlns:e="clr-namespace:Evil;assembly=Evil"` | só namespaces da lista podem ser declarados |
| Ler estado do app | `{x:Static Application.Current}` | `x:Static` só em donos permitidos (`SystemColors`, enums etc.) |
| Rede e rastreamento | `Image Source="https://..."`, `XmlDataProvider`, `Frame`, `WebBrowser` | URI absoluta rejeitada em qualquer atributo; tipos fora da lista |
| Ler arquivos fora do tema | `Source="../../x"`, `C:\...`, `\\servidor\...`, `pack://siteoforigin` | path traversal e caminhos absolutos |
| Bypass via `Setter` | `<Setter Property="Source" Value="..\..\x"/>` | `Value` é validado como a propriedade-alvo |
| XML malicioso | DTD / billion laughs, PI `<?Mapping ...?>` | DTD proibido, PIs rejeitadas, limites |
| Esconder conteúdo | `mc:AlternateContent`, `mc:ProcessContent`, prefixos ignoráveis arbitrários | só `mc:Ignorable` com `d:` ou `PresentationOptions:` |
| Carregar XAML sem validação | `<ResourceDictionary Source="Outro.xaml"/>` | rejeitado (ver descobertas) |
| Handlers de evento | `Click="OnClick"` | rejeitado pelo parser WPF (sem `x:Class`) e convertido em `ThemeXamlRejectedException` |
| Pedaços de caminho em literais | `<sys:String>\\servidor\x</sys:String>`, `StringFormat='\\\\srv\\{0}'`, `ConverterParameter`, `FallbackValue`, `TargetNullValue` | regras de URI no texto de elementos e em qualquer posição de formatos e literais de binding (questão 8) |
| Caminho montado em tempo de execução | `MultiBinding`/`StringFormat`/`sys:String` → `Image.Source`, `ImageBrush.ImageSource`, `MediaElement.Source` | guarda de mídia no conversor padrão do binding (`ThemeMediaGuard` + `ThemeSourcePolicy`, questão 8) |

## Resultados

- 49 testes do validador (corpus malicioso + um dicionário típico de tema) passam no macOS, rodando os mesmos arquivos-fonte fora do projeto WPF.
- Durante o spike, os testes encontraram um bug real: `Split('/', '\\', options)` cai na sobrecarga `Split(char, int count, ...)`, e contrabarras não eram tratadas como separador. Corrigido.
- **Pendente no Windows:** os 4 testes de `RestrictedXamlLoaderTests` (carregamento real pelo WPF numa thread STA) e a medição com temas reais (checklist abaixo).

## Descobertas e questões abertas

1. **`ResourceDictionary.Source` é um carregamento sem validação.** O WPF carregaria o arquivo referenciado por conta própria. Decisão do spike: rejeitar. O motor de temas (MVP-017) deve compor os dicionários: validar e carregar cada arquivo pelo `RestrictedXamlLoader` e adicioná-lo a `MergedDictionaries` por código.
2. **Namespaces do Playnite precisam de remapeamento.** Temas Playnite declaram tipos de assemblies como `Playnite` e `Playnite.FullscreenApp` (conversores, controles `PART_`, extensões de marcação; nomes exatos a confirmar com temas reais). Não vamos distribuir assemblies com esses nomes: o adaptador P10 deve registrar esses `xmlns` na política com a lista de tipos que implementa, e o carregador deve **reescrever** o `xmlns` para `Launcher.PlayniteCompat.P10` depois da validação. Isso fica para MVP-018/019.
3. **Behaviors (`http://schemas.microsoft.com/xaml/behaviors`) estão bloqueados.** Se temas homologados dependerem deles, cada behavior precisa entrar na lista individualmente, nunca o namespace inteiro.
4. **Bindings `TwoWay` com `RelativeSource FindAncestor`** podem alterar propriedades da janela (ex.: `Topmost`). Não executa código, mas é um argumento a favor do processo de UI isolado e com privilégios mínimos.
5. **Falso positivo conhecido:** qualquer atributo cujo valor seja uma URI absoluta (ex.: `Text="https://..."`) é rejeitado. É conservador de propósito, porque propriedades de controles de terceiros podem carregar URIs com nomes arbitrários.
6. **A lista de permissões é inicial.** `Popup` entrou só como lista de lista suspensa: dentro do `ControlTemplate` mais interno de um tipo em `PopupHostTemplates` (`ComboBox`; `ComboBoxEx` nos temas P10), com `IsOpen="{TemplateBinding IsDropDownOpen}"`, `Placement` `Bottom`/`Top` e sem offsets, `PlacementTarget`, `StaysOpen`, estilos implícitos de Popup nem gatilhos/animações que o movam ou abram. Tipos como `ContextMenu`, `KeyBinding`, `Hyperlink`, `SoundPlayerAction` e `CollectionViewSource` estão fora até a homologação (MVP-022) mostrar necessidade.
7. Recursos referenciados entre arquivos (`StaticResource` de outro arquivo) falham ao carregar um arquivo isolado. É esperado: a ordem de composição é responsabilidade do MVP-017.

## Resultado com tema real (tema Fullscreen padrão do Playnite 10)

Medido no Windows CI com `src/Launcher.UI.Wpf/Themes/Playnite-Default` (MIT):

- Sem adaptação: 18 de 35 arquivos passam no validador. As recusas confirmaram as questões 2 e 3 acima, com um achado: os controles do Playnite (`ButtonEx`, `ItemsControlEx`, `GameListItem`…) e as extensões `{ThemeFile}`/`{Settings}` **não têm prefixo**. O Playnite os registra no namespace do WPF via `XmlnsDefinition`, e o adaptador P10 faz o mesmo, sem reescrever o XAML.
- `Constants.xaml` só era recusado por **declarar** Interactivity sem usar. O validador agora tolera declarações não usadas (qualquer uso, inclusive em valores como `Setter Property="i:…"`, continua sendo violação) e o carregador as remove antes do parser.
- Com o adaptador P10: 32 de 35 passam na validação e 27 de 33 eram carregados pelo WPF. Com o `Popup` restrito à lista suspensa e os booleanos/conversores que o Playnite fornece aos temas: 33 de 35 na validação e **31 de 33 carregados e aplicados pelo WPF**. O tema é renderizado na prévia do launcher sem o Playnite instalado.
- O WPF adia erros de estilos e templates até o uso (ex.: `TemplateBinding InputHint`, propriedade que só existe no Playnite). O carregador agora sela estilos e carrega o conteúdo dos templates na carga, para o erro recusar o arquivo; e um erro tardio vira tela de recuperação.
- Questão 1 resolvida: o adaptador compõe os dicionários por código, na ordem do Playnite, e cada arquivo enxerga os recursos dos anteriores (`StaticResource` entre arquivos).

## Liberações para a interface inteira (MVP-018/019)

Para o `Views/Main.xaml` e o `Views/GameDetails.xaml` dos temas carregarem, o adaptador P10 (`src/Launcher.PlayniteCompat.P10/P10Policy.cs`) passou a liberar os itens abaixo. Nenhum namespace foi liberado por inteiro e nenhum item executa código do tema; cada um é uma implementação do próprio adaptador, com teste em `tests/Launcher.UI.Wpf.Tests/P10PolicyTests.cs` e `PlayniteAdapterTests.cs` (e `XamlValidatorTests` para a política base).

| Item | Onde aparece | O que o adaptador faz | Por que é seguro |
|---|---|---|---|
| `SplineThicknessKeyFrame` (política base) | PS5ish, `Constants.xaml` | Tipo do WPF: quadro-chave de animação de margem | Só interpola valores, como os `Spline*KeyFrame` que já eram permitidos |
| `{PluginSettings}` / `{PluginStatus}` | Toggle, PS5ish, Revisited | Devolvem `FallbackValue`/nulo e "não instalado" | É o comportamento do Playnite quando a extensão não existe; nenhum plugin é carregado |
| `{SettingsBinding}` e `Mode` em `{Settings}` | PS5ish (`Button.xaml`), Toggle | Valores do Fullscreen com os padrões do Tako | O binding TwoWay escreve numa cópia isolada; as configurações do launcher nunca mudam |
| `{ThemeFileBinding}` | Toggle | Binding a um dado do jogo formatado em `PathFormat` e resolvido como arquivo do tema | Mesma proteção do `{ThemeFile}`: caminho normalizado dentro da pasta do tema, sem `:`, UNC ou URL, e o arquivo precisa existir; senão o valor é nulo |
| `{Api …}` | Todos os `Main.xaml` (`Notifications.Count`) | Binding **OneWay** para a `P10Api` herdada pela árvore (`P10Context.Api`) | Objeto só-leitura do adaptador: notificações (0), caminhos **vazios**, `ApplicationInfo`, `MainView.SelectedGames` e o `MainModel`. Sem métodos, sem banco, sem plugins, sem serviços do launcher e sem estado global |
| `Main` | `{x:Type Main}` | Controle que liga os `PART_` ao modelo (MVP-019) | Código do adaptador; o tema só fornece o template |
| `HtmlTextView` | `GameDetails.xaml` | **Substituto**: `TextBlock` com texto puro (`PlainText.FromHtml`) | Nenhum HTML é interpretado: tags e scripts somem, sem imagens, links, estilos ou rede |
| `MediaElementBehaviors.Repeat` | PS5ish, Revisited | Reinicia a mídia ao terminar | Dono de propriedade anexada (não pode ser elemento); a origem da mídia continua sujeita às regras de caminho |
| `pbeh:FocusBahaviors` (`FocusBinding`, `OnVisibilityFocus`) | Toggle | Foca o elemento quando o valor fica verdadeiro ou quando ele aparece | Só move o foco dentro da tela do tema. Liberado **por nome** (`Playnite.Behaviors` → adaptador); outros behaviors continuam recusados |
| `pin:GameControllerInputBinding` | Toggle | Atalho inerte: o gesto nunca corresponde e o comando nunca roda | A entrada do controle é do launcher. Liberado por nome (`Playnite.Input` → adaptador) |

`XamlSecurityPolicy.Extend` ganhou `presentationOwnerTypes`, para liberar donos de propriedades anexadas sem permitir instanciá-los.

**Continua bloqueado:** caminhos fora do tema (os vídeos e a foto de perfil de `\ExtraMetadata` no `Main.xaml` do PS5ish e o fundo/logo do `GameDetails.xaml` do Revisited, montados com `<Binding Source="\ExtraMetadata\...">`), o `HtmlTextView` original, os demais behaviors e tipos de entrada do Playnite e as extensões de terceiros. Quando um arquivo do tema é recusado, entra o do tema padrão.

**Rede de segurança na execução (MVP-021):** antes de trocar a tela inicial, `P10Theme.Probe` monta `Main` e `GameDetails` fora da tela com o modelo real e faz medida e arranjo; erros que o WPF só mostra ao instanciar o template ficam ali, e o Tako continua na UI padrão. Em uso, o host (`PlayniteMainHost`) captura erros de layout do tema, e um `XamlParseException` tardio passa pela janela; nos dois casos o tema sai da tela e aparece a recuperação ("Usar a interface do Tako" limpa a escolha). O tema nunca é reaplicado sozinho (sem loop) e não é aplicado no modo seguro, que liga depois de duas aberturas sem fechamento limpo.

### Questão 8 (fechada na frente H): caminhos montados por dados

**Problema.** O validador confere valores literais. Um tema podia **compor** um texto em tempo de execução (`MultiBinding` com `StringFormat`, recursos `sys:String`, `Tag`/`Text` de outros elementos, conversores) e ligá-lo a `Image.Source`, `ImageBrush.ImageSource`, `MediaElement.Source` ou `BitmapImage.UriSource`. Assim alcançava arquivos fora da pasta do tema, URLs externas e caminhos UNC (`\\servidor\share`), que no Windows abrem uma conexão SMB e **vazam o hash NTLM** do usuário. O PS5ish já monta caminhos assim (`ExtraMetadataPath` + `MultiBinding`), e só era recusado porque o pedaço literal `\ExtraMetadata\` é bloqueado.

**Por que não coerção na propriedade.** O texto vira imagem no **conversor padrão do binding**, e o `ImageSourceConverter` já abre o arquivo ou inicia o download *durante a conversão* (`BitmapFrame.CreateFromUriOrStream`). Um `CoerceValueCallback` em `Image.Source` (ou um estilo implícito, ou um attached behavior) só vê o `BitmapFrame` pronto: a conexão SMB já teria acontecido. Subclasses do adaptador também não resolvem: `ImageBrush` e `BitmapImage` são `sealed`, e um `Setter` com `TargetType="Image"` continuaria indo para a propriedade original. A coerção só funcionaria para `MediaElement.Source` (o `Uri` é aberto depois), que mesmo assim não aceita `OverrideMetadata` no próprio tipo.

**Decisão: guarda no conversor padrão do binding** (`src/Launcher.Themes/Security/ThemeMediaGuard.cs`). Toda conversão de `string`/`Uri`/`object` → `ImageSource`/`Uri`/`FontFamily`/`Cursor` feita por `Binding`, `MultiBinding`, `PriorityBinding` (inclusive o fallback dinâmico quando um conversor devolve outro tipo) sai da tabela de conversores padrão do `DataBindEngine` do WPF, uma por thread de UI. A guarda troca essas entradas por um conversor que:

1. decide pela `ThemeSourcePolicy` (`src/Launcher.Core/Themes/ThemeSourcePolicy.cs`) **antes** do conversor original, resolvendo o caminho relativo a partir do `BaseUri` do elemento de destino, como o WPF faria;
2. entrega ao conversor original a **URI já resolvida e permitida** (o que carrega é exatamente o que foi conferido);
3. bloqueado: devolve nulo (imagem/mídia vazia, sem erro para o usuário) e dispara `ThemeMediaGuard.Blocked`, que o `PlayniteThemeService` registra no log uma vez por origem, só com esquema e host ou o nome do arquivo (e o `LogRedactor` por cima).

| Origem | Decisão |
|---|---|
| Arquivo dentro da pasta do tema (do XAML do elemento) ou do Fullscreen padrão (arquivos herdados) | ✅ permitido, depois de `GetFullPath` e sem link simbólico/junção entre a pasta e o arquivo |
| Cache de artes do Tako (`AppPaths.ArtCache`) | ✅ permitido |
| `pack://application:,,,/` e caminhos relativos sem XAML de origem (resolvidos no pacote do app) | ✅ permitido (só alcança recursos embutidos) |
| `ImageSource` já carregado (view models do Tako, `{ThemeFile}`, `{ThemeFileBinding}`) | ✅ passa direto, sem conversão |
| UNC, `\\?\UNC`, `\\.\`, `//servidor`, `file://servidor/...` | ❌ `Network`, decidido só pelo texto, sem tocar o disco |
| `http`, `https`, `ftp`, `pack://siteoforigin`, outros esquemas | ❌ `External` |
| Arquivo local fora dessas pastas (`..\..\..\Windows\win.ini`, `C:\...`) | ❌ `OutsideTheme` |
| Link simbólico ou junção dentro do tema | ❌ `Link` |
| `FontFamily` com local (`/`, `\`, `:`, `#`) montado por binding; `Cursor` que não seja do sistema | ❌ (nomes de família e cursores do sistema passam) |

A guarda vale para a thread de UI inteira, não só para a árvore do tema: os templates do tema são instanciados em vários lugares (prévia, `ThemeScope` da interface inteira, ensaio do `P10Theme.Probe`) e `Freezable`s como `ImageBrush` nem herdam propriedades da árvore. A UI do próprio Tako nunca converte texto em imagem (recebe `ImageSource` do cache de artes), então nada muda para ela.

**Instalação e falha fechada.** `P10Theme.Load` instala a guarda antes de ler qualquer XAML do tema e registra as pastas do tema e do tema padrão na política; o `ThemeScope` só aplica recursos de tema com a guarda ativa. O acesso à tabela é por reflexão a membros internos do WPF (`DataBindEngine.CurrentDataBindEngine`, `_valueConverterTable`, `GetDefaultValueConverter`); a instalação confere que a troca pegou. Se um WPF futuro mudar esses membros, `EnsureInstalled` falha, **nenhum arquivo do tema é carregado** (o relatório mostra o motivo) e o Tako fica na própria UI; os testes acusam no CI.

**Conversores do adaptador.** O `ImageStringToImageConverter` aceitava qualquer caminho absoluto existente, inclusive UNC (o `File.Exists` já abria a conexão). Agora usa a mesma política, decidida antes de qualquer acesso ao disco. `{ThemeFile}`/`{ThemeFileBinding}` passaram a recusar caminhos que atravessam link simbólico ou junção.

**Validação estática (complemento).** Pega o óbvio já na carga, com o código de violação de sempre:

- texto de elementos (`<sys:String>`, conteúdo de `TextBlock`…): URI ou caminho absoluto recusado como nos atributos (`ExternalUri`); em recursos primitivos (`sys:String` etc.), em qualquer posição do texto;
- `StringFormat`, `ContentStringFormat`, `ConverterParameter`, `FallbackValue`, `TargetNullValue` e qualquer literal dentro de uma extensão de marcação (depois de tirar os escapes `\`): `://`, `\\`, `//servidor`, letra de unidade e `file:` em qualquer posição viram `ExternalUri`; segmentos `..` viram `PathEscapesTheme`.

`PathEscapesTheme` **não** se aplica ao texto de `sys:String`: o `Constants.xaml` do PS5ish guarda `ExtraMetadataPath = ..\..\..\..` e seria recusado, mudando a homologação. O valor sozinho é inofensivo; o caminho que ele montar é barrado pela guarda.

**Testes.** `XamlValidatorTests` (literais em `sys:String`, formatos e extensões, e os formatos comuns dos temas que continuam aceitos), `ThemeSourcePolicyTests` (política, sem WPF), `ThemeMediaGuardTests` (WPF: tema que passa na validação e monta `\\exemplo\share\x.png`, `http://exemplo/x.png`, `..\..\..\Windows\win.ini` e um arquivo fora do tema por `MultiBinding`, `sys:String` e `StringFormat`, ligados a `Image`, `ImageBrush` e `MediaElement`; nada carrega, e o mesmo caminho montado dentro do tema carrega) e o bloco "Segurança de temas (frente H)" do teste de fumaça, que faz o mesmo com uma cópia do Fullscreen padrão dentro de um `ThemeScope` (relatório `theme-security.txt`) e confere que PS5ish, Toggle, Revisited e o padrão continuam carregando o mesmo número de arquivos.

**Limitações que ficam.**

- Depende de membros internos do WPF (ver falha fechada acima). É a única forma que encontramos de decidir **antes** da conversão; a alternativa seria reescrever todos os bindings do tema no carregador.
- Conversões que não passam pelo binding (`FallbackValue`/`TargetNullValue` e `Value` de `DataTrigger`) usam o conversor do tipo direto; o texto delas é sempre literal ou recurso, e por isso fica com a validação estática.
- Quando o WPF não sabe de qual XAML veio o elemento, vale qualquer pasta de tema registrada (um tema poderia exibir imagens de outro tema instalado), nunca fora delas.
- Texto de elemento com `://` passa a ser recusado (mesmo falso positivo conservador da questão 5).
- Continua não sendo sandbox: o processo de UI isolado e com privilégios mínimos (PRD §4.3) segue recomendado como segunda camada.

## Recomendação

Seguir com a abordagem de validação prévia com lista de permissões, que responde à pergunta do spike para o subconjunto WPF. A decisão final depende de medir os temas reais: se a maioria das rejeições cair nas questões 2 e 3, o caminho é o adaptador P10 com remapeamento; se houver dependência ampla de tipos perigosos, o escopo de compatibilidade (H2) precisa ser revisto.

## Checklist no Windows

1. `dotnet test --project tests/Launcher.Themes.Tests`: os 53 testes devem passar (49 do validador + 4 do carregador).
2. Extrair 3 temas Fullscreen do Playnite 10 (um pacote `.pthm` é um zip), incluindo um popular e um parcial.
3. Para cada um: `dotnet run --project tools/Launcher.ThemeCheck -- <pasta-do-tema>` e depois com `--load`.
4. Registrar aqui, por tema: arquivos aceitos/total e as violações mais frequentes (o resumo no fim da saída), além dos `xmlns` declarados que não são do WPF.
