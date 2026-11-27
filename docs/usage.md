# Using xuy

## Open and save notebooks

Click **open** on Home or press `⌘O` to choose a folder in the macOS picker. xuy shows its subfolders and Markdown files in the sidebar. Notes are ordinary `.md` or `.markdown` files stored in that folder.

Click **notebooks** on Home, press `⌘P`, or choose **Notebook > Notebooks** to see your saved notebooks. Choose **Add Folder…** to save an existing folder in place, then select its row to open it. To start a new notebook, press `⌥⌘N` or choose **Notebook > New Notebook…**. Use **New Folder** in the macOS picker, then select that folder. xuy opens it and adds it to **Notebooks** automatically. Saved notebooks stay available after you quit and reopen xuy.

When you switch notebooks with unsaved edits, **Keep editing** keeps your draft open. **Discard changes** switches to the other notebook.

Click a subfolder to browse it. The current folder appears in the title bar next to the back and forward arrows. Use the up arrow beside the folder name to return to its parent. The sidebar lists folders first and shows the most recently modified notes first. Click a note to open its Markdown source.

When you reopen xuy, it returns to the last notebook, sidebar folder, and active page. The page opens in edit mode. Choose **Notebook > Home** before quitting to start at Home next time.

Press `⌘K` to open the command bar from Home or while writing. Type to filter the list, use `↑` and `↓` to select an action, and press `Return` to run it. **New Project** opens the new notebook picker. **Go Home** returns to Home. Press `Esc` or `⌘K` again to close the bar.

Search for a setting such as **Writing Font**, **Line Spacing**, **Code Block Theme**, or **Cursor Shape** to change it directly. The current choices have checkmarks. With a page open, use **Show Preview**, **Edit Source**, or **Open Side by Side**. **Save Page** appears when the page has unsaved edits, and **Previous Page** and **Next Page** appear when there is page history in that direction.

Drag the divider between the sidebar and editor to change the sidebar width. With the divider focused, use the left and right arrow keys to resize it, or use `Home` and `End` for its width limits. In a narrow window, the file list moves above the editor. Use the sidebar button in the title bar to give the editor the full width.

## Create, edit, and save a note

1. On Home, click **new** or press `⌘N`, then choose a folder. In an open notebook, navigate to the folder where you want the note and press `⌘N` or choose **Notebook > New Note…**.
2. Enter a **Note name** and click **Create**. xuy adds `.md` when the name has no Markdown extension.
3. Write in the editor. xuy saves after one second without edits. Press `⌘S` or choose **Notebook > Save** to save immediately, including before closing the window or quitting.

To make a subfolder, press `⇧⌘N` or choose **Notebook > New Folder…**, enter a **Folder name**, and click **Create**. The folder appears in the current folder’s sidebar. Any draft stays open.

You can also right-click the sidebar, or press `⇧F10` with it focused, and choose **New Page…** or **New Folder…**. Both create an item in the folder currently open in the sidebar.

In Edit or Side by Side, use **Insert Code Block** in `⌘K` to add a fenced Markdown block at the cursor or wrap selected text. Add a language name such as `rust` or `python` after the opening backticks for syntax highlighting in Preview. Set **Code Block Theme** to **System**, **Paper**, or **Graphite** in Settings or `⌘K`.

The title bar and filename hide when you type, and the text stays in place. Move focus out of the editor, press `Esc`, or move the pointer to the top of the window to bring the title bar back. If you switch notes or return home with unsaved edits, **Keep editing** returns you to your draft. **Discard changes** continues without saving it.

## Use the notebook terminal

Click **Terminal** at the bottom of an open notebook, or press `⌘K` and choose **Open Terminal**. The shell starts in the folder shown in the sidebar. Run your preferred CLI there, including any LLM tool you have installed.

Use **Minimize Terminal** in `⌘K` to hide the terminal while its commands keep running. **Open Terminal** restores that session. **Close Terminal** ends it. The panel also has **Minimize** and **Close** buttons. Switching notebooks or returning Home ends the current session.

xuy reloads a clean note when another program writes to it. This works in Edit and Preview. When the disk and your unsaved draft have both changed, autosave pauses. Expand **Review disk version** to compare the other version with your draft. Choose **Save Draft as Copy** to open a separate Markdown file containing your draft, or **Use Disk Version** to discard the unsaved draft and open the latest file from disk.

## Read the Markdown preview

Press `⇧⌘P` to switch between Markdown source and the rendered preview. The eye menu in the title bar also offers **Edit**, **Preview**, and **Side by Side**. Choose **Open Side by Side** in `⌘K` or **Notebook > Side by Side** to edit on the left and see the rendered draft update on the right. Press `⇧⌘P` from Side by Side to return to Edit. The horizontal arrow opens the width picker with **Narrow**, **Standard**, and **Wide**.

Preview renders headings, emphasis, lists, quotes, code blocks, tables, strikethrough, task lists, footnotes, and GitHub-style callouts. Bare web URLs and email addresses become clickable links. Raw HTML displays as text.

Use `[Next page](Next%20page.md)`, `[[Next page]]`, or `[Section](#section)` to navigate in Preview. Local links and images resolve relative to the current note. Preview displays notebook PNG, JPEG, GIF, and WebP images and can also load web images.

Drop PNG, JPEG, GIF, or WebP files from Finder onto the Markdown source in Edit or Side by Side. xuy copies the files beside the note and inserts them below the line you dropped onto, with each image in its own Markdown block. Each file can be up to 12 MiB.

The bottom bar shows the file name, the heading path at your cursor, and the full draft’s word count. **Body** indicates that the cursor is outside a heading section. Preview keeps the last editing location in the bar.

You can also change **Writing width** in Settings. Settings and the toolbar share the same selection for Edit and Preview.

## Use the outline and highlights

The **Outline** in the right sidebar lists the active note’s headings and subheadings. Indents and guide lines show the hierarchy. Select a heading to jump to it in Edit or Preview.

In Edit, choose **Highlights**, select regular Markdown text, and pick **Yellow**, **Mint**, **Sky**, **Rose**, or **Lavender**. To recolor a highlight or clear it, select it or place the cursor inside it, then choose another color or **Clear highlight**. The colors adapt to Light and Dark. Highlights are saved in the Markdown file. Edit shows the inline markup and Preview shows the colored text.

## Change appearance

Open **settings** from Home or press `⌘,`. Choose **System**, **Light**, or **Dark** under **Theme**. **xuy > Appearance** offers the same choices. System follows your Mac’s appearance. xuy remembers your choice when you relaunch.

Under **Landing scene**, leave **Random** selected to draw from your theme’s three scenes each time xuy launches. Choose a scene by name to pin it. Light and Dark each remember their own choice.

**Settings font size** changes the text in Settings. **Markdown font size** changes the note text in Edit and Preview. The two sizes are saved separately.

**Writing Font** changes the note typeface in Edit and Preview. Choose iA Writer Mono, iA Writer Duo, iA Writer Quattro, Helvetica Neue, or JetBrains Mono. **Line Spacing** offers Compact, Standard, and Relaxed. xuy remembers both choices.

## Change the editor cursor

Open **settings** from Home or press `⌘,` while editing. Choose **Bar**, **Block**, or **Underline** under **Cursor shape**. Under **Cursor movement**, use **Immediate** for direct movement or **Smooth** for a short transition. Smooth movement respects macOS Reduce Motion.

Enable **Custom cursor color** to choose a color, or turn it off to use the theme color. Turn off **Blink cursor** for a steady cursor. When blinking is on, choose **Sharp** or **Fade** under **Blink style**. xuy remembers these choices when you relaunch.

## Check performance

Press `⌘I` or choose **xuy > Performance Panel** to toggle CPU usage and physical memory footprint for the main xuy process. The readings update about once a second while the panel is visible. CPU usage of 100% represents one fully used core.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `⌘K` | Open or close the command bar. |
| `⌘N` | Start a note. On Home, choose a folder first. |
| `⇧⌘N` | Create a folder inside the current notebook folder. |
| `⌥⌘N` | Choose or create a folder and save it as a notebook. |
| `⌘O` | Open a folder from the macOS picker. |
| `⌘P` | Open the saved notebooks picker. |
| `⌘S` | Save the current note. |
| `⇧⌘P` | Switch between Markdown source and rendered preview. From Side by Side, return to Edit. |
| `⌘B` | Show or hide the sidebar in an open notebook. |
| `⌘+` / `⌘−` | Zoom the app interface in or out. `⌘=` also zooms in. |
| `⌘,` | Open Settings. |
| `⌘D` | Open this guide. |
| `⌘I` | Show or hide the performance panel. |
| `Esc` | Close a dialog, reveal the window controls, or choose **Keep editing** in the unsaved-changes prompt. |
| `←` / `→`, `Home` / `End` | Resize the sidebar when the divider has focus. |
| `⌘Z` / `⇧⌘Z` | Undo / redo. |
| `⌘X` / `⌘C` / `⌘V` | Cut / copy / paste. |
| `⌘A` | Select all in the focused field. |
| `⌃⌘F` | Toggle full screen. |
| `⌘M` | Minimize the window. |
| `⌘H` / `⌥⌘H` | Hide xuy / hide other apps. |
| `⌘W` / `⌘Q` | Close the window / quit xuy. |

Use **docs** on Home to read this guide offline.
