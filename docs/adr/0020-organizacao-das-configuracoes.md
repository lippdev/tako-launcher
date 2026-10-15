# ADR-0020: Organização das Configurações e menu Modo Console

- Status: Proposta
- Data: 2026-10-15

## Contexto

Na alpha.9 as Configurações eram uma lista única com mais de 30 linhas (título, descrição e valor à direita) sobre a
arte do jogo. O mantenedor, testando com controle e teclado numa tela 1600×1200, relatou: navegação ruim, item focado
cortado pela barra de dicas, barra de dicas cortada à direita, valores ilegíveis sobre a arte, textos de
desenvolvedor visíveis ("(MVP-007)", "(H2)", "carregador XAML restrito", "Temas do Playnite 10 chegam no Marco 1"),
itens duplicados ("Tema" que só recarregava o tema, "Temas › Gerenciar", "Prévia de tema Playnite 10") e "Remover
permissão do contador de FPS" solto no meio. Tudo que veio do Console Mode (tela de jogo, TV, áudio, Steam, contador e
limite de FPS) estava misturado com as preferências do launcher. O PRD pede configurações completas na interface de
tela cheia (UX-006) e acessíveis (UX-009).

Duas causas técnicas do "cortado":

- A rolagem que acompanha o foco (`FocusNavigator.KeepVisible`) comparava a posição da linha **dentro da lista** com
  a faixa segura **da tela**. A lista das Configurações começava 140 unidades abaixo do topo, então uma linha podia
  ficar até 140 unidades além da faixa: atrás da barra de dicas. A rolagem também lia o deslocamento no meio da
  animação (não o destino), errando a conta com o direcional apertado rápido.
- A barra de dicas era um `StackPanel` alinhado à direita: sem largura (4:3, 16:10, escala alta, glifos de teclado
  como "Shift+Tab"), o WPF cortava o fim (o "Q" e o "E" cortado do relato).

## Decisão

1. **Configurações em categorias, estilo console** (PS5/Xbox): coluna de categorias à esquerda e a categoria escolhida
   à direita. Categorias: **Geral** (abrir com o Windows, idioma, atualizações), **Biblioteca e lojas** (adicionar,
   procurar, lojas, coleções, informações dos jogos), **Aparência** (um único lugar para temas — escolher, prévia,
   importar, remover —, cor dinâmica, reduzir animações), **Controles** (controle em uso e atalhos; a captura dos
   atalhos da Central entra aqui), **Sistema** (restaurar telas e som, diagnóstico, teste de controle, registros, modo
   seguro, Xbox mode, opções de desenvolvedor) e **Sobre** (versão, licença AGPL, créditos, código-fonte).
   - Focar uma categoria mostra o conteúdo dela; → ou A entram na primeira linha; LB/RB trocam de categoria (dão a
     volta); B no conteúdo volta para a categoria e B na coluna sai da página.
   - As linhas são dados (`SettingRowViewModel`: chave, tipo — ação, chave liga/desliga, escolha, informação, título
     de grupo —, título, descrição, valor, visível, esmaecida). A tela relê o estado real a cada mudança; nada fica
     guardado só na tela.
   - **Opções de desenvolvedor** (desligadas por padrão, preferência `DeveloperOptions`) escondem "Recarregar o tema
     do Tako", "Prévia do tema Playnite embutido" e "Testar a tela de recuperação".
   - Textos reescritos em linguagem de usuário, sem IDs do PRD, das ADRs ou de marcos, e sem nomes técnicos.
2. **Menu Modo Console** na barra lateral (acima de Configurações e no ciclo de LB/RB), fora das Configurações: um
   cartão por recurso — Tela de jogo, TV, Áudio do jogo, Steam na bandeja, Contador de FPS e Limite de FPS — com um
   selo de estado (Ligado, Desligado, Falta configurar, Com problema; regras em `ConsoleFeatures`, testadas). A abre o
   detalhe do recurso (as linhas dele; a TV e o áudio abrem os editores que já existiam) e B volta ao cartão.
   - **Assistente "Configurar para jogar na TV"**: escolher a tela (a que não é a principal é sugerida) → testar a
     imagem (o Tako aparece na tela por alguns segundos com "Você consegue ver esta tela?" e as telas voltam) → TV
     (abre o editor da TV ou pula) → áudio (sugere a saída HDMI com o nome da tela) → pronto (resumo). Cada passo já
     salva; B volta um passo.
   - Ligar o contador de FPS e remover a permissão continuam só com teclado e mouse (ADR-0017): com o controle as
     linhas ficam esmaecidas e explicam.
3. **Layout que não corta**:
   - A rolagem trabalha em coordenadas do palco, com a lista sem rolagem, e a faixa visível é a interseção da faixa
     segura (abaixo do cabeçalho, acima da barra de dicas) com a moldura com recorte mais próxima, menos 16 de folga
     para o anel de foco (`ScrollMath.Vertical`, testada). O destino da rolagem é guardado, não lido da animação.
   - As listas das Configurações e do Modo Console rolam numa moldura que termina 12 unidades acima da barra de dicas.
   - A barra de dicas usa o `HintBarPanel`: sem espaço, abrevia (texto curto e dicas mais próximas) e depois esconde
     as menos importantes (seção, busca), nunca a de A/B (`HintFit.Plan`, testada).
   - As páginas escurecem a arte do jogo (`SettingsScrim`) e os valores ficam numa pílula `surface-2`.
4. **Compatibilidade**: as chaves salvas no `tako.db` (e o `updates.json`) não mudam. As chaves de foco (`Nav.Key`)
   das Configurações continuam `settings:...`; as do Modo Console são `console:...`. Pedir o foco de uma linha de
   outra categoria (voltar pelo histórico, testes) troca para a categoria dela (`IKeyRevealer` + `RevealHost`).

## Consequências

- Uma opção nova é uma linha na categoria certa (`SettingsViewModel.Categories.cs`) ou no detalhe do recurso
  (`ConsoleModeViewModel.*.cs`), sem XAML novo.
- Duas listas (Configurações e Modo Console) em vez de uma; o que é do Console Mode tem um só lugar.
- A barra de dicas pode esconder dicas de atalhos em telas estreitas; os atalhos continuam funcionando.
- Corrigida na auditoria: gravar `updates.json` com falha de disco derrubava o Tako ao tocar em "Atualizar
  automaticamente"/"Receber versões de teste"; agora a escolha vale na sessão e um aviso explica.
- O teste da imagem do assistente mexe nas telas de verdade (como o primeiro jogo já fazia); validar no checklist de
  hardware.
