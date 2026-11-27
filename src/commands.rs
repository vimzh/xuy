//! Available command bar actions and their current settings.

use crate::{
    command_bar::Command, editor::ViewMode, notebook_menu, page_width::PageWidth, theme::Appearance,
};

pub(crate) const PREVIOUS_PAGE: &str = "previous-page";
pub(crate) const NEXT_PAGE: &str = "next-page";
pub(crate) const ZOOM_IN: &str = "zoom-in";
pub(crate) const ZOOM_OUT: &str = "zoom-out";
pub(crate) const RESET_ZOOM: &str = "reset-zoom";

const WIDTHS: [(PageWidth, &str, &str); 3] = [
    (PageWidth::Narrow, "width-narrow", "Narrow Writing Width"),
    (
        PageWidth::Standard,
        "width-standard",
        "Standard Writing Width",
    ),
    (PageWidth::Wide, "width-wide", "Wide Writing Width"),
];

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PageActions {
    pub(crate) has_page: bool,
    pub(crate) view_mode: ViewMode,
    pub(crate) dirty: bool,
    pub(crate) can_back: bool,
    pub(crate) can_forward: bool,
}

pub(crate) fn notebook_commands(in_notebook: bool, page: PageActions) -> Vec<Command> {
    use notebook_menu::*;
    let mut commands = vec![
        Command::new(
            NEW_NOTEBOOK,
            "New Project…",
            "add-01",
            "⌥⌘N",
            "new notebook create",
        ),
        Command::new(
            OPEN_FOLDER,
            "Open Folder…",
            "folder-open",
            "⌘O",
            "open project notebook",
        ),
        Command::new(
            NOTEBOOKS,
            "Saved Notebooks…",
            "notebook-01",
            "⌘P",
            "projects switch",
        ),
        Command::new(NEW_NOTE, "New Page…", "file-02", "⌘N", "new note create"),
    ];
    if in_notebook {
        commands.extend([
            Command::new(HOME, "Go Home", "home-01", "", "start landing"),
            Command::new(
                NEW_FOLDER,
                "New Folder…",
                "folder-01",
                "⇧⌘N",
                "subfolder create",
            ),
            Command::new(
                TOGGLE_SIDEBAR,
                "Toggle Sidebar",
                "sidebar-left",
                "⌘B",
                "show hide files",
            ),
        ]);
        if page.has_page {
            commands.push(Command::new(
                TOGGLE_PREVIEW,
                page.view_mode.preview_action(),
                "view",
                PREVIEW_SHORTCUT,
                "page markdown rendered raw toggle edit",
            ));
            commands.push(Command::new(
                TOGGLE_SPLIT,
                if page.view_mode == ViewMode::Split {
                    "Close Side by Side"
                } else {
                    "Open Side by Side"
                },
                "sidebar-left",
                "",
                "split view markdown preview editor",
            ));
        }
        if page.has_page && page.dirty {
            commands.push(Command::new(
                SAVE,
                "Save Page",
                "file-02",
                "⌘S",
                "save note write",
            ));
        }
        if page.can_back {
            commands.push(Command::new(
                PREVIOUS_PAGE,
                "Previous Page",
                "arrow-left-01",
                "",
                "go back history",
            ));
        }
        if page.can_forward {
            commands.push(Command::new(
                NEXT_PAGE,
                "Next Page",
                "arrow-right-01",
                "",
                "go forward history",
            ));
        }
    }
    commands
}

pub(crate) fn settings(appearance: Appearance, width: PageWidth) -> impl Iterator<Item = Command> {
    let themes = crate::theme::OPTIONS.map(|(option, id, _)| {
        let label = match option {
            Appearance::System => "System Theme",
            Appearance::Light => "Light Theme",
            Appearance::Dark => "Dark Theme",
        };
        Command::new(
            id,
            label,
            "settings-01",
            "",
            "settings appearance color mode",
        )
        .setting(appearance == option)
    });
    let widths = WIDTHS.map(|(option, id, label)| {
        Command::new(
            id,
            label,
            "arrow-horizontal",
            "",
            "settings editor page width",
        )
        .setting(width == option)
    });
    let zoom = [
        Command::new(
            ZOOM_IN,
            "Zoom In",
            "add-01",
            "⌘+",
            "settings interface scale larger",
        ),
        Command::new(
            ZOOM_OUT,
            "Zoom Out",
            "remove-01",
            "⌘−",
            "settings interface scale smaller",
        ),
        Command::new(
            RESET_ZOOM,
            "Reset Zoom",
            "arrow-horizontal",
            "",
            "settings interface scale 100% default",
        ),
    ];
    themes
        .into_iter()
        .chain(widths)
        .chain(zoom.map(|command| command.setting(false)))
}

pub(crate) fn width_for_id(id: &str) -> Option<PageWidth> {
    WIDTHS
        .iter()
        .find_map(|&(width, option_id, _)| (option_id == id).then_some(width))
}
