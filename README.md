<p align="center"><img src="assets/logo.svg" width="120" alt="Dustr logo"></p>

# Dustr

A free, open-source (and vibe-coded) app that makes cleaning out your files quick and painless:
see one file at a time and press an arrow key to decide what happens to it.

## Install

**[Download the latest version](https://github.com/TETRA326/dustr/releases/latest/download/Dustr-Setup.exe)**, then open `Dustr-Setup.exe`.

1. If Windows shows "Windows protected your PC", click **More info → Run anyway**. (The installer isn't code-signed yet.)
2. Click **Install**. No administrator rights are needed.

Older versions are on the [Releases](../../releases) page.

## Usage

1. **Start Dustr** in one of two ways:
   - Right-click a folder (or empty space inside one) in File Explorer, choose **Show more options → Dustr**, then:
     - **Dust only this directory**: just the files in that folder.
     - **Dust all recursive folders**: that folder and every folder inside it.
   - Or open Dustr from the Start menu and click **Choose folder**.
2. **Answer any questions.** The first time, Dustr reminds you that it changes real files. It may also ask whether to include hidden files.
3. **Sort your files.** Each file is shown one at a time. Press an arrow key:

   | Key | What happens |
   | --- | --- |
   | → | **Keep** it where it is |
   | ← | Send it to the **Recycle Bin** |
   | ↑ | **Move** it to another folder (Dustr asks which folder the first time) |
   | ↓ ↓ | **Delete it permanently** (press twice; this cannot be undone) |

   Made a mistake? Press **Ctrl+Z** or click **Undo**. Press **Esc** to stop at any time.
4. **Done.** Dustr shows a short summary of what you did. If you started it from the right-click menu, it closes by itself.

WASD works too (W = ↑, A = ←, S = ↓, D = →). Every key can be changed or turned off in **Settings** (the icon in the top-right corner).

### Going through subfolders

With **Dust all recursive folders**, Dustr shows the name of each next folder for a few seconds before starting it:
press **→** to skip that folder's files (its subfolders are still visited), **←** to skip it and everything inside it, or **Enter** to start right away.
If a folder ends up empty, Dustr can offer to remove it.

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
