# xuy product requirements

xuy is a minimal, monochrome personal notes app for macOS, built in Rust with Dioxus Desktop. It should make it easy to start writing, find a note by folder, and choose a writing font.

## First version

| Area | Requirement | Acceptance check |
| --- | --- | --- |
| Launch screen | Center the regular-weight xuy name beneath the first frame of the black hole ASCII art. Show **new**, **open**, **settings**, and **docs** in that order, one option per row, with **Recently Opened** below. Use a light, monochrome layout with monospace text throughout. [Black hole by ascii.rest](https://ascii.rest/black-hole/) | Launch the app to see the black hole above xuy and four borderless rows in a narrow centered column. Each row has a Hugeicons outline icon followed by its label on the left and a boxed placeholder shortcut on the right. |
| Notebook | Open a local folder as a notebook with the macOS folder picker. When no notebook is open, **new** first asks for a folder. | Open a folder and see its Markdown files and subfolders in the sidebar. |
| Notes | Create a named Markdown file in the current folder. Open notes in edit mode by default and offer a rendered Markdown preview. | Create a note, enter Markdown, and save it. Reopen the note and check that its source is focused and ready to edit. Choose Show Preview to see the rendered content. |
| Writing | Center a left-aligned text column in the editor. Put the sidebar toggle, note history, centered filename, writing-width picker, and Preview toggle in the title bar. Keep Home, Edit Source or Show Preview, and Save in the macOS Notebook menu. | Open two notes and navigate between them. Change the writing width and toggle Preview. Type in a note to hide the window controls while keeping the filename visible and the text in place. Move the pointer to the top or press Escape to reveal the controls. |
| Folders | Browse existing subfolders and create notes inside them. Show a compact sort control, file modification dates on the left, page icons, and two-line excerpts. Limit excerpts to the current folder and load full text for the selected note. | Browse a nested folder and sort its notes by name or date. Check that opening a note shows its full content. |
| Fonts | Offer iA Writer Mono, iA Writer Duo, iA Writer Quattro, and Helvetica Neue. [iA’s font downloads](https://ia.net/downloads/fonts) | Check that the picker lists all four families and applies each one to the editor when selected. |

## Working assumptions

- xuy starts as a single-user app. Notes and folders are stored locally and can be reopened after quitting and relaunching without an internet connection.

## Design decisions for later passes

- Decide the window layout, spacing, and visual style step by step in the running desktop app.
- Decide how folder creation and moving notes should work, where the font picker lives, and whether the selected font applies across the app or per note.

## Development sequence

1. Run a minimal Dioxus desktop window on the Mac with `dx serve --desktop`.
2. Build the launch screen, then the sidebar and editor as the visual direction is established.
3. Implement notes, folders, local persistence, and the editor font picker. Verify the first-version checks on the Mac.
