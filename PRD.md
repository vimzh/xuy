# Dojo product requirements

Dojo is a minimal personal notes app for macOS, built in Rust with Dioxus Desktop. It should make it easy to start writing, find a note by folder, and choose a writing font.

## First version

| Area | Requirement | Acceptance check |
| --- | --- | --- |
| Sidebar | Show a **New Note** action and **Folders**. | The sidebar exposes those two entries. |
| Notes | Create a note, write and edit its content, and reopen it later. | Create a note, enter text, switch away, and reopen it with the saved text intact. |
| Folders | Create folders and use them to organize and browse notes. | Create a folder, place a note in it, and reopen the note by browsing that folder. |
| Fonts | Offer iA Writer Mono, iA Writer Duo, and iA Writer Quattro. [iA’s font downloads](https://ia.net/downloads/fonts) | Check that the picker lists exactly those three families and applies each one to the editor when selected. |

## Working assumptions

- Dojo starts as a single-user app. Notes and folders are stored locally and can be reopened after quitting and relaunching without an internet connection.
- Dojo is the working name, taken from the project directory.

## Design decisions for later passes

- Decide the window layout, spacing, and visual style step by step in the running desktop app.
- Decide where notes appear when a folder is selected, where the font picker lives, and whether the selected font applies across the app or per note.

## Development sequence

1. Run a minimal Dioxus desktop window on the Mac with `dx serve --desktop`.
2. Build the sidebar and editor as the visual direction is established.
3. Implement notes, folders, local persistence, and the three-font picker. Verify the first-version checks on the Mac.
