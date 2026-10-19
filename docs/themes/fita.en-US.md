# Ribbon Theme (Fita / Modern Console)

[Português (Brasil)](fita.md)

**Ribbon** (Fita) is an official full-screen theme for Tako Launcher (PRD item THEME-008), inspired by modern living room consoles (PS5, Apple TV).

The focused game artwork reigns over the screen with cinematic presence, complemented by a minimalist top HUD and a continuous horizontal ribbon of compact cards anchored in the bottom third of the display.

Source code: [`crates/tako-ui/theme/extras/tako-fita/`](../../crates/tako-ui/theme/extras/tako-fita/)

---

## 1. Visual Identity

- **Palette:** Deep slate graphite canvas (`#0D0F14`), dark translucent glass surfaces (`#141821a6`), pearl white focus ring (`#F5F7FA`) with a dynamic ambient halo tint matching the active game (`Ui.accent`).
- **Typography:**
  - Main titles in **Poppins** (Bold / ExtraBold);
  - Metadata, buttons, numbers and labels in **Plus Jakarta Sans** (Medium / SemiBold / Bold).
- **Motion & Effects:**
  - Focus expansion (1.12x scale) with subtle elevation shadow;
  - Smooth continuous horizontal sliding along the card ribbon;
  - Strict support for "Reduce motion" (`Ui.reduce-motion` sets animation durations to zero) and text scaling (`Ui.text-scale`).

---

## 2. Screens

### Home (Início)
- **Top and Middle Third:**
  - Full-screen game artwork with gentle vignettes for legibility;
  - Game title (or official transparent logo), status indicator (running, continue playing, installed), metadata (play time, store, last session);
  - Primary quick action buttons (`Play` and `Details`).
- **Bottom Third (The Ribbon):**
  - Collection / active row title and fast collection switcher tabs;
  - Horizontal carousel of compact cards with zoom and illuminated halo on the focused card;
  - "See all" card at the end of each row to open the complete collection.

### Library (Biblioteca)
- Clean top HUD with view title and total game count;
- Chip bar for filters and sorting (All, Installed, Favorites, Filters, Sort);
- Traditional cinematic grid with dynamic column layout (5 columns in 720p, up to 7 in 1080p);
- Focus ring with card expansion (1.08x), pearl white luminous border and halo in the game's accent color.

### Sidebar, Details, and Control Center
- **Sidebar:** Remains collapsed by default to dedicate 100% of the display width to artwork and ribbon. Smoothly expands when focused via D-pad Left or the Back button.
- **Details and Settings:** Consistent dark slate surfaces and cinematic typography.
- **Control Center:** Fully integrated with transparent background support over active games.

---

## 3. Controller and Keyboard Navigation

| Input | Home | Library |
|---|---|---|
| **D-pad Left / Right** | Navigates the horizontal ribbon (keeps focused item centered) | Moves across grid columns |
| **D-pad Up** | Moves focus up to primary buttons (`Play` and `Details`) | Moves focus up to filter/sort chips |
| **D-pad Down** | Moves focus down across collection rows | Moves focus down to the grid |
| **A Button (Accept)** | Activates the focused button or opens game details | Opens game Details page |
| **B Button (Back)** | Returns focus to top action buttons or opens Sidebar | Returns focus to chips or opens Sidebar |
| **X Button (Options)** | Opens quick actions dialog for featured game | Opens quick actions dialog for focused game |
| **Y Button (Search)** | Opens search keyboard | Opens search keyboard |

---

## 4. Verification

The theme implements API v1 and can be validated with:

```bash
tako theme check crates/tako-ui/theme/extras/tako-fita
```
