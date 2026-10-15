# Changelog

English (US) release notes, mirroring CHANGELOG.md (Brazilian Portuguese). Before publishing a version, add a `## [VERSION]` section to **both** files: the workflow publishes the section matching the tag (this file's, with a link to the Portuguese one) and fails if either is missing.

## [Unreleased]

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
