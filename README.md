# xuy

xuy is a desktop Markdown notebook for macOS and Linux. Open a local folder, write in plain Markdown, and see the rendered page as you type. Your notes are ordinary files you can open in any text editor.

[Download xuy](https://github.com/vimzh/xuy/releases/latest) · [User guide](docs/usage.md)

![xuy in Light mode with a notebook sidebar, Markdown source, live preview, and a heading outline](docs/screenshots/workspace.webp)

## Install

| Platform | v0.1.0 download |
| --- | --- |
| macOS 14.5+, Apple Silicon | [Apple Silicon DMG](https://github.com/vimzh/xuy/releases/download/v0.1.0/xuy-0.1.0-macos-arm64.dmg) |
| macOS 14.5+, Intel | [Intel DMG](https://github.com/vimzh/xuy/releases/download/v0.1.0/xuy-0.1.0-macos-x86_64.dmg) |
| Ubuntu 24.04, x86-64 | [Linux DEB](https://github.com/vimzh/xuy/releases/download/v0.1.0/xuy-0.1.0-linux-amd64.deb) |

**macOS.** Open the DMG and drag **Xuy** into **Applications**. The app uses an ad hoc signature and is unnotarized. If macOS blocks the first launch, see [Apple’s instructions for opening the app](https://support.apple.com/en-us/102445).

**Linux.** From your download directory, run `sudo apt install ./xuy-0.1.0-linux-amd64.deb`. The Linux build is a preview. Notes persist on disk, and saved notebook shortcuts and several preferences reset when you relaunch. Session restore and the performance panel are currently macOS-only.

[Release notes](https://github.com/vimzh/xuy/releases/tag/v0.1.0) · [SHA-256 checksums](https://github.com/vimzh/xuy/releases/download/v0.1.0/SHA256SUMS)

## Start writing

1. Choose **open** on Home to browse an existing folder, or **notebooks** to add a folder you want to return to.
2. Open a Markdown file, or choose **new** on Home to create one. xuy saves after one second without edits.
3. Use the view menu to switch between **Edit**, **Preview**, and **Side by Side**. Select a heading in the right sidebar to jump to that section.

| Home | Command bar |
| --- | --- |
| ![Dark Home screen with monochrome ASCII artwork above links for new notes, folders, saved notebooks, settings, and docs](docs/screenshots/home.webp) | ![Dark command bar filtering the five writing fonts, with JetBrains Mono checked](docs/screenshots/command-bar.webp) |

The screenshots are from xuy running on macOS with a sample notebook.

## What you can do

- Use relative links and wikilinks to connect notes. Keep images beside a note and render them in Preview.
- Render tables, task lists, footnotes, and fenced code blocks with syntax highlighting. Choose a code theme and enable **Edit in Preview** to edit formatted text directly.
- Adjust the writing font, line spacing, and column width. Apply colored highlights that are saved in the Markdown file.
- Find notebook actions and settings in the command bar. Search for **Writing Font**, **Line Spacing**, or **Code Block Theme** to change them while writing.
- Open a terminal in the current notebook folder. xuy reloads clean notes when another program changes them and lets you save a copy of your draft if edits conflict.

### macOS shortcuts

| Shortcut | Action |
| --- | --- |
| `⌘K` | Open the command bar |
| `⌘O` | Open a folder |
| `⌘P` | Open saved notebooks |
| `⌘N` | Create a note |
| `⌘S` | Save the current note |
| `⇧⌘P` | Switch between Edit and Preview |
| `⌘,` | Open Settings |

See the [full user guide](docs/usage.md) for more shortcuts and details on notebook management, preview, and external edits. It’s also available offline from **docs** on Home.

## Develop

Built with Rust and Dioxus Desktop. Install [stable Rust](https://rustup.rs/) and [Dioxus CLI 0.7.10](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.10).

On Ubuntu 24.04, install the desktop build dependencies and the portal used by the file picker.

```sh
sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev xdg-desktop-portal xdg-desktop-portal-gtk
```

```sh
git clone https://github.com/vimzh/xuy.git
cd xuy
dx serve --desktop
```

Run the project checks before committing changes to Rust code.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The [product brief](PRD.md) covers the design and requirements.

## Credits

Home scenes by [ascii.rest](assets/ascii-rest/LICENSE.txt). Icons by [Hugeicons](assets/hugeicons/LICENSE.md). Bundled typefaces by [iA Writer](assets/writing-fonts/LICENSE-iA-Writer.md) and [JetBrains Mono](assets/writing-fonts/OFL-JetBrains.txt). The terminal uses [xterm.js](assets/xterm/LICENSE-xterm.txt) and its [fit addon](assets/xterm/LICENSE-addon-fit.txt).
