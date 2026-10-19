# Tema Fita (Ribbon / Console Moderno)

[English](fita.en-US.md)

O **Fita** é um dos temas oficiais em tela cheia do Tako Launcher (item THEME-008 do PRD), projetado com a metáfora de consoles de sala contemporâneos (PS5, Apple TV).

A arte do jogo em foco reina no topo com presença cinematográfica, complementada por uma barra de status (HUD) minimalista e uma fita horizontal contínua de cartões compactos ancorada no terço inferior da tela.

Código-fonte: [`crates/tako-ui/theme/extras/tako-fita/`](../../crates/tako-ui/theme/extras/tako-fita/)

---

## 1. Identidade Visual

- **Paleta:** Fundo cinza ardósia grafite profundo (`#0D0F14`), superfícies translúcidas em vidro escurecido (`#141821a6`), anel de foco em branco pérola (`#F5F7FA`) com halo suave e dinâmico na cor do jogo em foco (`Ui.accent`).
- **Tipografia:**
  - Títulos principais em **Poppins** (Bold / ExtraBold);
  - Metadados, botões, números e rótulos em **Plus Jakarta Sans** (Medium / SemiBold / Bold).
- **Movimento e Efeitos:**
  - Animação de foco com expansão (escala 1.12x) e elevação com sombra suave;
  - Deslize contínuo horizontal do trilho de cartões;
  - Respeito integral a "Reduzir animações" (`Ui.reduce-motion` zera os tempos de animação) e à escala de texto (`Ui.text-scale`).

---

## 2. Telas

### Início (Home)
- **Terço Superior e Médio:**
  - Arte de fundo em tela cheia com vinhetas suaves para leitura;
  - Título do jogo (ou logotipo oficial), indicador de status (em execução, continuar jogando, instalado), metadados (tempo de jogo, loja, última sessão);
  - Botões de ação primária rápida (`Jogar` e `Detalhes`).
- **Terço Inferior (A Fita):**
  - Título da coleção / fileira ativa e abas para troca rápida de fileira;
  - Carrossel horizontal de cartões compactos com zoom e halo iluminado no item selecionado;
  - Ação "Ver tudo" ao final da fileira para abrir a coleção completa.

### Biblioteca (Library)
- HUD superior limpo com título da visualização e contagem de jogos;
- Barra de chips para filtros e ordenação rápida (Todos, Instalados, Favoritos, Filtros, Ordem);
- Grade cinematográfica tradicional com número dinâmico de colunas (5 colunas em 720p, até 7 em 1080p);
- Foco com ampliação de card (1.08x), borda luminosa branco pérola e halo na cor de destaque do jogo.

### Barra Lateral, Detalhes e Central
- **Barra Lateral:** Permanece recolhida por padrão para liberar 100% da largura para a arte e a fita. Ao receber foco (via D-pad Esquerda ou botão B), expande suavemente.
- **Detalhes e Configurações:** Telas com identidade escura e tipografia cinematográfica.
- **Central de Controle:** Integrada com suporte transparente sobre jogos em execução.

---

## 3. Navegação por Controle e Teclado

| Comando | Início | Biblioteca |
|---|---|---|
| **D-pad Esquerda / Direita** | Percorre a fita horizontal de jogos (mantém o item focado centralizado) | Move entre as colunas da grade |
| **D-pad Cima** | Sobe o foco para os botões do topo (`Jogar` e `Detalhes`) | Sobe o foco para os chips de filtro/ordenação |
| **D-pad Baixo** | Desce o foco para as outras fileiras de coleções | Desce o foco para a grade |
| **Botão A (Aceitar)** | Executa a ação do botão focado ou abre detalhes do card | Abre a página de Detalhes do jogo |
| **Botão B (Voltar)** | Volta o foco para os botões do topo ou abre a Barra Lateral | Volta o foco para os chips ou abre a Barra Lateral |
| **Botão X (Opções)** | Abre o menu de ações rápidas do jogo em destaque | Abre o menu de ações rápidas do jogo focado |
| **Botão Y (Buscar)** | Abre o teclado de busca rápida | Abre o teclado de busca rápida |

---

## 4. Validação

O tema é compatível com a API v1 de temas do Tako Launcher e pode ser verificado com:

```bash
tako theme check crates/tako-ui/theme/extras/tako-fita
```
