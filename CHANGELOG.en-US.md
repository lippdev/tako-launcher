# Changelog

English (US) release notes, mirroring CHANGELOG.md (Brazilian Portuguese). Before publishing a version, add a `## [VERSION]` section to **both** files: the workflow publishes the section matching the tag (this file's, with a link to the Portuguese one) and fails if either is missing.

## [Unreleased]

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
