# Changelog

English (US) release notes, mirroring CHANGELOG.md (Brazilian Portuguese). Before publishing a version, add a `## [VERSION]` section to **both** files: the workflow publishes the section matching the tag (this file's, with a link to the Portuguese one) and fails if either is missing.

## [Unreleased]

### What's new
- **Desktop motion (TAKO):** animations follow the design system timings and easing curve; page changes fade with a subtle scale, respect Reduce motion, and skip transitions while the window is inactive.
- **Floating global search with Ctrl+K (TAKO):** 560-wide floating search panel positioned 110 px from the top over a dimmed backdrop, reachable via keyboard shortcut or on pages without an embedded search; shows library results with cover, title, publisher, and status with a Library badge; supports arrow key navigation (↑↓), Enter to open the selected game directly, and Esc to clear or dismiss.
- **Desktop layer screens (TAKO):** image dialogs adjusted to 540px, the Image picker to 1180px, and updates to Text field and App update dialogs.
- **Layers in the new desktop style (TAKO):** 220px menus with high-contrast borders and elevated shadows opening from the anchor in 160/200 ms; bottom-centered notification toasts with light background, dark text, 2.3s duration, and a 20px rise entrance; tooltips with a 500ms delay and 120ms fade; 400px side panels (480px in game settings) sliding from the right without dimming the background; and 480px modal dialogs (compact 440px, wide 680px) with 18px radius and a 60% dimming backdrop. Esc closes the topmost layer and focus stays locked to the active layer.
- **Redesigned game page (TAKO):** the 560 to 700 highlight carries the art edge to edge, fading into the list and the bottom (the new art comes in over the old one and settles; with no art it becomes a gradient of the game's color). Bottom left are the data (last session, play time and Metacritic, only with a score); bottom right, the status, the logo or name, the description and the actions (Play, Install or Back to game, favorite, ⋯, gear and Quit), with the game status and the store as chips in the corner. The page now scrolls, with the body below (a still trailer that only plays on click, a screenshot, About, Features and Details, and the sessions with their numbers on the right, which follow the scroll; at the end, Similar games from your library with the same genre or developer; each block only shows when there is data; the keyboard and the controller reach the trailer, See all and the Similar covers, scrolling to the item), and a thin bar at the top (← Library › game) that, when scrolled, gets a background, the cover and the main button. Page Up and Page Down scroll one screen, and ↓ on the actions enters the body and scrolls with the keyboard or the controller (↑ goes back); the two-phase highlight ("More about the game") is gone. Switching games does not reload the page: the parts leave and come back in a 55 ms staircase and the new art comes in over the old one (with Reduce motion, the switch is instant). Playing shows "PLAYING · 42 MIN" and ✕ to quit; downloading turns the progress into a bar at the foot of the button; while the library loads, the page shows placeholder blocks.
- **Your week, Needs attention and Installed in the Overview:** below the highlight, four groups of blocks with the week's play time (and the day bars), games played and sessions, the game of the week and the last and longest session, built only from the recorded sessions (with none, the section disappears); the list of what needs attention (background error with Try again, game with missing files, install in progress and game without a cover with Choose art); and the row of installed game covers, with ‹ › arrows that only scroll the row. The keyboard and the controller walk through everything in screen order.
- **Library Overview (TAKO):** the desktop now opens on the Overview, the row at the top of the list: the title with the game and hour count, the highlight with the last game you played (data bottom left; status, logo and description bottom right, with the white Play button) and, beside it, Jump back in with the next four games. Without art the highlight becomes a gradient of the game's color; with an empty or loading library it shows the octopus or placeholder blocks. Picking a game in the list opens its page; with the keyboard and the controller, the highlight and Jump back in join the zone order.
- **Redesigned desktop profile (TAKO):** the profile page (`desk-profile`) now follows the new design system visual language with an 88 px avatar header and accent ring, status indicator, stats in 16 px rounded block groups, recent sessions list with 40×52 px game covers, and Overview, Sessions (with weekly/monthly/all filters and daily grouping), and Collections (collection grid with cover mosaics) tabs.
- **Redesigned onboarding and splash screen (TAKO):** the desktop first-run wizard (`TelaOnboarding`) and loading splash (`splash.slint`) follow the new design system. At the top, the new brand with the octopus, purple glow, TAKO wordmark and タコ kana, plus "Skip setup". Background with tilted cover mural and gradient scrim; step progress indicator `PassosDoOnboarding` with dashes and back action on hover, 46 px white primary desktop buttons with 36 px dark chip and chevron, 36 px outlined buttons, mode selection with 290 px preview cards, and 280 ms motion transitions disabled when Reduce motion is active. Splash screen with 16 px radius and stone `#d8c8bc` progress bar over `#1a1a20`.
- **First run and loading:** Tako presents setup steps with clearer guidance, accessible actions and better support for smaller windows. The loading screen follows the desktop visual identity.
- **Fixed game list (TAKO):** the library list is now a 292-wide column against the left edge, from under the top bar to the bottom of the window, with no divider in the TAKO theme. At the top, the All · Favorites · Installed tabs and the search with the Sort, view and filter button, which gathers order, view mode, filters, collections, Add game and Update library. Collections leave the tabs and move into that menu (the active collection becomes a chip in place of Overview); from the keyboard or a controller, Tab reaches the button and the chip (Enter opens, Esc on the chip goes back to All). The tabs get the room their text needs, even at 125%. The joined sidebar and the sidebar width leave Settings.
- **Collapsed list and Grid inside the list (TAKO):** Ctrl + B (or the button next to the search) collapses the list to 76 wide, with only the 46 icons; the Grid now lives in the list itself, 3 covers per row, with the game page beside it (collapsed, it becomes the icon list). Games that are not installed get a dimmed name and icon, and the empty list shows the design system octopus with one sentence.
- **New brand on the desktop:** the octopus, the TAKO wordmark and タコ in the top bar, the octopus as the app icon (desktop window, installer and packages, on a dark plate) and new thin-stroke icons across the desktop interface. Full screen is unchanged.
- **Game art colors (TAKO):** in the TAKO theme, the accent and the window corner glows take the strongest color of the game art on screen and slide to the new one in 1 s. You can turn it off in Settings › Appearance; off (or without art), it keeps the logo stone. The chosen accent applies to the other themes.
- **Desktop themes and accent color:** in Settings › Appearance, pick TAKO (the new default), Dark, Midnight, OLED black or Liquid Glass (with the game art as ambient light, opacity and pill controls) and the accent color, including a custom one. Colors, radii and animation timings follow the new design system; full screen is unchanged.
- **Redesigned top bar on the desktop:** 52 high and edge to edge, with the Library tab in thin letters and a glowing underline, the profile as an avatar with a ring in the accent color (its menu opens the Profile page), the full screen button and smaller window buttons. All of them can be reached from the keyboard.
- **Downloads in the top bar:** the ↓ button before the profile shows with a number how many things Tako is doing in the background (library refresh, update check, downloads, covers) and gets a badge when something fails; when one finishes or fails, a notice tells the result. Click it to open the list, with progress, "Try again" on errors and "Clear finished".
- **Settings redone on the desktop:** categories sit in a fixed 292 column against the edge, each with its icon in the header; rows form 64-tall outlined groups and, when a link opens a specific setting (e.g. "Manage sources" in Downloads), that row flashes in the accent color, except with Reduce motion. Big Picture is unchanged.
- **Select, slider and switch in the new style (desktop):** a choice among several options is an outlined select that opens the list with the row name and a ✓ on the current one; volume and dead zone are a thin-track slider with the value written out (drag, click the track, wheel, ← → 5% and Shift 10%) and the switch turns on and off in the accent color. The slider's mute and max-volume buttons are gone.
- **Settings › Theme (desktop):** a new category with the five themes as preview cards, the accent color as dots (the nine, a custom color and its code; outside TAKO), the options of the chosen theme (TAKO: game art colors; Liquid Glass: art as light, opacity and pills) and a preview with status, progress, switch and Play. The theme and color selects leave Appearance; ← → move between cards.
- **Cover queue and controllers in the new style (desktop):** the "Accept" (with ✓) and "Choose another" buttons in the cover review and "Accept all N" are outlined like the other rows. Fixed: the controller battery showed an empty bar next to the right percentage and, with no controller connected, an empty battery icon.
- **ScreenScraper with your developer account:** anyone with ScreenScraper developer credentials can enter them in Settings and use ScreenScraper even on builds that don't include them.
- **Windows full screen mode:** use Tako as the home app from Settings.
- **Torrent downloads:** download games from sources you add yourself, in a panel with pause, resume and a queue. Off by default.
- **Games not installed:** games from your store accounts that aren't on this PC show up after the installed ones, grouped by store, in the list and the grid. Each group opens and closes, and Tako remembers how you left it.
- **Install from the store:** the ⬇ button on the row, the cover and the highlight opens the install in the store client, showing the store and the size. Games bought in more than one store let you pick which to install from.
- **Install progress:** on Steam, Tako shows the percentage as a ring in the list and grid and "Downloading… 42%" in the highlight. When it finishes, the game becomes installed without restarting. Epic shows "Installing" without a number.
- **Free space:** the highlight shows how much space is left on the disk and warns when there isn't enough for the game.
- "Install" is also in the game menu and in multi-select, which open one at a time in the store.
- **Game icon in the list:** the list shows the game icon, like Steam (from the store or the Windows executable, or the title's initial); the cover stays in the Grid.
- **Amazon games:** on Windows, games from your Amazon account show up as not installed and Install opens Amazon Games.
- **Per-game VRR:** on Linux (KDE and wlroots), the game screen can turn VRR on just for that game and off at the end, even after a crash.
- **Theme preview:** in full screen, picking a theme shows a 15 s preview; A applies, B goes back.
- **App intro:** full screen opens with a short octopus animation that any key skips and that "Reduce motion" turns off.
- **Tako's octopus:** the animated octopus shows up in empty states, while the library loads and in the desktop "Welcome back".
- **Tools with the game:** in the game settings, extra programs (FPS overlay, mods) open with the game and can close at the end, with your permission.
- **`tako://` links on macOS:** shortcuts and Stream Deck open games and Tako screens on the Mac too.
- **Linux packages:** AUR PKGBUILDs and a Flatpak in the repository (not published yet).
- **Bottles and itch.io on Linux:** games installed through Bottles and the itch.io app show up in the library.
- **Local API:** other programs on your computer (Stream Deck, scripts) can read the library and open games through an API on 127.0.0.1 with a token. Off by default.
- **Edit the theme:** accent color, font and background (color or image) of the current theme, with a live preview and "Restore default".
- **Create a theme:** start from the default theme or an installed one, adjust colors, font and background, name it and save it as a new theme; export it as `.takotheme` to share.
- **Theme music and sounds:** in full screen, a theme can bring background music and navigation sounds, with their own volume in Settings › Sound. They never play during a game.
- **Fita theme:** a new official full-screen theme, with games in a horizontal carousel and the art filling the screen.
- **Proton and Wine on Linux:** Windows games added by hand can open with the system Wine, Steam's Proton (including GE-Proton) or a Lutris runner, with their own prefix.
- **Backup and restore:** Settings › System creates a `.takobackup` with the library, settings, themes and chosen covers. Restoring saves a copy of the current state first; permissions and the local API token never go into the backup, and commands, arguments and environment variables that come from it ask for confirmation before they apply.
- **Sunshine and Moonlight:** in Settings › Integrations, export your games to Sunshine/Apollo and open them from Moonlight. Tako shows a preview, backs up `apps.json` and never touches the apps you already had.
- **Console Mode:** Settings › Integrations shows whether Console Mode is installed and which API version it offers.

- **Refresh library:** a new button on the desktop list bar rescans the stores right away.
### Fixes
- **Windows full screen mode:** setting Tako as the home app works again; Windows used to reject the package (0x80073CF6).
- **Steam icon fallback:** when a Steam game's primary icon address fails on the CDN, Tako falls back to the community icon from Steam's API and avoids caching the failure indefinitely.
- **Update checks:** slow connections keep going while receiving data, with gzip compression and a clear failure when the network stalls.
- **Library refresh feedback:** while the library refreshes, the button spins and the bar shows "Updating…"; clicking again says it is already running, and at the end a notice shows the game count, new games and any store that failed.
- **Desktop background and glass:** the dark vertical band in the middle of light artwork is gone: the side darkening only decreases and the ellipse behind the text reaches zero before its edge. The bars and glass cards blur the already-darkened artwork and use the solid tint until the blurred copy arrives (UI-GLASS-001).
- **Step instructions:** appear one per line.
- **Faster startup:** the whole library shows up as soon as Tako opens, from the last store scan, while a fresh scan runs in the background.
- **Selected game art:** the background and logo now change with the game, installed or not. Steam art opens straight from Steam's own cache, the last game you picked loads first, and Tako no longer holds the previous game's art while the new one arrives. Covers and backgrounds that fail on a slow network are retried, on desktop and in full screen, without restarting.
- **Steam icons:** Steam game icons show up again in the list and the image picker. Steam moved them and the old address stopped answering.
- **Arguments and variables need confirmation:** extra arguments, environment variables and Proton/Wine set per game now go into the confirmation, together with the before and after commands; if you already used them, you confirm once.
- **Slow list and images not loading (alpha.15):** list icons have their own queue and no longer hold up covers and backgrounds; the UI only redraws when an image arrives, and art the store doesn't have isn't requested again on every launch.
- **Play time:** a game with a session but no recorded time shows "Not tracked" instead of "Not played yet".
- **Window on Windows:** Minimize, Maximize and Close on the desktop bar respond again; Maximize snap layouts still work.
- **UI closing and reopening when playing:** new logs to diagnose the issue (BUG-001).

## [0.2.0-alpha.14]
### What's new
- **One top bar:** Tako's bar is now the window's own bar, edge to edge, with Minimize, Maximize and Close on the right. On Windows, the empty part of the bar drags the window, double-click maximizes and Maximize shows the snap layouts.
- **Profile at the top:** avatar and name in a pill, like Steam, with View profile, Switch profile… and Edit profile. Full screen is now a button with just the controller icon.
- **Search where the content is:** in the list panel, under the tabs; centered in the Grid and Categories bar; at the top of the Settings panel. In Store and Profile, Ctrl + K opens a floating search.
- **Joined sidebar:** an option in Settings › Appearance that attaches the list to the top bar as a single piece of glass, in Normal or Narrow width.
- **Game status:** Playing, Want to play, Paused, Beaten or Abandoned, from a picker in the corner of the highlight, the game menu and multi-select. Playing for the first time sets Playing, and you can filter by status.
- **Active filters:** shown as fixed chips under the count, each with ✕, and "Clear" when there is more than one.
- **Highlight:** the card row is now About, Details and Sessions, and the Metacritic score joins the numbers strip when the game has one. The trailer moved to the second phase, which no longer repeats the first one and scrolls more smoothly.
- **Updates:** "Check for updates…" answers right in the menu (checking, up to date, or an error with Try again). The new dialog shows the download, the release's highlights and "Restart now". If installing fails, Tako goes back to the previous version and tells you.
- **Emulators:** every one has its official logo, also in the Settings emulators screen.
- Stacked buttons: the main action sits on top and the two below split the width evenly.

## [0.2.0-alpha.13]
### What's new
- **New first run:** a loading screen with the octopus assembling while Tako looks for your games; stores found, emulators with each one's systems and games and the ROM folder, covers with a before and after and the choice between fetching from IGDB or using only what the stores have, how to open Tako and a final summary with each store and emulator.
- **Profile:** a new tab in the top bar, with this week's play time in bars, most played games, sessions by day and collections as cards. The background is the art of the month's most played game.
- **Tags:** the editor shows the game's tags as chips with ×, suggests existing ones (with how many games each has) and creates a new one right from the field.
- **Review covers:** the queue shows the current and proposed covers side by side, the confidence as a percentage, "Accept" and "Choose another" on each row, "Accept all N" for high confidence and "Undo" after accepting.
- **Controls:** every connected controller is listed, and the tests are grouped together.
- **Dialogs:** actions sit in a footer, with the main one on the right.
- **Opening a game:** the button shows a spinning ring; if it takes long, "Still opening…" and "Cancel" appear.
- **Toasts:** they stay put while the mouse is over them.
- **Loading:** the library shows placeholder blocks in the highlight and the cards too.
- Cut text shows in full on hover, and the game count fits the small window.

## [0.2.0-alpha.12]
### Fixes
- **Game logo:** games with a logo no longer show the name as text before switching to the logo. While the logo loads, the title space stays reserved, and the logo fades in when it arrives.

## [0.2.0-alpha.11]
### What's new
- **Lighter and smoother:** Tako uses much less memory (each image lives once on the graphics card and the background loads at the window's size), the game list scrolls without stutter and the app keeps up with 120 Hz or faster monitors.
- **Compact game list:** shorter rows without the store icon; the selected game's store shows in the top-right corner. Selecting a game no longer shifts anything in the row.
- **Add a game:** a "+" button next to the game count and "Add game…" in the Tako menu.
- **More about the game:** scroll over the highlight (or press Page Down) to see a second screen with the full description, every detail, sessions and images. Scrolling up, Page Up or Esc go back.
- **Staged opening:** when Tako opens, the screen appears in parts, smoothly. "Reduce motion" turns it off.
- **Game switching without flashing:** the glass and colors change together with the art.
- The window bar no longer repeats the logo, and the top bar logo is sharper and reacts to the mouse.

## [0.2.0-alpha.10]
### What's new
- **First run:** the subtitle shows in full, the "How do you want to open Tako?" step fits small windows and the last step has "Review covers". With no games found, it offers "Add a game by hand" and "Search again".
- **Categories:** when no game has a status, the screen explains how to mark one and offers "Group by Store".
- **Playing:** the highlight shows how long the current session has been open.
- **This game's settings:** what you changed shows Tako's default under it, and "Use all of Tako's settings" asks before discarding the changes.
- **Controls:** low battery shows in yellow, and "Restore shortcuts" asks first.
- **Toasts:** they stay 4 seconds, or 8 when they offer "Undo", and stay readable over any art.
- **Game that closed on its own:** the dialog shows the cover, how long after opening it closed and "Repair files" when the store allows it.

## [0.2.0-alpha.9]
### What's new
- **Game logo in the highlight:** the game title becomes its official logo (from Steam, or SteamGridDB when missing). Without a logo, with low contrast over the art or if you prefer, the name comes back as text. Choose in Settings › Appearance › "Game titles", or per game in the Logo tab of Change images.
- **Draggable volume:** the navigation sound volume is now a slider, with click on the track, mouse wheel, arrows (Shift moves more), Home/End, mute and 100% buttons and a sample tone on release.
- **Display confirmation:** changing the game screen or resolution asks "Keep this setting?" and reverts on its own after 15 s unless you confirm.
- **Window bar:** right click opens the window menu, the maximize button follows Win+Up/Down, the bar dims when the window loses focus and "Update ready" shows up when a new version is downloaded.
- **Game switching:** the new art slides in gently and the text rises as it appears; with the keyboard, the art only changes once you stop on a game.
- **Change images:** Style is now a list and the arrows move through the grid as it is shown.
- **Collections:** the empty tab offers "New collection…".
- Tako's icon shows on the Windows taskbar and shortcuts, store logos have no outline in any menu and genres read "Action - Platformer".

## [0.2.0-alpha.8]
### What's new
- **Own window bar:** Tako draws the window bar over the game art, with the logo and Minimize, Maximize and Close (drag by the bar; double click maximizes).
- **Choices become lists:** in desktop mode, every choice between several options (language, display, resolution, audio, profiles, filters, sorting) opens a list next to the button, with ✓ on the current one. Switches only turn things on and off.
- **Collections:** the Collections tab picks the collection from a list with each one's game count, "New collection…" and "Manage collections", and remembers the last one.
- **Change images:** picking cover, background, logo and icon is now a large popup with a full-width grid, a preview and Use, Back to original and Cancel; clicking only picks.
- **Animations:** page, tab, mode (list, grid, categories) and game changes, popups, panels, menus and toasts now animate as designed, and "Reduce motion" still turns everything off.
- **Game switching without flashing:** the new art fades in over the old one and settles slowly; the old one stays until the end.
- **Background art:** covers the whole window at any size, without stretching or bars, following Steam's background format (3840 × 1240).
- **First run:** the stores screen shows what Tako found and offers "Find games" and "Choose folders…"; Covers and Mode got a Back button.
- About and Details cards fit any window, long titles wrap to two lines and shrink, the small window uses compact rows and Settings use the right width on large screens.
- Side panels open below the bar, the list scrollbar sits on the edge and icons (stores, list, grid) no longer look outlined.

## [0.2.0-alpha.7]
### What's new
- **New desktop mode look:** the focused game's art fills the whole window and the bar, list and cards float over it in glass. Without art, the background takes the game's color. Titles in Poppins and text in Plus Jakarta Sans.
- **Top bar:** the Tako logo opens a menu with Settings (Ctrl + ,), Switch profile, Go to full screen, Check for updates and Quit Tako. Search lives in the bar and opens with Ctrl + K (Ctrl + F still works).
- **Game list:** tabs as a segmented control, each game's store icon, ▶ on the chosen row, reverse order and filter on the count line, and the octopus when there's nothing to show.
- **Grid and Categories:** besides the list, the library can show as a cover grid or as rows grouped by status, collection, tag, genre, store or installation, with "See all". The choice is saved per profile.
- **Select several games:** Ctrl or Shift + click and Ctrl + A mark games to add to a collection, tag, favorite, mark as beaten or remove hand-added ones at once.
- **Game highlight:** status label, a strip with the last session and play time, a prominent Play (Back to game while it runs, Install via the store when it isn't installed) and buttons to favorite, more actions and this game's settings. About (with "More"), Details, Sessions and Trailer cards, when the game has one.
- **Dialogs, panels, menus and toasts** in the same glass, with short transitions that go away with "Reduce motion".
- **Full-screen first run:** stores, emulators (when there are any), covers, how to open Tako (in a window or full screen) and a summary of the games found. The choice becomes "Open in", in Settings › General.
- After favoriting or changing collections for several games, the toast offers **Undo**. Icon-only buttons and long names show a tip when you rest the mouse on them.
- **Settings** with the same structure as the library, over gray, and search in the bar (Ctrl + K); the controller battery shows in Controls.
- **Reduce transparency** in Settings › Appearance makes panels and buttons opaque in desktop mode.
- Full screen is unchanged.

## [0.2.0-alpha.6]
### What's new
- **Desktop mode:** Tako now opens in a mouse and keyboard window, with the library as a list, the game highlight with Play, favorite and more actions, search, filter and sort, All, Favorites and Collections tabs, and Settings inside the window.
- **Full screen as the "Big Picture":** the controller and TV interface is still there. Switch with the Full screen button or F11, and come back from the Control Center or Settings › General. Themes apply in full screen.
- With a controller connected, Tako asks whether you want full screen when it opens (you can stop it from asking). Opened with `--big-picture`, from Xbox mode (AnyFSE) or from Console Mode, it goes straight to full screen.
- On the desktop, the window stays minimized during a game and comes back when it closes.

## [0.2.0-alpha.5]
### What's new
- **First-run wizard:** language, detected stores and cover providers, with a controller or the keyboard.
- **Organizing:** your own tags (applied in bulk and usable as a filter), smart collections that update themselves from the filters, and how much space each game and each drive takes, with sort by size.
- **Local profiles:** several people on the same PC, each with their own library, favorites, play time and settings, and a "Who's playing?" question when Tako opens.
- **Stores:** games you own but haven't installed (Steam, Epic and GOG; Amazon on Linux and macOS), with Install through the store and a notice when it finishes. Non-Steam games added to Steam. Emulators and ROMs (RetroArch, ES-DE, Dolphin, PCSX2, DuckStation, PPSSPP, Ryujinx, Cemu, RPCS3 and ROM folders), each opening in the right emulator.
- **Covers for the whole library:** cover, background, logo and icon on their own, and "Review covers" lists only the doubtful ones.
- **When playing:** a notice when the store app is missing, with a link to its official page; "Repair files" for Steam games; a warning when a game closes right after opening, never opens or exits with an error, with the code and what it usually means; environment variables and commands before and after playing, with your permission.
- **Screens:** each game can open on another screen with its own resolution and HDR, and Tako remembers the resolution and HDR of each monitor.
- **Controllers:** choose the button for each Tako action, with one profile per controller and the Control Center shortcut among them. With HidHide (optional), the game stops seeing the controller while the Control Center is open (Windows).
- **Accessibility:** text size, navigation sounds with their own volume, and "Reduce motion" now also covers opening a game.
- **`tako://` links** open a screen or a game from outside Tako (shortcuts, Stream Deck), on Windows and Linux.
- **Themes:** install a theme from its GitHub repository address and update it later; theme API documentation and `tako theme check` for authors.
- **Windows:** Windows 11 Xbox mode through AnyFSE, with the steps in Settings › General.
- **Tako performance** in Settings › System: memory, CPU and startup time.
- **Nightly builds:** one build a day from develop, for those who opt in under Settings › General.

## [0.2.0-alpha.4]
### Fixes
- The portable build (and the installed one) no longer offers an update to the version that is already open.
- Moving the mouse over cards no longer scrolls the row or the page; the mouse only highlights, scrolling stays with the controller and keyboard.
- Less memory: the image cache is much smaller and large libraries no longer keep re-decoding covers.
- Inside the Control Center, the keyboard shortcut (Ctrl+Shift+Tab by default) goes back to the game, as the on-screen hint said.

## [0.2.0-alpha.3]
### What's new
- Play status for each game: Playing, Beaten, Paused, Abandoned or Wishlist. Set it from the card menu or from Options in the details; it shows in the details header and has its own filter.
- Broken games: Tako checks added games whose file or folder is gone. The "Broken only" filter lists them all, and playing one offers Edit or Remove.
- Session history: every time a game runs, the date, time and length are kept (the last 100 per game), under Options › Sessions in the details.
- Battery of wireless controllers in the top corner and the Control Center, with a warning when it runs low.
- When the controller disconnects, Tako tells you and keeps your place; once it reconnects, navigation picks up where you left off.

## [0.2.0-alpha.2]
### What's new
- Opening Tako again brings the window that is already open to the front. During a game, it opens the Control Center over the game.
- The Control Center keyboard shortcut is picked by pressing the keys again, as before.
- When adding an `.exe` on Windows, the game's name comes from the product name stored in the file instead of the file name.
- Image picker: SteamGridDB gets filters (style, shape, animated) on the X button and loads more images when you reach the end of the list.
- Sources and IDs: a typed ID is only used after you confirm it ("Use {game}?").

### Fixes
- Dialogs that open on their own (the "Can you see this screen?" question, the Playnite preview, the update-ready notice and the system report) only showed after a key press. They now show right away.
- Windows: the release ships x64 (Intel/AMD) only.

## [0.2.0-alpha.1]
### What's new
- **Tako is now written in Rust, with a Slint interface, and runs on Windows, Linux and macOS.** Installer and portable on Windows; deb, AppImage and tar.gz on Linux; dmg and zip on macOS. (PRD v0.2)
- While you play, Tako's window closes and only a small process stays, with no window and nothing drawn. It follows the game, hides the cursor, moves the pointer with the controller and puts the display and audio back at the end. (ADR-0026)
- **Smaller quick menu:** Home without closing the game, back to game, audio, screen, pointer and close game. Opens with Select + Y on the controller and, on Windows, with Ctrl+Shift+Tab.
- **Themes switch on the fly**, including the new "Tako Alto contraste" (high contrast). A theme can change only the default theme's colors (`tokens.toml`). If a theme breaks, Tako falls back to the default one.
- Linux: Steam (native, Flatpak and Snap), Heroic and Lutris. macOS: Steam and Heroic (Epic and GOG). "Add game" takes `.desktop` files, AppImages, binaries and `.app` bundles.
- Coming from the alphas, you keep your library, collections, play time, chosen covers and settings. Provider keys move to the system's password vault.

### Changes
- Moved out of Tako and into Console Mode: TV control, FPS counter and limit, Steam in the tray and "console mode on launch". (ADR-0025)

### For developers
- Rust workspace in `crates/`; ADR-0023 to ADR-0026. ADRs 0011 to 0022 are now history.
- `tako --bench` and the CI `bench` job measure PRD §8.1. Parity status in `docs/marco-2/paridade.md`; test scripts in `docs/testing/`.

## [0.1.0-alpha.10]
### What's new
- More stores in the library: **Epic Games**, **GOG** and **Xbox / PC Game Pass** games show up on their own and launch through the store's own client. Each store can be turned on or off in Settings › Library and stores; a game in two stores shows once per store, marked "Also in". (#39, #40)
- **Console Mode menu** in the sidebar, with cards for Game screen, TV, Game audio, Steam in the tray, FPS counter and FPS limit, each showing whether it is on, off or needs attention. The "Set up to play on the TV" wizard walks you through it. (#45)
- **Console mode on launch:** with the option on, Tako opens straight on the TV and only gives the displays back to the desk when you quit or choose "Back to PC". Emergency shortcut: Ctrl+Alt+Shift+D. Returning to the desk no longer freezes the screen. (#42)
- **LG (webOS) and Samsung (Tizen) TVs**, besides Google TV and Android TV: turn on, switch to the PC's input and standby. (#38)
- **Collections and per-game settings:** create collections, which show on the home screen and in filters. Each game can have its own game screen, resolution, HDR, audio output, FPS limit and counter. The ☰ button on a cover opens quick actions. (#41)
- **Game data from IGDB and ScreenScraper**, with your own key (Twitch) or account, stored encrypted on this PC. Steam is still queried for every game. ScreenScraper identifies emulator ROMs by file, and "Add game" already accepts ROMs with a system picker. Launching ROMs in an emulator comes later. (#46, #47)
- **Control Center with the game live behind it**, like Steam's, when the game runs windowed or borderless. The shortcut is now holding **Select + △** (Create + △ on PlayStation, View + Y on Xbox) or **Ctrl+Shift+T**; both can be changed in Settings › Controls. (#44)
- Settings › System › **Controller test** shows live what the controller sends and copies it so you can send it when something goes wrong. (#43)

### Fixes
- **DualSense and DualSense Edge over Bluetooth with Steam closed:** D-pad up got stuck and the buttons did not respond. Tako also merges Steam's virtual controller and the PlayStation controller without one cancelling the other, and ignores the arrow-key echo Steam sends. (#43)
- **Settings:** organized in categories, nothing hidden behind the hint bar, no technical text and developer options tucked away. The hint bar shortens instead of being cut off, and passwords and keys are masked. (#45)
- **Open windows in the Control Center:** titles are no longer cut and the focused card is always fully visible. (#44)

### For developers
- ADR-0019 (library sources), ADR-0020 (settings layout), ADR-0021 (IGDB) and ADR-0022 (ScreenScraper). The hardware checklist now goes to section 24.
- ScreenScraper's software credential comes from the `SCREENSCRAPER_DEVID` and `SCREENSCRAPER_DEVPASSWORD` CI secrets; without them ScreenScraper shows as unavailable.

## [0.1.0-alpha.9]
### What's new
- Tako's own FPS counter, no RTSS: Tako measures FPS, frame time and 1% low through Intel's PresentMon, like CapFrameX, without touching the game. No more conflicts with MSI Afterburner or anti-cheat. The counter shows in Tako's own window over the game, in the FPS only, Compact and Detailed styles. (#36)
- To measure, Windows has to allow Tako once. "Show FPS" is turned on in Settings with a keyboard and mouse, because the Windows prompt doesn't take a controller. After that, sign out and back in to Windows once. The installer has an "Enable FPS counter" option, unchecked by default, to handle it during setup. In the Control Center, the FPS options are dimmed with a notice until the counter is active. (#36)
- FPS limit through the graphics driver (NVIDIA, AMD and Intel), also without RTSS. Your previous value comes back when the game ends and after a Tako crash. Options: no limit, 30, 40, 45, 60, 90, 120, 144 and "Display − 3" for VRR. (#35)

### Changes
- Tako no longer uses or changes RTSS. If you use RTSS or Afterburner, they stay as they are. A limit set through RTSS by earlier Tako versions is not undone automatically. (#36)
- In exclusive fullscreen the counter doesn't show over the game, but the Control Center shows the FPS. (#36)

### For developers
- [ADR-0017](docs/adr/0017-contador-de-fps-com-presentmon.md) (FPS counter and permission) and [ADR-0018](docs/adr/0018-limite-de-fps-pelo-driver.md) (driver limits). The hardware checklist gained sections 13 and 14.

## [0.1.0-alpha.8]
### What's new
- Game screen, like Console Mode: in Settings pick the screen you play on (the TV, for example). When a game starts, Tako turns that screen on, turns off or blacks out the others, applies resolution, refresh rate and HDR if you want, and puts everything back when it closes. The first time on a new screen it asks "Can you see this screen?" and reverts on its own if nobody answers. Off by default. (#30)
- Audio and TV: "Audio output while playing" switches sound to the TV during a game and back afterwards. Google TV and Android TV can be turned on over the network and switched to the PC's input when a game starts, with optional standby on exit. (#31)
- Steam games tracked more precisely (by the game Steam itself reports as running), plus an option to send Steam to the tray when you leave the game. (#31)
- A much richer in-game Control Center: open windows to switch to or close, volume and audio output, real CPU, GPU and RAM usage, FPS counter and FPS limit through RTSS, hide the mouse pointer or drive it with the stick, and open ControlFS. (#33, #31)
- Library search, filters and sorting: on-screen keyboard for the controller, accent-insensitive search, filters (installed, favorites, recent, source, genre) and sorting by name, last played, playtime, score or date added. Also works in Playnite themes. (#32)
- Large libraries without stutter: with 1000 games the library opens in about 0.1 s (was 1.8 s) and Tako uses 170 MB idle (was 507 MB); with 5000 games, 177 MB (was over 2 GB). Covers load only when they come on screen. (#29)

### Changes
- Tako is now free software under the AGPL-3.0, the same license as Console Mode, and uses its display, audio, Steam, TV and session menu code. (#27)
- The portable build is less than half the size: 69 MB (was 141 MB). (#28)

### For developers
- [ADR-0014](docs/adr/0014-licenca-agpl.md) (license), [ADR-0015](docs/adr/0015-monitores-do-console-mode-e-tela-de-jogo.md) (game screen) and [ADR-0016](docs/adr/0016-audio-steam-e-tv-na-sessao.md) (audio, Steam and TV). The hardware checklist gained sections for the game screen, the Control Center and audio/Steam/TV.
- 5000-game scenario and "open library" time in `tools/Launcher.Bench`. (#29)

## [0.1.0-alpha.7]
### What's new
- Displays and audio go back to normal after a game: Tako records resolution, refresh rate, primary display, audio output, volume and mute before launching the game and, when it ends, restores only what the game changed. If Tako crashes during the game, it offers to restore on the next start. It can be turned off in Settings. (#22)
- Playnite themes across the whole interface: in Settings → Themes, "Use this theme" replaces Tako's home screen with the theme's, with your library, Play, details and back working with a controller and keyboard. The Menu button always leads to Tako's Settings; any failure falls back to the default interface. (#24)
- Tako protects itself: a lightweight supervisor reopens the interface if it hangs or closes unexpectedly, then tries safe mode and, as a last resort, leaves you on the desktop. Games are not closed. (#23)
- Settings → System diagnostics builds a report (no personal data) with version, Windows, displays, controllers, Steam and Xbox mode, to attach when something goes wrong. (#21)

### Fixes
- Security: themes can no longer load images or videos from outside their own folder, the internet or the network, even when the path is built at runtime. (#25)

### For developers
- Named-pipe IPC with a versioned protocol between supervisor and interface ([ADR-0012](docs/adr/0012-supervisor-e-ipc.md)); our own display and audio contracts ([ADR-0013](docs/adr/0013-estado-de-monitores-e-audio.md)). (#22, #23)
- `tools/Launcher.Bench` and a `benchmark` CI job with the baseline in `docs/marco-1/benchmark.md`. (#21)
- Marco 1 plan and status in `docs/marco-1/plano.md`.

## [0.1.0-alpha.6]
### What's new
- Games tracked for real: when you play, Tako notices when the game ends (through its process or, for Steam games, the game folder), records the real play time, comes back to the front and puts the focus back on the game, with "Welcome back". For added games, Options → Edit changes the name, arguments, working folder and how the end of the game is detected. (#18)
- Playnite themes: import `.pthm` packages (with a safety check before any file is written) and manage everything in Settings → Themes: preview, compatibility report, use, remove and restore the default. Partial themes inherit the default theme. Three real themes were validated (PS5ish, Toggle and Revisited). (#19)
- Safe mode: if Tako closes badly twice in a row, it opens with the built-in theme only and no updates, with an option to go back to normal. An unexpected error shows the "Something went wrong" screen with a one-step way back to the desktop. (#17)
- "Open full screen when Windows starts" now works on the installed and portable builds. Settings shows whether the PC has Xbox mode. (#17)
- Your settings and library live in a database (SQLite) and survive restarts; the old file is imported by itself. Diagnostic logs, with no personal data, in Settings → Open logs folder. (#16)
- Automatic updates: when it opens, Tako looks for a new version and downloads it in the background, checking the file against the SHA-256 published on GitHub. When it's done, it asks whether to restart now or install on exit; during a game, the notice waits for the game to end. Works on the installed build (runs the installer silently) and the portable one (swaps the .exe). In Settings: check now, turn automatic updates off and get test versions. (#15)

### Fixes
- B on the "Starting" screen now really cancels (before, the game opened anyway). (#17)
- A controller that disconnects, gets swapped or loses window focus no longer leaves the focus "moving by itself". (#17)

### Updating from 0.1.0-alpha.5
- This is the last manual update: download the installer or the portable build from the release. From here on, Tako updates itself.

## [0.1.0-alpha.5]
### What's new
- Your real library: games installed through Steam show up by themselves (every library folder, with play time and last played, no need to have Steam open), and any other game comes in through "Add game" (executable, shortcut or link). Play launches the game for real. Favorites and history are saved. (#13)
- Automatic details and art, like Plex: description, genres, release date, developer, Metacritic score and high-resolution covers and backgrounds, from the Steam store. Added games are matched by name; if the wrong game comes up, Details → Options → Fix match lists the results to pick from. (#13)
- Game options: open in Steam, replace the cover with your own picture, open the folder and remove from the library (the game's files are not deleted). (#13)
- PlayStation controllers (DualSense, DualSense Edge and DualShock 4), over USB or Bluetooth, without Steam or DS4Windows: hints show ✕ ○ □ △ and the PS button opens the Control Center. (#12)
- Sharp 2K and 4K screens, and the layout fills the screen in any shape (21:9 ultrawide, 16:10). (#12)
- Installer and portable build, in the same format as Console Mode. (#14)
- Preview of Playnite 10 Fullscreen themes in Settings: 31 of the default theme's 33 files load, through the safe theme loader. (#7, #8)

### Changes
- New name: **Tako**, with the octopus on the icon and in the sidebar. (#11)
- The Control Center opens with Shift + Tab on the keyboard (like Steam's overlay), instead of G. (#12)
- Titles use a light, PS5-style font across the full width; the open sidebar shows the Tako logo. (#12)
- The sample games are gone from the app. (#13)

### For developers
- Marco 0 WPF interface built from the approved prototype, with a smoke test that walks every screen and publishes screenshots in CI. (#6, #7)
- Restricted XAML loader for themes and Playnite 10 theme adapter. (#3, #7, #8)
- Xbox mode / FSE spike with the FseProbe tool; its kit ships with releases while the spike is open. (#4)
- Releases through `build/Publish-Tako.ps1` (single-file portable and Inno Setup installer); every PR builds both in CI. (#10, #14)

## Earlier versions
Pre-releases 0.1.0-alpha.1 to 0.1.0-alpha.4 were internal test builds without their own changelog; what they brought is summarized in 0.1.0-alpha.5.
