<p align="center"><img src="assets/logo.svg" width="120" alt="Dustr logo"></p>

# Dustr

FOSS app vibe-coded to make file management more intuative.

Right-click a folder in Explorer → **Show more options** → **Dustr**:

```
Dustr
└ Dust only this directory
└ Dust all recursive folders
```
## Install

**[Download the latest version](https://github.com/TETRA326/dustr/releases/latest/download/Dustr-Setup.exe)**

Or pick a specific version from [Releases](../../releases), then run `Dustr-Setup.exe`.
It installs per user (no admin), adds a Start menu shortcut and the right-click menu; a desktop shortcut is optional.
The installer is not code-signed yet, so Windows SmartScreen may ask for confirmation (**More info → Run anyway**).

When Dustr is started from the right-click menu, it closes on its own a few seconds after the summary.

## Controls (defaults, all configurable)

| Key | Action |
| --- | --- |
| ↑ / W | Move to a folder (asked on the first move of each session, or a fixed folder) |
| → / D | Keep |
| ← / A | Move to the Recycle Bin |
| ↓ / S | Permanently delete (press twice within 3 s; a quick double tap works) |
| Ctrl+Z | Undo last swipe (not available for permanent deletes) |
| Esc | End the session and show the summary |

Any direction can be set to *Nothing*.

**Recursive mode** goes through the chosen folder, then each subfolder (depth first). Before each subfolder a short countdown appears:
→ skips that folder's files but still visits its subfolders, ← skips the folder and everything under it, Enter starts immediately.
Subfolders left completely empty can be removed (Ask / Remove / Keep).

## Previews

Images, video, audio, PDF and text are shown directly. On Windows, Office documents and anything else with a registered
preview handler use the same previews as Explorer's preview pane. They load in the background a few files ahead and are view-only,
so the keyboard always stays with Dustr.

When no preview handler is installed (for example, without Microsoft Office), and on other platforms, .docx, .xlsx and .pptx
files get a basic built-in preview: document and slide text, or the first sheet as a grid.

## Settings

Theme (light / dark / system) · animations · direction mapping · arrow keys and/or WASD · move destination ·
delete confirmation and time window (0–10 s) · sort order (default: oldest modified first) · hidden and system files
(Ask / Include / Skip) · empty folders · next-folder countdown · document previews · Explorer right-click menu.

Settings are stored in `%APPDATA%\io.github.dustr\settings.json`.

## Build from source

Requirements: [Rust](https://rustup.rs), Node.js 18+, and on Windows the WebView2 runtime (included in Windows 11).

```
npm install
npm run dev      # run in development
npx tauri build --no-bundle
ISCC /DAppVersion=0.2.0 installer\dustr.iss   # Inno Setup 6, installer goes to dist/
```

Linux builds also compile (`sudo apt install libwebkit2gtk-4.1-dev librsvg2-dev`); the right-click menu and
document previews are Windows-only for now.

## Project layout

```
src/                  UI (plain HTML/CSS/JS, no bundler)
src-tauri/src/
  main.rs             command-line args, command registration
  fsops.rs            list, move, recycle, restore, delete
  preview.rs          Windows preview-handler host
  office.rs           built-in docx/xlsx/pptx reader
  shellmenu.rs        Explorer context menu (HKCU)
  settings.rs         settings file
installer/dustr.iss   Windows installer (Inno Setup), incl. the context menu
```

## License

GPL-3.0. See [LICENSE](LICENSE).
