<p align="center"><img src="assets/logo.svg" width="96" alt="Dustr logo"></p>

# Dustr

Free, open-source app that makes cleaning out folders quick: one file at a time, one key press per decision.

Right-click a folder in Explorer → **Show more options** → **Dustr**:

```
Dustr
└ Dust only this directory
└ Dust all recursive folders
```

## Controls (defaults, all configurable)

| Key | Action |
| --- | --- |
| ↑ / W | Move to a folder (asked on the first move of each session, or a fixed folder) |
| → / D | Keep |
| ← / A | Move to the Recycle Bin |
| ↓ / S | Permanently delete (press twice within 3 s) |
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

## Install

Download the installer from [Releases](../../releases) (or the latest *Build* workflow run) and run it.
It installs per user (no admin) and adds the right-click menu. The installer is not code-signed yet,
so Windows SmartScreen may ask for confirmation (**More info → Run anyway**).

## Build from source

Requirements: [Rust](https://rustup.rs), Node.js 18+, and on Windows the WebView2 runtime (included in Windows 11).

```
npm install
npm run dev      # run in development
npm run build    # installer in src-tauri/target/release/bundle/nsis
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
src-tauri/windows/hooks.nsh   installer hooks for the context menu
```

## License

GPL-3.0. See [LICENSE](LICENSE).
