# xuy to-do

## Definition of done for every task

- **Complete.** Verify the full workflow in the app, including relevant edge cases, and run the applicable checks before marking the task done.
- **Modular.** Keep related code in focused modules with clear interfaces. Split distinct responsibilities and reuse existing code where it fits.
- **Fast.** Check that the affected interactions stay responsive with realistic input. Avoid redundant work and blocking the UI thread.
- **Battery-efficient.** Keep idle work low. Use events where possible, avoid unnecessary polling, and stop timers or background work when they are no longer needed.

## Main work

These tasks change shared app behavior, editor internals, macOS window behavior, stored notebook data, or performance sampling.

- [x] Implement dark theme and add a theme selector to **Settings**. Verify the landing page and notebook workspace in both themes.
- [x] Add a cursor movement option to **Settings** with smooth and immediate modes.
- [x] Add a bottom status bar that shows the active note, the heading and subheading at the cursor, and the note's word count.
- [x] Hide the macOS title bar and window controls when the user starts typing in the editor. Keep window controls accessible when the user leaves the writing view.
- [x] Make the app responsive across supported window sizes. Drag-resize the landing page and notebook workspace to verify smooth reflow and catch clipped content, layout jumps, or stuttering.
- [x] Add app-wide keyboard shortcuts for zooming in (`⌘+`) and out (`⌘−`). Verify that both work while the Markdown editor is focused.
- [x] Add editor cursor appearance options in **Settings** for bar, block, and underline shapes, with a separate blink on/off toggle. Verify each shape with blinking enabled and disabled in the Markdown editor.
- [x] Add cursor color and blink style options in **Settings**. Keep the existing blink on/off toggle and apply the chosen blink style when blinking is enabled. Verify both options with bar, block, and underline cursors.
- [x] Add a top-left performance panel for xuy's CPU and memory usage, with room for other metrics that help diagnose editor performance. Add a keyboard shortcut to show or hide it. Verify that sampling runs only while the panel is visible.
- [x] Add a **Notebooks** picker on `⌘P` for quick access to saved notebooks. Let users save an existing folder as a notebook in place and reopen it from the picker after relaunching. Keep **open** as the folder picker for new or specific folders.
- [x] Wire up the basic file and notebook shortcuts. Include creating a notebook, creating a folder or Markdown file in the current notebook, and opening an existing folder. Show the bindings in the native menus and starter-page key hints where applicable. Verify each shortcut from Home or with the editor focused, and check for conflicts with existing bindings, including `⌘P`.
- [x] Implement text highlighting with five color options. Verify that highlighted text stays readable and all five colors are distinguishable in both light and dark themes.
- [x] Add a secondary sidebar with a toggle between **Outline** and **Highlights** once text highlighting is implemented. In **Outline**, show headings and subheadings to the left of the page with subtle guide lines for the hierarchy. In **Highlights**, show the text highlighting options.

## Side work

These tasks can run alongside main work once the interfaces they use are stable. Write the docs against working features and finish the shortcut reference once the bindings are in place.

- [x] Make each starter-page action clickable with the mouse across its full row. Add a subtle hover background and verify mouse and keyboard activation in light and dark themes.
- [x] Add font size controls in **Settings** for the Settings screen and Markdown text. Verify that Markdown sizing applies in both edit and read views.
- [x] Add three Markdown page width presets in **Settings**. Verify **Narrow**, **Standard**, and **Wide** in both edit and read views.
- [x] Update the user docs end to end. Explain how to open a notebook, create and edit Markdown notes, navigate the app, and change settings. Include a reference for every supported keyboard shortcut, and verify the steps and shortcuts against the app.
  - [x] Document the current notebook workflow and supported shortcuts in [the user guide](docs/usage.md).
  - [x] Verify the guide in the app and include Settings instructions and all current shortcuts.
- [x] Replace the landing page's black hole with six locally bundled ASCII scenes, three for light theme and three for dark theme. By default, pick one of the active theme's three scenes at random on each launch with no prompt. In **Settings**, let users pin a scene for the active theme or switch back to **Random**. Remember each theme's choice across launches. Keep the art most visible at the top and fade it completely into the theme background around the home-page text so the controls stay readable. Remove the visible **xuy** heading to free up space. [ASCII scene gallery](https://ascii.rest/#scenes)

