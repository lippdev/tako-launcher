# Barra única do desktop — Implementation Plan

> **For agentic workers:** Implement this plan task-by-task, in order, checking off each step as it passes.

**Goal:** Implementar o item 1 da referência Tako 71ad29a.
**Architecture:** Reutilizar TopBar, Vidro, ações winit e menus de perfis. Publicar a geometria real do vazio para o hit-test nativo, sem estimar larguras de textos.
**Tech Stack:** Rust, Slint, winit, Win32.
**Spec:** components/BarraDaJanela/README.md e components/BarraSuperior/README.md do tako-design em 71ad29a (referência externa, somente leitura).

## Global Constraints
- Só desktop; nenhuma mudança na TV.
- Barra 64; controles 46; conteúdo 20 abaixo; menu 8 abaixo.
- Commit único em português, sem push nem PR.
- CARGO_TARGET_DIR compartilhado; CARGO_INCREMENTAL=0.

## Review Focus
- Janela 960×600: controles visíveis e nome oculto.
- Foco do teclado alcança perfil e Tela cheia, sem entrar nos controles nativos.
- Hit-test não captura guias ou botões da aplicação.
- Trocar/editar perfil usa persistência e validação existentes.
- Windows: comportamento nativo precisa de validação posterior no sistema real.

### Task 1: Barra, menus e integração nativa
**Files:** desktop.slint, tokens.slint, shell/{mod,desktop,profile}.rs, app.rs, módulo de hit-test e locales desktop.
**Interfaces:** TopBar publica limites do vazio; window-action mantém drag/min/max/close/menu/update; desk-profile-menu abre MenuLayer.
- [x] Unir barra, controles, perfil e geometrias, retirando busca.
- [x] Integrar menu e navegação por teclado; hit-test Windows.
- [ ] Testar UI/core, fmt e clippy; capturar 1600×900 e 960×600 e comparar referências.
- [x] Revisar diff e criar commit único.

## Validação e entrega

`cargo fmt --all` e `git diff --check` concluídos. A prévia de BarraDaJanela foi aberta com tokens.css e bundle.css da referência, confirmando a geometria da barra. Testes de hit-test e navegação adicionados.

A última execução anterior à pausa teve 122 testes aprovados e quatro falhas (inicial calculada no Slint e catálogo compartilhado desatualizado). A inicial agora vem do Rust. Por instrução posterior do coordenador, novos builds foram suspensos; UI/core, clippy e snapshots finais 1600×900/960×600 ficam para a validação integrada. O target compartilhado pode conter arquivos gerados com caminhos de outro worktree; conferir a origem do tema e dos catálogos antes de validar. Windows não foi compilado nem executado neste Mac: conferir snap, DPI, duplo clique, menu do sistema e comandos nativos em Windows 11.
