# Spike MVP-016: carregador XAML restrito

- Status: protótipo implementado; aguardando validação no Windows e com temas reais
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
6. **A lista de permissões é inicial.** Tipos como `Popup`, `ContextMenu`, `KeyBinding`, `Hyperlink`, `SoundPlayerAction` e `CollectionViewSource` estão fora até a homologação (MVP-022) mostrar necessidade.
7. Recursos referenciados entre arquivos (`StaticResource` de outro arquivo) falham ao carregar um arquivo isolado. É esperado: a ordem de composição é responsabilidade do MVP-017.

## Recomendação

Seguir com a abordagem de validação prévia com lista de permissões, que responde à pergunta do spike para o subconjunto WPF. A decisão final depende de medir os temas reais: se a maioria das rejeições cair nas questões 2 e 3, o caminho é o adaptador P10 com remapeamento; se houver dependência ampla de tipos perigosos, o escopo de compatibilidade (H2) precisa ser revisto.

## Checklist no Windows

1. `dotnet test --project tests/Launcher.Themes.Tests`: os 53 testes devem passar (49 do validador + 4 do carregador).
2. Extrair 3 temas Fullscreen do Playnite 10 (um pacote `.pthm` é um zip), incluindo um popular e um parcial.
3. Para cada um: `dotnet run --project tools/Launcher.ThemeCheck -- <pasta-do-tema>` e depois com `--load`.
4. Registrar aqui, por tema: arquivos aceitos/total e as violações mais frequentes (o resumo no fim da saída), além dos `xmlns` declarados que não são do WPF.
