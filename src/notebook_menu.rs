//! Native commands kept out of the writing surface.

use std::cell::RefCell;

use dioxus::desktop::muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

pub(crate) const HOME: &str = "xuy-home";
pub(crate) const TOGGLE_PREVIEW: &str = "xuy-toggle-preview";
pub(crate) const SAVE: &str = "xuy-save";

struct Commands {
    notebook: Submenu,
    home: MenuItem,
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

    let home = MenuItem::with_id(HOME, "Home", false, None);
    let toggle_preview = MenuItem::with_id(TOGGLE_PREVIEW, "Edit Source", false, None);
    let save = MenuItem::with_id(SAVE, "Save", false, None);
    let notebook = Submenu::with_items("Notebook", false, &[&home, &toggle_preview, &save])
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
        home,
        toggle_preview,
        save,
    }));
    menu
}

pub(crate) fn update(open: bool, has_note: bool, preview: bool, dirty: bool, busy: bool) {
    COMMANDS.with_borrow(|commands| {
        if let Some(commands) = commands {
            commands.notebook.set_enabled(open);
            commands.home.set_enabled(open && !busy);
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
