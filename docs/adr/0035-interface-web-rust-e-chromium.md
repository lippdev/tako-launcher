# ADR-0035: Interface web (React sobre Chromium embutido) com o núcleo em Rust

- Status: Aceita (decisão do mantenedor em 2026-10-24); o motor no Windows e as metas finais dependem do gate do MIG-W01
- Data: 2026-10-24
- Substitui: a parte de interface do [ADR-0023](0023-slint-gate.md) (Slint) e a D-02 do PRD

## Contexto

O Tako em Rust chegou à paridade com a interface em Slint (MIG-029) e absorveu o design system (UX-007). No uso
real, o mantenedor encontrou problemas de acabamento que o Slint não resolve bem, e que não aparecem nos testes
automáticos:

1. **Texto "granulado" em animações.** Menus e dropdowns abrem com `transform-scale` sobre o conteúdo
   (`desktop.slint`, menu com `layer: false`). A cada quadro o texto é redesenhado num tamanho fracionário, e os
   glifos "fervem". O mesmo padrão existe em diálogos, cards e páginas. Um navegador compõe a camada pronta na GPU
   e não refaz o texto durante a animação.
2. **Qualidade do texto parado.** O renderizador em uso (FemtoVG, OpenGL) rasteriza o texto de forma visivelmente
   mais pobre que o Chromium (DirectWrite/Skia).
3. **Telas com frame rate baixo.** Algumas seções (vidro com desfoque, muitas camadas animadas) caem de 60 fps. O
   Slint dá pouca ferramenta para achar o gargalo; o Chromium tem o painel de desempenho do DevTools.
4. **Velocidade de trabalho.** O mantenedor domina React, CSS e WebView2 e já tem um app Rust + WebView2 com esse
   tipo de animação funcionando bem. O Slint tem menos componentes prontos, menos exemplos e uma comunidade menor;
   cada detalhe de polimento custa mais.

Alternativas consideradas:

| Alternativa | Por que não |
|---|---|
| Refinar no Slint (tirar escala de texto, renderizador Skia) | Resolve o item 1 e melhora o 2, mas não os itens 3 e 4; o teto de polimento continua incerto |
| GPUI (Zed) | Texto bom, mas pré-1.0, API instável, pouca documentação, Windows recente, sem tema em tempo de execução e reescrita do mesmo tamanho |
| WebView do sistema em todo lugar (Tauri padrão) | No Linux é o WebKitGTK, bem pior em animação e desempenho que o Chromium; a experiência deixaria de ser igual nos três SOs (princípio §1.3) |
| Electron com o núcleo Rust à parte | Traz o Node.js inteiro e é o modelo que gera a fama de "app pesado" |

Referência de mercado: desde 2022 o **Steam Big Picture** é a interface do Steam Deck, feita em React e
desenhada por um Chromium embutido (CEF, processo `steamwebhelper`); em 2023 o cliente desktop do Steam inteiro
foi para a mesma base. Battle.net, EA app e Epic também usam CEF, e o Heroic usa Electron.

## Decisão

1. **A interface passa a ser web: React + TypeScript, desenhada por um Chromium embutido (CEF)**, no modelo do
   Steam. O usuário não instala navegador nem WebView: o Chromium vai junto com o Tako. Mesmo motor nos três SOs.
2. **O núcleo continua em Rust e é a fonte da verdade.** `tako-core`, `tako-sources`, `tako-platform`,
   `tako-themes`, `tako-downloads` e o supervisor não mudam de papel. O React desenha e guarda só o estado de
   tela (foco, navegação, animação); dados e ações vivem no Rust.
3. **Ponte tipada, sem porta de rede.** React ↔ Rust por mensagens do próprio CEF (comandos, consultas e
   eventos), com os tipos TypeScript gerados a partir dos tipos Rust. Imagens e capas saem do cache do Rust por
   um esquema próprio (`tako-img://`), sem passar por HTTP nem por base64. A API local (ADR-0033) continua sendo
   para integrações externas, não para a interface.
4. **Interface descartável.** Durante o jogo, minimizado e na bandeja, os processos do Chromium terminam e fica só
   o núcleo Rust (continua o item 2 do ADR-0023). Ao voltar, a interface reabre e o React restaura a tela a
   partir do estado do Rust.
5. **Motor no Windows decidido por números.** O padrão é o CEF nos três SOs. No gate do MIG-W01 o mesmo React
   também roda sobre o WebView2 (que já é Chromium, vem no Windows e a Microsoft atualiza); se ele empatar em
   qualidade e desempenho, o Windows usa o WebView2 e economiza o tamanho e as atualizações do Chromium. A
   decisão volta ao mantenedor com os números.
6. **Instalador no modelo do Steam** (PRD D-11): um instalador pequeno baixa e confere os componentes (núcleo,
   runtime do Chromium e pacote da interface) e emenda no onboarding. Os componentes se atualizam separados.
7. **Slint congelado até a paridade da interface web** (PRD D-12): só correções críticas, como a D-08 fez com o
   WPF.

## Consequências

Ganhos esperados (medidos no gate e na paridade, PRD §8.5):

- Texto e animações com o acabamento do Chromium: composição em camadas na GPU, animação de `transform` e
  `opacity` sem refazer o texto e DevTools para medir cada tela.
- O mesmo desenho nos três SOs, sem depender do driver OpenGL de cada máquina.
- Itens do backlog ficam mais baratos: trailers e arte animada (UX-002) com `<video>`, sem libmpv; controle remoto
  pelo celular (STREAM-002) reaproveitando os componentes; acessibilidade com leitor de tela pela árvore de
  acessibilidade do Chromium.
- Hot reload do Vite e DevTools no desenvolvimento.

Custos e riscos aceitos:

- **Tamanho:** o Chromium soma cerca de 150 a 250 MB à instalação. O instalador por componentes (item 6) evita
  baixar de novo quando só a interface muda.
- **Atualizações de segurança:** quem embute o Chromium responde por ele. Toda versão estável do CEF com correção
  de segurança vira release do runtime do Tako em poucos dias (processo no MIG-W40).
- **Memória com a interface aberta:** o Chromium tem processos próprios (principal, GPU, renderização). A meta é
  medida contra a linha de base do Slint (MIG-W00) e compensada pela interface descartável (item 4).
- **Reescrita da camada de apresentação:** cerca de 47 mil linhas de Rust da `tako-ui` (estado das telas, foco,
  diálogos, configurações) e 13,7 mil linhas de `.slint`. A lógica que é regra de negócio sai para uma camada de
  serviços em Rust (`tako-app`) e é reaproveitada; o resto é refeito no React.
- **Temas mudam de formato de novo.** Temas `.slint` deixam de funcionar. As paletas (`tokens.toml`, THEME-003 e
  THEME-006) continuam e viram variáveis CSS. Layouts alternativos (como o Fita) passam a ser layouts oficiais em
  React. Tema continua sem executar código: CSS, tokens e arquivos, sem JavaScript, bloqueado por CSP (PRD §7).
- **A validar no spike:** bindings Rust do CEF (crate `cef`), CEF no Wayland e no gamescope, Linux arm64, e
  empacotamento e notarização do macOS com os processos auxiliares do CEF.
