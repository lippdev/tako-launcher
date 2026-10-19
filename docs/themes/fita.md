# Tema Fita (Ribbon / Console Moderno)

[English](fita.en-US.md)

O **Fita** é um dos temas oficiais em tela cheia do Tako Launcher (item THEME-008 do PRD), projetado com a metáfora de consoles de sala contemporâneos (PS5, Apple TV).

A arte do jogo em foco reina no topo com presença cinematográfica, complementada por uma barra de status (HUD) minimalista e uma fita horizontal contínua de cartões compactos ancorada no terço inferior da tela.

Código-fonte: [`crates/tako-ui/theme/extras/tako-fita/`](../../crates/tako-ui/theme/extras/tako-fita/)

---

## 1. Identidade Visual

- **Paleta:** Fundo cinza ardósia grafite profundo (`#0D0F14`), superfícies translúcidas em vidro escurecido (`#141821a6`), anel de foco em branco pérola (`#F5F7FA`) com halo suave e dinâmico na cor do jogo em foco (`Ui.accent`). Contraste de texto ≥ 4,5:1 em todos os elementos, com scrim escuro dedicado atrás das áreas de títulos e metadados sobre artes claras.
- **Tipografia:**
  - Títulos principais em **Poppins** (Bold / ExtraBold);
  - Metadados, botões, números e rótulos em **Plus Jakarta Sans** (Medium / SemiBold / Bold).
- **Movimento e Efeitos:**
  - Animação de foco com expansão (escala 1.12x) e elevação com sombra suave;
  - Deslize contínuo horizontal do trilho de cartões com item focado mantido centralizado na tela;
  - Respeito integral a "Reduzir animações" (`Ui.reduce-motion` zera os tempos de animação) e à escala de texto (`Ui.text-scale`).

---

## 2. Telas

### Início (Home)
- **Terço Superior e Médio:**
  - Arte de fundo em tela cheia com vinhetas suaves e scrim escuro dedicado para leitura confortável;
  - Título do jogo (ou logotipo oficial), indicador de status (em execução, continuar jogando, instalado), metadados (tempo de jogo, loja, última sessão);
  - Botões de ação primária rápida (`Jogar` e ícone de detalhes).
- **Terço Inferior (A Fita):**
  - Título da coleção / fileira ativa e abas para troca rápida de fileira;
  - Carrossel horizontal de cartões compactos com zoom e halo iluminado no item selecionado (com foco mantido centralizado na fita);
  - Cartões exibem título e metadados com scrim inferior suave mesmo quando não focados;
  - Ação "Ver tudo" ao final da fileira para abrir a coleção completa.

### Biblioteca (Library)
- HUD superior limpo com título da visualização e contagem de jogos;
- Barra de chips para filtros e ordenação rápida (Todos, Instalados, Favoritos, Filtros, Ordem);
- Grade cinematográfica tradicional com 5 colunas em 720p (centralizadas com margens proporcionais, sem espaço vazio assimétrico à direita);
- Foco com ampliação de card (1.08x), borda luminosa branco pérola e halo na cor de destaque do jogo;
- Cartões da grade exibem título e metadados na base.

### Barra Lateral, Detalhes e Central
- **Barra Lateral:** Mantém um trilho estreito acessível por mouse e recolhido para liberar largura para a arte e a fita. Ao receber foco ou clique (via D-pad Esquerda, mouse ou botão B), expande suavemente.
- **Detalhes e Configurações:** Telas com identidade escura, contraste garantido e tipografia cinematográfica. Cabeçalhos e botões de ação adaptados com elisão e layout responsivo que acomodam escala de texto até 1,5x sem cortes.
- **Central de Controle:** Integrada com suporte transparente sobre jogos em execução.

---

## 3. Navegação por Controle e Teclado

| Comando | Início | Biblioteca |
|---|---|---|
| **D-pad Esquerda / Direita** | Percorre a fita horizontal de jogos (mantém o item focado centralizado) | Move entre as colunas da grade |
| **D-pad Cima** | Sobe o foco para os botões do topo (`Jogar` e ícone de detalhes) | Sobe o foco para os chips de filtro/ordenação |
| **D-pad Baixo** | Desce o foco para as outras fileiras de coleções | Desce o foco para a grade |
| **Botão A (Aceitar)** | Executa a ação do botão focado ou abre detalhes do card | Abre a página de Detalhes do jogo |
| **Botão B (Voltar)** | Volta o foco para os botões do topo ou abre a Barra Lateral | Volta o foco para os chips ou abre a Barra Lateral |
| **Botão X (Opções)** | Abre o menu de ações rápidas do jogo em destaque | Abre o menu de ações rápidas do jogo focado |
| **Botão Y (Buscar)** | Abre o teclado de busca rápida | Abre o teclado de busca rápida |

---

## 4. Validação e Contrato v1

O tema implementa a API pública v1 de temas em tela cheia do Tako Launcher (`App`) e pode ser verificado com:

```bash
tako theme check crates/tako-ui/theme/extras/tako-fita
```

### Nota de Compatibilidade e Contrato v1
Os componentes `ControlCenter`, `OverlayView` e `RowView` são componentes auxiliares internos importados da biblioteca padrão `@tako/` para suporte a camadas, central rápida e opções de configurações, mas **não fazem parte do contrato formal da API v1 de temas**.
