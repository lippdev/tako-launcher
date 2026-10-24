# Navegação por direção (WEB-05a)

Setas movem o foco para o vizinho mais próximo naquela direção; Enter e Espaço ativam; Esc e Backspace voltam;
Q e E trocam de seção. Mesmo mapa do app atual (`crates/tako-ui/src/shell/keys.rs`).

| Arquivo | O que é |
|---|---|
| `spatial.ts` | Escolha do próximo por geometria. Puro (retângulos, sem DOM), testado em `spatial.test.mjs`. |
| `keymap.ts` | Tecla → ação (`NavAction`), e a tecla que o componente focado entenderia no lugar de uma ação. |
| `engine.ts` | DOM: candidatos, camadas, memória por região, foco com rolagem, recuperação do foco. |
| `useSpatialNavigation.ts` | Liga o teclado e o resto no `AppShell`. Uma vez. |

## Marcas no DOM

- `data-focus-region="top|list|content|categories"`: região. Vale o mais próximo ancestral. A seta anda dentro da
  região; só atravessa quando ela acaba, e só para uma região que fica naquela direção e se sobrepõe ao foco no outro
  eixo (não se pula na diagonal). Ao atravessar para uma região que já teve foco, volta ao elemento de onde saiu.
- `data-focus-initial`: a região onde cai o primeiro foco, quando nada está focado.
- `data-focus-group`: fileira rolável (as capas). Quem está nela alcança as vizinhas escondidas pela rolagem; quem está
  fora só enxerga as que aparecem.
- `data-focus-skip`: o elemento (ou a árvore) não entra na navegação por direção. Continua no Tab. Para controle que só
  duplica o que o foco já faz (as setas ‹ › das fileiras, os botões da janela).
- `data-focus-sections="regiao …"`: conjunto de seções que Q e E percorrem (as categorias das Configurações); vale
  para a região dele e para as listadas. `role="tablist"` já é um conjunto.

Só entra na navegação o que o Tab alcança: `tabindex="-1"` (roving: abas não marcadas, itens de menu) fica de fora.

## Quem trata a seta: o componente primeiro

**Regra:** a tecla chega primeiro ao componente focado. Se ele a trata, chama `event.preventDefault()` e o navegador não
faz nada. Se não chama, o navegador de foco age (ouve no `window`, depois dos componentes, e olha `defaultPrevented`).

Para a borda passar o foco adiante, o componente **não** chama `preventDefault` quando a seta não tem para onde ir:

- `ListaDeJogos`: ↑ na primeira linha e ↓ na última (um aperto novo; segurar a tecla até a ponta não pula para fora).
- `Abas`: ← na primeira e → na última (não dá a volta).
- `ControleDeslizante`: ← no mínimo e → no máximo (também só no aperto novo). ↑ ↓ não são do deslizante: ficam para
  a navegação entre as linhas.
- `CampoDeSelecao`: ↑ ↓ não abrem mais o menu (Enter e Espaço abrem), para as linhas das Configurações serem
  alcançáveis por ↑ ↓.
- Categorias das Configurações: ↑ ↓ nas pontas.
- Campo de texto: ← e → são do texto até o cursor chegar à ponta; ↑ ↓ navegam. Letras, Espaço e Backspace são do
  campo. Esc limpa o campo com texto; com o campo vazio, sai dele.

Componente novo com teclado próprio segue a mesma regra: trata, `preventDefault`, e na borda deixa passar.

## Camadas

Diálogo, painel e menu prendem a navegação dentro deles (`escapeLayers.hasOpenLayer`): os candidatos são só os da
camada que contém o foco. O Esc e o "voltar" fecham a de cima (`closeTopLayer`); quem fecha devolve o foco a quem abriu
(`focusTrap`, `MenuDeContexto`).

## Ações e o ponto único

`dispatchNavAction(action, source)` em `engine.ts` é o único lugar que executa ações. O teclado chega pelo
`useSpatialNavigation`; o controle chega pela ponte (`source: "controller"`), e a ação é antes oferecida ao componente
focado como a tecla equivalente (`keyForAction`), para lista, abas, deslizante e menu continuarem donos das setas.

`// TODO(WEB-05b)` em `useSpatialNavigation.ts`: o evento do núcleo (SDL3) que chama `dispatchNavAction`.

"Voltar" quando nada acima tratou: `setBackHandler`, registrado pelo `AppShell` (página do jogo → Visão geral; outras
vistas → biblioteca).

## Foco visível e rolagem

- `document.documentElement.dataset.input` = `keyboard | controller | mouse`. O mouse (`pointerdown`) apaga o foco
  visível; o controle usa `focus({ focusVisible: true })`. O desenho do foco é o de sempre (`focus.module.css`,
  `focus-fill`, `focus-ring`, `focus-glow`).
- O elemento focado entra na tela com `scrollIntoView({ block: "nearest" })`, suave, e instantâneo com "Reduzir
  animações". A folga é o `scroll-margin` de `base.css`.
- Se o foco cai no `body` (a página do jogo fechou, a categoria trocou) e a última entrada foi teclado ou controle,
  volta ao último foco da região, ou ao primeiro dela.
