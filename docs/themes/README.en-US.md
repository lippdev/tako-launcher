# Tako themes: guide and API reference

[Português](README.md)

A theme changes how Tako **looks**: colors, fonts, screen layout and animations. Data (games, covers, translated
texts, focus) and navigation stay with Tako. The theme receives everything ready through properties and reports
clicks and keys through callbacks. This page describes **theme API version 1**.

- Default theme (complete reference): [`crates/tako-ui/theme/`](../../crates/tako-ui/theme/)
- Sample theme that only changes colors: [`crates/tako-ui/theme/extras/tako-alto-contraste/`](../../crates/tako-ui/theme/extras/tako-alto-contraste/)
- Language of the screen files: [Slint](https://slint.dev/docs) (Tako uses the Slint 1.x interpreter)

## 1. Quick start

The shortest way to make a theme is to reuse the default one and change the palette:

```
my-theme/
├── theme.toml
├── main.slint
└── tokens.toml
```

`theme.toml`:

```toml
id = "author.my-theme"
name = "My theme"
version = "1.0.0"
author = "Your name"
api = 1
description = "Navy background."
```

`main.slint`:

```slint
export { App, Ui, Hint, PanelRow } from "@tako/main.slint";
```

`tokens.toml`:

```toml
canvas = "#0B1A33"
surface-1 = "#FFFFFF24"
```

Check it:

```
tako theme check my-theme
```

To try it, copy the folder into `<Tako data>/themes/` and pick the theme in **Settings › Appearance › Theme**.
Picking a theme shows a preview: it fills the screen, but only becomes Tako's theme if you confirm with **Apply**.
**Back**, or 15 seconds without an answer, restores the previous theme; if the theme fails to load, the previous one
stays.
To share it, zip the folder as a `.takotheme` (see [section 7](#7-package-and-distribution)).

## 2. What a theme can and cannot do

- **Can:** draw every screen however it wants, use its own images and fonts, and animate.
- **Cannot:** reach the network, the disk or other programs. Slint doesn't offer that, and Tako only loads files
  from inside the package. A theme can't change what the buttons do either: navigation belongs to Tako.
- **If it breaks:** Tako falls back to the default theme. Three load failures within 10 seconds trigger the
  fallback automatically, with no crash loop.

## 3. `theme.toml`

| Field | Required | Rule |
|---|---|---|
| `id` | yes | Letters, digits, `.`, `-` and `_`, up to 128 characters. Can't start or end with `.` or be a reserved Windows name (`CON`, `NUL`, `COM1`…). `default` belongs to Tako. Use `author.theme` to avoid clashes. |
| `name` | yes | Name shown in the theme list. |
| `version` | yes | Free text, shown next to the author (SemVer suggested). |
| `author` | yes | Who made it. |
| `api` | yes | API version: `1` or `"1.x"`. Only the major version counts. Any other major version is refused. |
| `entry` | no | Main `.slint` file, relative to the root. Default: `main.slint`. Can't leave the theme folder. |
| `description` | no | One sentence, shown in the list. |

The manifest is at most 64 KB. Unknown fields are ignored.

## 4. The `App` component

The entry file must **export an `App` component** (usually `inherits Window`). Tako fills its `in` properties by
name and wires the callbacks. **A property the theme doesn't declare is simply ignored**, so a theme can start
small and grow. Size units are up to the theme. The default one uses a 1080-line stage (`Ui.u = height / 1080`).

To avoid repeating types, import them from the default theme:

```slint
import { Ui, Hint, PanelRow } from "@tako/main.slint";
import { Tile, HomeRow, NavItem, Hero, Stat, Details, Overlay, Key } from "@tako/main.slint";
```

### 4.1 General

| Property | Type | What it is |
|---|---|---|
| `page` | `string` | Current page: `home`, `library`, `details` or `settings`. |
| `backdrop` | `image` | Background art of the focused game (may be empty). |
| `clock` | `string` | Time, already formatted (`21:30`). |
| `input-label`, `input-icon` | `string` | Input in use (name and icon: `gamepad` or `keyboard`). |
| `pad-battery` | `string` | Wireless controller battery (`"80%"`), or empty. |
| `toast` | `string` | Current short notice, or empty. |
| `hints` | `[Hint]` | Bottom bar hints (button glyph and text), in order. |
| `transparent-background` | `bool` | The window is over the game: draw only what's needed, on a transparent background. |
| `leaving` | `bool` | Leaving for the game: Tako closes after `Ui.leave-time`. Ideal for a fade. |
| `intro` | `bool` | App intro: true for about 1 s on launch only, with the first screen already ready underneath. Cover it with a short animation and fade it out when it turns false. Any key, button or `clicked` (from any zone) skips the intro; the theme may use the `intro` zone for the veil area. It is never true with "Reduce animations" nor in a theme that does not declare the property. |

### 4.2 Sidebar

| Property | Type | What it is |
|---|---|---|
| `sidebar` | `[NavItem]` | Items (Home, Library, collections, Settings…). |
| `sidebar-focus` | `int` | Focused item; `-1` = the sidebar doesn't have focus. |
| `sidebar-visible` | `bool` | Show the sidebar. |

### 4.3 Home

| Property | Type | What it is |
|---|---|---|
| `hero` | `Hero` | Top highlight (focused game or one to continue). |
| `hero-focus` | `int` | Focused hero button; `-1` = none. |
| `home-rows` | `[HomeRow]` | Cover rows. |
| `home-row-focus`, `home-col-focus` | `int` | Focused row and column (`-1` = outside the rows). |
| `home-empty` | `bool` | Empty library: show the empty state. |
| `home-empty-title`, `home-empty-text` | `string` | Empty state texts. |
| `home-empty-actions` | `[PanelRow]` | Empty state buttons. |
| `see-all-label` | `string` | "See all" text. |

### 4.4 Library

| Property | Type | What it is |
|---|---|---|
| `library-title`, `library-count` | `string` | Title and count. |
| `library-chips` | `[PanelRow]` | Filters and sorting (`kind` = `choice` or `action`). |
| `library-chip-focus` | `int` | Focused chip; `-1` = focus on the grid. |
| `library-tiles` | `[Tile]` | Grid covers. |
| `library-focus` | `int` | Focused cover. |
| `library-empty-title`, `library-empty-text` | `string` | Empty grid. |
| `library-columns` | `out int` | **Output:** how many columns fit. Tako uses it to move focus up and down. |

### 4.5 Details

| Property | Type | What it is |
|---|---|---|
| `details` | `Details` | Game title, texts and art. |
| `details-stats` | `[Stat]` | Numbers (time played, last played…). |
| `details-actions` | `[PanelRow]` | Buttons (Play, Favorite, Options…). |
| `details-focus` | `int` | Focused button. |

### 4.6 Settings

| Property | Type | What it is |
|---|---|---|
| `settings-title` | `string` | Page title. |
| `settings-categories` | `[NavItem]` | Categories. |
| `settings-category-focus` | `int` | Focused category. |
| `settings-heading`, `settings-subtitle` | `string` | Category title and description. |
| `settings-rows` | `[PanelRow]` | Category rows. |
| `settings-row-focus` | `int` | Focused row; `-1` = focus on the categories. |

### 4.7 Control Center

| Property | Type | What it is |
|---|---|---|
| `control-center` | `bool` | The Control Center is open. |
| `cc-icons` | `[PanelRow]` | Control Center bar icons. |
| `cc-icon-focus` | `int` | Focused icon. |
| `cc-panel-for` | `int` | Icon whose panel is open; `-1` = none. |
| `cc-panel` | `[PanelRow]` | Rows of the open panel. |
| `cc-panel-focus` | `int` | Focused panel row. |
| `cc-game-title`, `cc-game-meta` | `string` | Game in progress. |

With `control-center` and `transparent-background` both on, the Control Center is **over the game**: show only
its bar.

### 4.8 Layers (dialogs, panels, keyboard)

| Property | Type | What it is |
|---|---|---|
| `overlays` | `[Overlay]` | Open layers, bottom to top. The last one gets focus and clicks. |

`Overlay.kind`:

| `kind` | What it is | Fields used |
|---|---|---|
| `dialog` | Centered dialog | `title`, `overline`, `text`, `rows`, `sel`, `wide`, `image` |
| `panel` | Side panel | `title`, `overline`, `text`, `rows`, `sel` |
| `keyboard` | Virtual keyboard | `title`, `input`, `input-hint`, `secret`, `keys`, `key-cols`, `sel` |
| `art` | Art picker | `tabs`, `tab`, `tabs2`, `tab2`, `thumbs`, `thumb-cols`, `preview`, `preview-text`, `sel` |
| `launch` | "Opening…" | `title`, `text`, `image` |
| `progress` | Task in progress | `title`, `text` |
| `recovery` | Recovery screen | `title`, `text`, `rows`, `sel` |

`sel = -1` means "no selectable row" (read only).

Since THEME-002, an empty (or unknown) `kind` is a layer with no drawing, like the theme preview: draw nothing and let the `hints` bar
do the talking. Never cover the screen with a scrim for an unknown `kind`.

### 4.9 Callbacks

| Callback | When to call it |
|---|---|
| `clicked(zone: string, index: int)` | Click or tap on an item. |
| `hovered(zone: string, index: int)` | Pointer entered an item (focus follows the mouse). |
| `hint-clicked(index: int)` | Click on a bottom bar hint (index in `hints`). |
| `key(text, repeat, ctrl, shift, alt) -> bool` | Physical key (Slint `KeyEvent`). Return the result: `true` = Tako handled it. |
| `key-released(text)` | Key released. |

Zones Tako understands:

| Where | Zone | Index |
|---|---|---|
| Sidebar | `sidebar` | item |
| Home | `hero`, `row-N` (N = row), `empty` | button, column, action |
| Library | `chip`, `grid` | chip, cover |
| Details | `action` | button |
| Settings | `category`, `row` | category, row |
| Control Center | `cc-icon`, `cc-panel` | icon, row |
| Top layer | `overlay-row`, `overlay-key`, `overlay-tab`, `overlay-tab2`, `overlay-thumb`, `overlay-outside` | row, key, tab, thumbnail (outside the layer: `0`) |

For the controller and keyboard to work, `App` needs a focused `FocusScope` that forwards
`key-pressed`/`key-released` to the callbacks, as the default theme does (look for `fs.focus()` and `FocusScope`
in `main.slint`). The controller doesn't go through the theme.

## 5. Types

```slint
struct Hint { glyph: string, label: string }

// kind: toggle, choice, action, info, header, radio, check, slider, text, button
// style: "", "primary" or "danger"
struct PanelRow {
    key: string, kind: string, title: string, description: string, value: string,
    on: bool, enabled: bool, emphasized: bool, icon: string, style: string, image: image,
}

struct Tile {
    id: string, title: string, meta: string, cover: image, accent: color,
    installed: bool, favorite: bool, running: bool,
}

struct HomeRow { title: string, count: string, tiles: [Tile], see-all: bool }
struct NavItem { id: string, label: string, icon: string, count: string, section: string, selected: bool }
struct Hero {
    has-game: bool, title: string, overline: string, meta: string, description: string,
    art: image, logo: image, play-label: string, running: bool,
}
struct Stat { label: string, value: string }
struct Details {
    title: string, overline: string, meta: string, description: string,
    art: image, cover: image, logo: image, hint: string,
}
struct Key { label: string, row: int, col: int, span: int }
```

Texts arrive **already translated** into the chosen language. The theme doesn't need its own catalog.

`icon` (in `PanelRow` and `NavItem`) is a name from the default theme's set (`Icons.stroke(name)` and
`Icons.fill(name)` in `@tako/icons.slint`): `alert`, `battery`, `camera`, `check`, `chevron`, `close`, `filter`,
`folder`, `gamepad`, `gauge`, `gear`, `grid`, `home`, `image`, `info`, `keyboard`, `limit`, `monitor`, `more`,
`palette`, `play`, `pointer`, `power`, `search`, `sort`, `star`, `starfill`, `tv`, `volume`, plus the logos
`store-amazon`, `store-battlenet`, `store-ea`, `store-epic`, `store-gog`, `store-manual`, `store-steam`,
`store-ubisoft`, `store-xbox`. A theme can draw icons however it likes from the name.

## 6. The `Ui` global and `tokens.toml`

`Ui` (in `@tako/tokens.slint`) holds the default theme's tokens. Tako updates these fields, and a theme of your own
can read the same ones if it exports the default `Ui`:

| Field | What Tako sets |
|---|---|
| `accent`, `accent-deep` | Color of the focused game and its dark version (changes with every focus). |
| `reduce-motion` | Accessibility: no animations. Respect it (the default durations already become `0ms`). |
| `text-scale` | Chosen text size (1 = normal). |
| `pointer` | The last focus came from the mouse: highlight, but don't scroll lists to the item. |

In a theme that reuses the default one, **`tokens.toml`** changes the `in-out` colors, fonts and background of `Ui` without writing
Slint: `name = "#RRGGBB"` or `"#RRGGBBAA"`. Available colors: `accent`, `canvas`, `on-accent`, `on-accent-muted`,
`on-accent-subtle`, `surface-1`, `surface-2`, `surface-sidebar`, `surface-solid`, `on-surface-solid`,
`on-surface-solid-muted`, `control-track-off`, `control-off`, `control-on-on-solid`, `control-off-on-solid`,
`control-track-on-solid`, `value-pill-on-solid`, `scrim`, `success`, `warning`, `error`. It also accepts fonts (`font`, `font-title`, `font-text`, `font-heading`, `font-desk`, `font-wordmark`) and background image (`custom-background`, `background-image`). Inside Tako (Settings › Appearance), users can customize the current theme, storing an override without altering original theme files. When loading, Tako ignores (with a log entry) an invalid key or value. `tako theme check` reports both.

### The `@tako` library

`@tako/...` points to the default theme files of the installed Tako version:

- `@tako/main.slint`: the complete `App` and the types.
- `@tako/tokens.slint`: the `Ui` global.
- `@tako/widgets.slint`: `RowView`, `HintBar`, `Pill`, `Toggle`, `Icon`, `Glyph`.
- `@tako/icons.slint`: the `Icons` global.

Reusing pieces of the default theme (`import { RowView } from "@tako/widgets.slint";`) is the recommended way to
start. Keep in mind, though, that the default theme's internal components may change between minor Tako versions.
**The stable contract is section 4 (the `App`) and section 5 (the types).**

## 7. Package and distribution

A theme is a **folder** with `theme.toml` or a **`.takotheme`**: the same folder zipped, with `theme.toml` at the
zip root or inside a single top folder.

Limits (third-party data is checked before any file is written):

| Limit | Value |
|---|---|
| Package on disk | 200 MB |
| Total uncompressed | 200 MB |
| One file | 64 MB |
| Files and folders | 5,000 |
| Path | 240 characters, no `..`, no absolute path, no reserved Windows names |
| File types | `.slint`, `.toml`, `.png`, `.jpg`, `.jpeg`, `.svg`, `.webp`, `.ttf`, `.otf`, `.txt`, `.md` |

Images and fonts come in through Slint itself (`@image-url("background.png")`, `import "fonts/Mine.ttf";`), with
paths relative to the `.slint` file.

To install:

- **File:** Settings › Appearance › Theme › **Install theme…**, then pick the `.takotheme`.
- **Folder (while developing):** copy it to `<Tako data>/themes/<id>/`; the theme shows up in the list.
- **Address:** publish a GitHub *release* with the `.takotheme` attached. People using the theme type the
  repository address in **Install from address…**. With **Update themes**, Tako downloads the new release when
  there is one.

Themes live in `<Tako data>/themes/<id>/`. Installing a theme with the same `id` replaces the previous one.

| System | `<Tako data>` |
|---|---|
| Windows (installed) | `%LOCALAPPDATA%\Tako` |
| Windows (portable) | `Tako_Data` next to `tako.exe` |
| Linux | `$XDG_DATA_HOME/tako` (or `~/.local/share/tako`) |
| macOS | `~/Library/Application Support/Tako` |

## 8. `tako theme check`

```
tako theme check <theme-folder>
tako theme check my-theme.takotheme
```

It applies the same rules as installing (manifest, API version, limits, paths, file types), compiles the `.slint`
with this Tako version's `@tako` library and checks `tokens.toml`. Everything happens in a temporary folder and
**nothing is installed**.

- Exit `0`: valid theme (`My theme 1.0.0 (author.my-theme): valid theme, API 1.`).
- Exit `1`: problems, one per line, with the file and line of the Slint error.
- Exit `2`: wrong usage.

## 9. API versions

The major version (`api = 1`) only goes up when the contract breaks: a property or callback from sections 4 and 5
is removed or changes type or meaning. In that case, themes for the previous version are refused with a clear
message and Tako falls back to the default theme. **New** properties come in without changing the version. That's
why Tako ignores what the theme doesn't declare, and an older theme keeps working, just without the new feature.

## 10. Checklist before publishing

- [ ] `tako theme check` reports no problems.
- [ ] Controller: every page, the Control Center and dialogs have visible focus.
- [ ] Mouse: `clicked` and `hovered` in the zones of [section 4.9](#49-callbacks).
- [ ] `Ui.reduce-motion` turns animations off; `Ui.text-scale` at 1.5 doesn't clip texts.
- [ ] Small window (640×360) and large (4K).
- [ ] Over the game (`transparent-background`): only the Control Center bar shows.
- [ ] Both languages (pt-BR and en-US): longer texts fit.
- [ ] License for the bundled fonts and images (a `LICENSE.txt` or `.md` in the package).
