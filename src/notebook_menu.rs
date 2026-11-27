//! Native commands kept out of the writing surface.

use std::cell::RefCell;

use dioxus::desktop::muda::accelerator::{Accelerator, Code, Modifiers};
use dioxus::desktop::muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

pub(crate) const HOME: &str = "xuy-home";
pub(crate) const TOGGLE_SIDEBAR: &str = "xuy-toggle-sidebar";
pub(crate) const TOGGLE_PREVIEW: &str = "xuy-toggle-preview";
pub(crate) const SAVE: &str = "xuy-save";

pub(crate) const NEW_NOTE: &str = "xuy-new-note";
pub(crate) const NEW_FOLDER: &str = "xuy-new-folder";

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
    notebook: Submenu,
    new_note: MenuItem,
    new_folder: MenuItem,
    home: MenuItem,
    toggle_sidebar: MenuItem,
    toggle_preview: MenuItem,
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

    let new_note = MenuItem::with_id(
        NEW_NOTE,
        "New Note…",
        false,
        Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyN)),
    );
    let new_folder = MenuItem::with_id(
        NEW_FOLDER,
        "New Folder…",
        false,
        Some(Accelerator::new(
            Some(Modifiers::SUPER | Modifiers::SHIFT),
            Code::KeyN,
        )),
    );
    let home = MenuItem::with_id(HOME, "Home", false, None);
    let toggle_sidebar = MenuItem::with_id(
        TOGGLE_SIDEBAR,
        "Toggle Sidebar",
        false,
        Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyB)),
    );
    let toggle_preview = MenuItem::with_id(TOGGLE_PREVIEW, "Edit Source", false, None);
    let save = MenuItem::with_id(SAVE, "Save", false, None);
    let notebook = Submenu::with_items(
        "Notebook",
        false,
        &[
            &new_note,
            &new_folder,
            &home,
            &toggle_sidebar,
            &toggle_preview,
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
        notebook,
        new_note,
        new_folder,
        home,
        toggle_sidebar,
        toggle_preview,
        save,
    }));
    menu
}

pub(crate) fn update(open: bool, has_note: bool, preview: bool, dirty: bool, busy: bool) {
    COMMANDS.with_borrow(|commands| {
        if let Some(commands) = commands {
            commands.notebook.set_enabled(open);
            commands.new_note.set_enabled(open && !busy);
            commands.new_folder.set_enabled(open && !busy);
            commands.home.set_enabled(open && !busy);
            commands.toggle_sidebar.set_enabled(open);
            commands.toggle_preview.set_enabled(has_note && !busy);
            commands.toggle_preview.set_text(if preview {
                "Edit Source"
            } else {
                "Show Preview"
            });
            commands.save.set_enabled(dirty && !busy);
        }
    });
}
