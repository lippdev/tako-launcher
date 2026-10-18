# Changelog

English (US) release notes, mirroring CHANGELOG.md (Brazilian Portuguese). Before publishing a version, add a `## [VERSION]` section to **both** files: the workflow publishes the section matching the tag (this file's, with a link to the Portuguese one) and fails if either is missing.

## [0.2.0-alpha.7]
### What's new
- **New desktop mode look:** the focused game's art fills the whole window and the bar, list and cards float over it in glass. Without art, the background takes the game's color. Titles in Poppins and text in Plus Jakarta Sans.
- **Top bar:** the Tako logo opens a menu with Settings (Ctrl + ,), Switch profile, Go to full screen, Check for updates and Quit Tako. Search lives in the bar and opens with Ctrl + K (Ctrl + F still works).
- **Game list:** tabs as a segmented control, each game's store icon, ▶ on the chosen row, reverse order and filter on the count line, and the octopus when there's nothing to show.
- **Grid and Categories:** besides the list, the library can show as a cover grid or as rows grouped by status, collection, tag, genre, store or installation, with "See all". The choice is saved per profile.
- **Select several games:** Ctrl or Shift + click and Ctrl + A mark games to add to a collection, tag, favorite, mark as beaten or remove hand-added ones at once.
- **Game highlight:** status label, a strip with the last session and play time, a prominent Play (Back to game while it runs, Install via the store when it isn't installed) and buttons to favorite, more actions and this game's settings. About (with "More"), Details, Sessions and Trailer cards, when the game has one.
- **Dialogs, panels, menus and toasts** in the same glass, with short transitions that go away with "Reduce motion".
- **Full-screen first run:** language, stores, covers, how to open Tako (in a window or full screen) and a summary of the games found. The choice becomes "Open in", in Settings › General.
- **Settings** with the same structure as the library, over gray; the controller battery shows in Controls.
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
