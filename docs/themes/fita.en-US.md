# Ribbon Theme (Fita / Modern Console)

[Português (Brasil)](fita.md)

**Ribbon** (Fita) is an official full-screen theme for Tako Launcher (PRD item THEME-008), inspired by modern living room consoles (PS5, Apple TV).

The focused game artwork reigns over the screen with cinematic presence, complemented by a minimalist top HUD and a continuous horizontal ribbon of compact cards anchored in the bottom third of the display.

Source code: [`crates/tako-ui/theme/extras/tako-fita/`](../../crates/tako-ui/theme/extras/tako-fita/)

---

## 1. Visual Identity

- **Palette:** Deep slate graphite canvas (`#0D0F14`), dark translucent glass surfaces (`#141821a6`), pearl white focus ring (`#F5F7FA`) with a dynamic ambient halo tint matching the active game (`Ui.accent`). Text contrast meets or exceeds WCAG AA (≥ 4.5:1) everywhere, with a dedicated dark scrim behind title and metadata areas over bright artwork.
- **Typography:**
  - Main titles in **Poppins** (Bold / ExtraBold);
  - Metadata, buttons, numbers and labels in **Plus Jakarta Sans** (Medium / SemiBold / Bold).
- **Motion & Effects:**
  - Focus expansion (1.12x scale) with subtle elevation shadow;
  - Smooth continuous horizontal sliding along the card ribbon, keeping the focused card centered;
  - Strict support for "Reduce motion" (`Ui.reduce-motion` sets animation durations to zero) and text scaling (`Ui.text-scale`).

---

## 2. Screens

### Home (Início)
- **Top and Middle Third:**
  - Full-screen game artwork with gentle vignettes and dedicated dark scrim for comfortable reading;
  - Game title (or official transparent logo), status indicator (running, continue playing, installed), metadata (play time, store, last session);
  - Primary quick action buttons (`Play` and details icon).
- **Bottom Third (The Ribbon):**
  - Collection / active row title and fast collection switcher tabs;
  - Horizontal carousel of compact cards with zoom and illuminated halo on the focused card (focused item centered on the ribbon);
  - Unfocused cards display title and metadata over a subtle bottom gradient scrim;
  - "See all" card at the end of each row to open the complete collection.

### Library (Biblioteca)
- Clean top HUD with view title and total game count;
- Chip bar for filters and sorting (All, Installed, Favorites, Filters, Sort);
- Traditional cinematic grid with 5 columns in 720p (symmetrically centered with balanced side margins, eliminating asymmetric empty space on the right);
- Focus ring with card expansion (1.08x), pearl white luminous border and halo in the game's accent color;
- Grid cards display title and metadata at the base.

### Sidebar, Details, and Control Center
- **Sidebar:** Maintains a narrow, mouse-accessible clickable rail and stays collapsed by default to preserve width for artwork and ribbon. Smoothly expands when focused via D-pad Left, mouse hover/click, or the Back button.
- **Details and Settings:** Consistent dark slate surfaces, guaranteed contrast, and cinematic typography. Headers and action buttons are responsive, accommodating text scales up to 1.5x without truncation or clipping.
- **Control Center:** Fully integrated with transparent background support over active games.

---

## 3. Controller and Keyboard Navigation

| Input | Home | Library |
|---|---|---|
| **D-pad Left / Right** | Navigates the horizontal ribbon (keeps focused item centered) | Moves across grid columns |
| **D-pad Up** | Moves focus up to primary buttons (`Play` and details icon) | Moves focus up to filter/sort chips |
| **D-pad Down** | Moves focus down across collection rows | Moves focus down to the grid |
| **A Button (Accept)** | Activates the focused button or opens game details | Opens game Details page |
| **B Button (Back)** | Returns focus to top action buttons or opens Sidebar | Returns focus to chips or opens Sidebar |
| **X Button (Options)** | Opens quick actions dialog for featured game | Opens quick actions dialog for focused game |
| **Y Button (Search)** | Opens search keyboard | Opens search keyboard |

---

## 4. Verification and Contract v1

The theme implements the public fullscreen theme API v1 (`App`) and can be validated with:

```bash
tako theme check crates/tako-ui/theme/extras/tako-fita
```

### Compatibility and Contract v1 Notes
`ControlCenter`, `OverlayView`, and `RowView` are internal helper components imported from the standard `@tako/` library for layer, quick menu, and settings support, but **are not part of the formal theme API v1 contract**.
