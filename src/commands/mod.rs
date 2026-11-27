//! Available command bar actions and their current settings.

pub(crate) mod native;
pub(crate) mod palette;

use crate::workspace::terminal::{self, State};
use crate::{
    editor::ViewMode,
    settings::{page_width::PageWidth, theme::Appearance},
};

use self::palette::Command;

pub(crate) const PREVIOUS_PAGE: &str = "previous-page";
pub(crate) const NEXT_PAGE: &str = "next-page";
pub(crate) const INSERT_CODE_BLOCK: &str = "insert-code-block";
pub(crate) const ZOOM_IN: &str = "zoom-in";
pub(crate) const ZOOM_OUT: &str = "zoom-out";
pub(crate) const RESET_ZOOM: &str = "reset-zoom";

pub(crate) fn terminal_commands(state: State) -> Vec<Command> {
    let mut commands = Vec::new();
    if state != State::Visible {
        commands.push(Command::new(
            terminal::OPEN,
            "Open Terminal",
            "add-01",
            "",
            "shell show restore",
        ));
    } else {
        commands.push(Command::new(
            terminal::MINIMIZE,
            "Minimize Terminal",
            "remove-01",
            "",
            "shell hide",
        ));
    }
    if state != State::Closed {
        commands.push(Command::new(
            terminal::CLOSE,
            "Close Terminal",
            "cancel-01",
            "",
            "shell quit end",
        ));
    }
    commands
}

const WIDTHS: [(PageWidth, &str, &str); 3] = [
    (PageWidth::Narrow, "width-narrow", "Narrow"),
    (PageWidth::Standard, "width-standard", "Standard"),
    (PageWidth::Wide, "width-wide", "Wide"),
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
    use native::*;
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
            if page.view_mode != ViewMode::Preview {
                commands.push(Command::new(
                    INSERT_CODE_BLOCK,
                    "Insert Code Block",
                    "file-02",
                    "",
                    "markdown fenced code snippet add wrap selection",
                ));
            }
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

pub(crate) fn settings(
    appearance: Appearance,
    width: PageWidth,
    edit_in_preview: bool,
) -> impl Iterator<Item = Command> {
    let themes = crate::settings::theme::OPTIONS.map(|(option, id, _)| {
        let label = match option {
            Appearance::System => "System",
            Appearance::Light => "Light",
            Appearance::Dark => "Dark",
        };
        Command::new(
            id,
            "Theme",
            "settings-01",
            "",
            "settings appearance color mode",
        )
        .setting_value(label, appearance == option)
    });
    let widths = WIDTHS.map(|(option, id, label)| {
        Command::new(
            id,
            "Writing Width",
            "arrow-horizontal",
            "",
            "settings editor page width",
        )
        .setting_value(label, width == option)
    });
    let preview_editing = Command::new(
        native::TOGGLE_PREVIEW_EDITING,
        if edit_in_preview {
            "Disable Editing in Preview"
        } else {
            "Enable Editing in Preview"
        },
        "view",
        "",
        "edit rendered editable read-only toggle",
    )
    .setting(false);
    let zoom = [
        Command::new(
            ZOOM_IN,
            "Zoom",
            "add-01",
            "⌘+",
            "settings interface scale larger",
        )
        .setting_value("In", false),
        Command::new(
            ZOOM_OUT,
            "Zoom",
            "remove-01",
            "⌘−",
            "settings interface scale smaller",
        )
        .setting_value("Out", false),
        Command::new(
            RESET_ZOOM,
            "Zoom",
            "arrow-horizontal",
            "",
            "settings interface scale 100% default",
        )
        .setting_value("Reset", false),
    ];
    themes
        .into_iter()
        .chain(widths)
        .chain([preview_editing])
        .chain(zoom)
}

pub(crate) fn width_for_id(id: &str) -> Option<PageWidth> {
    WIDTHS
        .iter()
        .find_map(|&(width, option_id, _)| (option_id == id).then_some(width))
}
