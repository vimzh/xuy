# xuy product requirements

xuy is a minimal, monochrome personal notes app for macOS, built in Rust with Dioxus Desktop. It should make it easy to start writing, find a note by folder, and choose a writing font.

## First version

| Area | Requirement | Acceptance check |
| --- | --- | --- |
| Launch screen | Center the regular-weight xuy name beneath the first frame of the black hole ASCII art. Show **new**, **open**, **settings**, and **docs** in that order, one option per row, with **Recently Opened** below. Use a light, monochrome layout with monospace text throughout. [Black hole by ascii.rest](https://ascii.rest/black-hole/) | Launch the app to see the black hole above xuy and four borderless rows in a narrow centered column. Each row has a Hugeicons outline icon followed by its label on the left and a boxed placeholder shortcut on the right. |
| Notebook | Open a local folder as a notebook with the macOS folder picker. When no notebook is open, **new** first asks for a folder. | Open a folder and see its Markdown files and subfolders in the sidebar. |
| Notes | Create a named Markdown file in the current folder, using `.md` by default. Open notes in edit mode by default and offer a rendered Markdown preview. | Create a note, enter Markdown, and save it. Reopen the note and check that its source is focused and ready to edit. Choose Show Preview to see the rendered content. |
| Writing | Center a left-aligned text column in the editor. Put the sidebar toggle, note history, current folder, centered filename, writing-width menu, and view-mode menu in the title bar. Keep Home, Edit Source or Show Preview, and Save in the macOS Notebook menu. | Open two notes and navigate between them. Change the writing width and switch between Edit and Preview from the toolbar menus. Type in a note to hide the window controls while keeping the filename visible and the text in place. Move the pointer to the top or press Escape to reveal the controls. |
| Folders | Browse existing subfolders and create notes inside them. List folders first and notes by most recent modification. Show each note's modification date on the left, page icon, and title without its Markdown extension on one line. Show up to two lines of excerpt below the title. Limit excerpts to the current folder and load full text for the selected note. | Open a nested folder and check that its name appears next to the history arrows. Use the up arrow to return to its parent. Check that opening a note shows its full content. |
| Fonts | Offer iA Writer Mono, iA Writer Duo, iA Writer Quattro, and Helvetica Neue. [iA’s font downloads](https://ia.net/downloads/fonts) | Check that the picker lists all four families and applies each one to the editor when selected. |
| Appearance | Offer System, Light, and Dark under the macOS app menu. Keep the warm writing page distinct from the window decoration, and remember the chosen appearance. | Switch appearances while editing or previewing a note. Relaunch to confirm the choice is retained, then choose System to follow macOS. |

## Working assumptions

- xuy starts as a single-user app. Notes and folders are stored locally and can be reopened after quitting and relaunching without an internet connection.

## Design decisions for later passes

- Decide the window layout, spacing, and visual style step by step in the running desktop app.
- Decide how folder creation and moving notes should work, where the font picker lives, and whether the selected font applies across the app or per note.

## Development sequence

1. Run a minimal Dioxus desktop window on the Mac with `dx serve --desktop`.
2. Build the launch screen, then the sidebar and editor as the visual direction is established.
3. Implement notes, folders, local persistence, and the editor font picker. Verify the first-version checks on the Mac.
