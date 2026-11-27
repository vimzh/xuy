//! Native commands kept out of the writing surface.

use std::cell::RefCell;

use dioxus::desktop::muda::accelerator::{Accelerator, Code, Modifiers};
use dioxus::desktop::muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

use crate::editor::ViewMode;

pub(crate) const HOME: &str = "xuy-home";
pub(crate) const COMMAND_BAR: &str = "xuy-command-bar";
pub(crate) const TOGGLE_SIDEBAR: &str = "xuy-toggle-sidebar";
pub(crate) const TOGGLE_PREVIEW: &str = "xuy-toggle-preview";
pub(crate) const TOGGLE_SPLIT: &str = "xuy-toggle-split";
pub(crate) const PREVIEW_SHORTCUT: &str = "⇧⌘P";
pub(crate) const SAVE: &str = "xuy-save";

pub(crate) const NEW_NOTE: &str = "xuy-new-note";
pub(crate) const NEW_FOLDER: &str = "xuy-new-folder";
pub(crate) const NEW_NOTEBOOK: &str = "xuy-new-notebook";
pub(crate) const OPEN_FOLDER: &str = "xuy-open-folder";
pub(crate) const NOTEBOOKS: &str = "xuy-notebooks";

#[cfg(target_os = "macos")]
pub(crate) fn show_sidebar(window: &dioxus::desktop::tao::window::Window) {
    use dioxus::desktop::{muda::ContextMenu, tao::platform::macos::WindowExtMacOS};

    let menu = Menu::with_items(&[
        &MenuItem::with_id(NEW_NOTE, "New Page…", true, None),
        &MenuItem::with_id(NEW_FOLDER, "New Folder…", true, None),
    ])
    .expect("build sidebar menu");
    // Tao owns the view, and the desktop event loop runs on the main thread.
    unsafe { menu.show_context_menu_for_nsview(window.ns_view(), None) };
}

struct Commands {
    new_note: MenuItem,
    new_folder: MenuItem,
    home: MenuItem,
    toggle_sidebar: MenuItem,
    toggle_preview: MenuItem,
    toggle_split: CheckMenuItem,
    save: MenuItem,
}

thread_local! {
    static COMMANDS: RefCell<Option<Commands>> = const { RefCell::new(None) };
}

pub(crate) fn build(appearance: crate::theme::Appearance) -> Menu {
    let app = Submenu::with_items(
        "xuy",
        true,
        &[
            &MenuItem::with_id(
                COMMAND_BAR,
                "Command Bar…",
                true,
                Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyK)),
            ),
            &crate::theme::menu(appearance),
            &PredefinedMenuItem::fullscreen(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::close_window(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ],
    )
    .expect("build application menu");
    let edit = Submenu::with_items(
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::select_all(None),
        ],
    )
    .expect("build editing menu");

    let home = MenuItem::with_id(HOME, "Home", false, None);
    let toggle_sidebar = MenuItem::with_id(
        TOGGLE_SIDEBAR,
        "Toggle Sidebar",
        false,
        Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyB)),
    );
    let command = |id, text, enabled, modifiers, key| {
        MenuItem::with_id(
            id,
            text,
            enabled,
            Some(Accelerator::new(Some(modifiers), key)),
        )
    };
    let toggle_preview = command(
        TOGGLE_PREVIEW,
        ViewMode::Edit.preview_action(),
        false,
        Modifiers::SUPER | Modifiers::SHIFT,
        Code::KeyP,
    );
    let toggle_split = CheckMenuItem::with_id(TOGGLE_SPLIT, "Side by Side", false, false, None);
    let new_note = command(NEW_NOTE, "New Note…", true, Modifiers::SUPER, Code::KeyN);
    let new_folder = command(
        NEW_FOLDER,
        "New Folder…",
        false,
        Modifiers::SUPER | Modifiers::SHIFT,
        Code::KeyN,
    );
    let new_notebook = command(
        NEW_NOTEBOOK,
        "New Notebook…",
        true,
        Modifiers::SUPER | Modifiers::ALT,
        Code::KeyN,
    );
    let open_folder = command(
        OPEN_FOLDER,
        "Open Folder…",
        true,
        Modifiers::SUPER,
        Code::KeyO,
    );
    let notebooks = command(NOTEBOOKS, "Notebooks…", true, Modifiers::SUPER, Code::KeyP);
    let save = command(SAVE, "Save", false, Modifiers::SUPER, Code::KeyS);
    let notebook = Submenu::with_items(
        "Notebook",
        true,
        &[
            &new_note,
            &new_folder,
            &new_notebook,
            &open_folder,
            &notebooks,
            &PredefinedMenuItem::separator(),
            &home,
            &toggle_sidebar,
            &toggle_preview,
            &toggle_split,
            &save,
        ],
    )
    .expect("build notebook menu");
    let menu = Menu::with_items(&[&app, &edit, &notebook]).expect("build menu bar");

    #[cfg(debug_assertions)]
    {
        let help = Submenu::with_items(
            "Help",
            true,
            &[
                &MenuItem::with_id(
                    "dioxus-toggle-dev-tools",
                    "Toggle Developer Tools",
                    true,
                    None,
                ),
                &MenuItem::with_id(
                    "dioxus-float-top",
                    "Float on Top (dev mode only)",
                    true,
                    None,
                ),
            ],
        )
        .expect("build development menu");
        menu.append(&help).expect("add development menu");
    }

    COMMANDS.set(Some(Commands {
        new_note,
        new_folder,
        home,
        toggle_sidebar,
        toggle_preview,
        toggle_split,
        save,
    }));
    menu
}

pub(crate) fn update(open: bool, can_view: bool, view_mode: ViewMode, dirty: bool, busy: bool) {
    COMMANDS.with_borrow(|commands| {
        if let Some(commands) = commands {
            commands.new_note.set_enabled(!busy);
            commands.new_folder.set_enabled(open && !busy);
            commands.home.set_enabled(open && !busy);
            commands.toggle_sidebar.set_enabled(open);
            commands.toggle_preview.set_enabled(can_view);
            commands.toggle_preview.set_text(view_mode.preview_action());
            commands.toggle_split.set_enabled(can_view);
            commands
                .toggle_split
                .set_checked(view_mode == ViewMode::Split);
            commands.save.set_enabled(dirty && !busy);
        }
    });
}
