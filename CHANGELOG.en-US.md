# Changelog

English (US) release notes, mirroring CHANGELOG.md (Brazilian Portuguese). Before publishing a version, add a `## [VERSION]` section to **both** files: the workflow publishes the section matching the tag (this file's, with a link to the Portuguese one) and fails if either is missing.

## [Unreleased]

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
