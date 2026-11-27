# xuy product requirements

xuy is a minimal, monochrome personal notes app for macOS, built in Rust with Dioxus Desktop. It should make it easy to start writing, find a note by folder, and choose a writing font.

## First version

| Area | Requirement | Acceptance check |
| --- | --- | --- |
| Launch screen | Center the regular-weight xuy name beneath the first frame of the black hole ASCII art. Show **new**, **open**, **settings**, and **docs** in that order, one option per row, with **Recently Opened** below. Use a light, monochrome layout with monospace text throughout. [Black hole by ascii.rest](https://ascii.rest/black-hole/) | Launch the app to see the black hole above xuy and four borderless rows in a narrow centered column. Each row has a Hugeicons outline icon followed by its label on the left and a boxed placeholder shortcut on the right. |
| Notebook | Open a local folder as a notebook with the macOS folder picker. When no notebook is open, **new** first asks for a folder. | Open a folder and see its Markdown files and subfolders in the sidebar. |
| Notes | Create a named Markdown file in the current folder, using `.md` by default. Open notes in edit mode by default and offer a rendered Markdown preview. Save automatically after one second without edits and keep `⌘S` for an immediate save. | Open a note, type Markdown, pause for one second, and check the file on disk. Reopen the note and check that its source is focused and ready to edit. Choose Show Preview to see the rendered content. |
| Session | Reopen the last notebook, sidebar folder, and active page in edit mode after closing xuy. If the notebook or page is unavailable, show a message and reopen what is still accessible. Returning to Home clears the open session. | Open a page in a subfolder, quit, and reopen xuy. Check the sidebar folder, active page, and editor focus. Move the page while xuy is closed, then reopen and check the notebook stays accessible with an error message. |
| Command bar | Open a centered, searchable list of commands with `⌘K`. Use Hugeicons for project, navigation, page, and settings actions. Offer theme, writing width, and zoom directly. Show Save Page and page history commands when available. | Open the bar from Home and while editing. Use the arrow keys and Return to run a command, and Escape to return to writing. Search for a theme or writing width, apply it, and reopen the bar to check its selection mark. |
| Writing | Center a left-aligned text column in the editor. Put the sidebar toggle, note history, current folder, centered filename, writing-width menu, and view-mode menu in the title bar. Keep Home, Toggle Sidebar (⌘B), Edit Source or Show Preview, and Save in the macOS Notebook menu. Use `⌘+` and `⌘−` to zoom the app, including while editing. | Open two notes and navigate between them. Change the writing width and switch between Edit and Preview from the toolbar menus. Type in a note to hide the title bar and filename while keeping the text in place. Zoom in and out with the editor focused and check that typing can continue. Press ⌘B to close and reopen the sidebar. Move the pointer to the top or press Escape to reveal the title bar. |
| Folders | Browse existing subfolders and use `⇧⌘N` to create one inside the current folder. Right-click the sidebar to create a page or subfolder in the current folder. List folders first and notes by most recent modification. Each note uses one row with its icon and title on the left and its local modification time for today or date for earlier days on the right. Omit Markdown extensions from titles. Load the note text when it is opened. Resize the sidebar by dragging its divider or using the arrow keys when the divider is focused. | Open a folder with notes modified on different days and check that each row has just the icon, title, and right-aligned time or date. Right-click the sidebar in an empty subfolder and create a page and a folder there. Open a note to see its full content. Resize the sidebar, then close and reopen it with ⌘B to check that the width is retained. Use the up arrow beside the folder name to return to its parent. |
| Fonts | Offer iA Writer Mono, iA Writer Duo, iA Writer Quattro, and Helvetica Neue. [iA’s font downloads](https://ia.net/downloads/fonts) | Check that the picker lists all four families and applies each one to the editor when selected. |
| Appearance | Offer System, Light, and Dark under the macOS app menu. Use neutral tones in both themes and keep the writing page subtly distinct from the window decoration. Remember the chosen appearance. | Switch appearances while editing or previewing a note. Relaunch to confirm the choice is retained, then choose System to follow macOS. |

## Working assumptions

- xuy starts as a single-user app. Notes and folders are stored locally and can be reopened after quitting and relaunching without an internet connection.

## Design decisions for later passes

- Decide the window layout, spacing, and visual style step by step in the running desktop app.
- Decide how folder creation and moving notes should work, where the font picker lives, and whether the selected font applies across the app or per note.

## Development sequence

1. Run a minimal Dioxus desktop window on the Mac with `dx serve --desktop`.
2. Build the launch screen, then the sidebar and editor as the visual direction is established.
3. Implement notes, folders, local persistence, and the editor font picker. Verify the first-version checks on the Mac.
