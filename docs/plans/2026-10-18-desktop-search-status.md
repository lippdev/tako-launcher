# Desktop search, filters and game status implementation plan

> **For agentic workers:** Implement this plan task-by-task, in order, checking off each step as it passes.

**Goal:** Apply the desktop search, filter count/chips and game status design from 71ad29a.

**Architecture:** Reuse SearchBox, library criteria, PlayStatus and existing desktop menu layers. Keep desktop behavior gated and leave TopBar to the parallel implementation.

**Tech Stack:** Rust and interpreted Slint 1.18.

**Spec:** /tmp/tako-brief/common.md and the requested component READMEs at tako-design commit 71ad29a.

## Global Constraints

- Desktop only; no design reference files copied into this public repository.
- New UI copy in pt-BR and en-US; animation duration uses Ui tokens.
- One commit per requested item, in order; No push or PR; return commits for integration.

## Review Focus

- Keyboard focus follows the visible search in every implemented desktop page.
- Small windows retain usable search and list space.
- Removing one chip preserves other filters and the current search.
- Status changes retain valid selection when a filtered game disappears.
- Status defaults only change on a successful first launch and preserve user choices.

### Task 1: Search outside the top bar

- [x] Add panel SearchBox instances to DeskLibrary and DeskSettings and floating search to Profile, with shared focus routing and Escape/Down/Enter handling.
- [x] Keep settings search logic; filter existing profile session/collection data without changing totals.
- [ ] Verify focus/filtering and render 1600×900 and 960×600 library, grid, categories, settings and profile captures.
- [x] Commit item 2 in Portuguese.

### Task 2: Count alignment and active filters

- [x] Set count padding to 9/8, targets to 30×30, spacing to 2 and icons to 17; remove filter badge.
- [x] Build fixed chips from LibraryCriteria with individual removal and clear-all, preserving search/sort.
- [ ] Test removal and render both requested sizes with multiple active filters.
- [x] Commit item 5 in Portuguese.

### Task 3: Game status

- [ ] Add compact status pill beside StorePill, hide both on small windows/phase 2 and reuse anchored menu presentation.
- [ ] Add desktop status select in Mostrar, including Sem status, keeping TV filters unchanged; add bulk status action.
- [ ] Confirm category grouping/context status paths; implement first-launch transition if absent and test persistence.
- [ ] Render both sizes and status/filter/bulk menus, then run cargo fmt, UI/core tests and workspace clippy.
- [ ] Commit item 6 in Portuguese and return all three commits.
