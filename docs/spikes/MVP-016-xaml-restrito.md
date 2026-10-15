# Spike MVP-016: carregador XAML restrito

- Status: implementado e homologado com temas reais (MVP-022); liberações da interface inteira (MVP-018/019) descritas abaixo
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

### Questão aberta 8: caminhos montados por dados

O validador confere valores literais. Um tema ainda pode **compor** um texto em tempo de execução (por exemplo `MultiBinding` com `StringFormat`, ou um recurso `sys:String`, cujo texto não é validado) e ligá-lo a `Image.Source`/`MediaElement.Source`. O WPF resolve o caminho relativo ao arquivo XAML do tema, e um texto como `..\..\x` ou um caminho UNC escaparia da pasta (leitura de mídia local ou acesso SMB). Não é novo nesta etapa: o PS5ish já faz isso no `Main.xaml` e continua recusado só porque o pedaço literal `\ExtraMetadata\` é bloqueado. Recomendações: validar o texto de `sys:String` com as mesmas regras de URI e, como segunda camada, o processo de UI isolado e com privilégios mínimos (PRD §4.3).

## Recomendação

Seguir com a abordagem de validação prévia com lista de permissões, que responde à pergunta do spike para o subconjunto WPF. A decisão final depende de medir os temas reais: se a maioria das rejeições cair nas questões 2 e 3, o caminho é o adaptador P10 com remapeamento; se houver dependência ampla de tipos perigosos, o escopo de compatibilidade (H2) precisa ser revisto.

## Checklist no Windows

1. `dotnet test --project tests/Launcher.Themes.Tests`: os 53 testes devem passar (49 do validador + 4 do carregador).
2. Extrair 3 temas Fullscreen do Playnite 10 (um pacote `.pthm` é um zip), incluindo um popular e um parcial.
3. Para cada um: `dotnet run --project tools/Launcher.ThemeCheck -- <pasta-do-tema>` e depois com `--load`.
4. Registrar aqui, por tema: arquivos aceitos/total e as violações mais frequentes (o resumo no fim da saída), além dos `xmlns` declarados que não são do WPF.
