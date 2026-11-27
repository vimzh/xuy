mod chrome;
mod command_bar;
mod commands;
mod dropdown;
mod editor;
mod markdown;
mod markdown_paths;
mod markdown_preview;
mod modal;
mod note_history;
mod notebook;
mod notebook_index;
mod notebook_menu;
mod notebook_picker;
mod page_width;
mod preview_edit;
mod theme;
mod workspace;
mod workspace_session;
mod workspace_sidebar;
mod writing_style;
mod zoom;

use dioxus::desktop::{Config, WindowBuilder, use_muda_event_handler};
use dioxus::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChooseFolder {
    Open,
    NewNote,
    NewNotebook,
    AddNotebook,
}

fn main() {
    let appearance = theme::Appearance::load();
    let window = WindowBuilder::new()
        .with_title("xuy")
        .with_theme(appearance.native());

    LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_menu(notebook_menu::build(appearance)),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    let mut appearance = theme::use_appearance();
    let (zoom, set_zoom) = zoom::use_zoom_shortcuts();
    let scale = f64::from(zoom()) / 100.0;
    let mut opened = use_signal(|| None::<workspace::OpenNotebook>);
    let mut pending = use_signal(|| None::<workspace::OpenNotebook>);
    let mut palette_command = use_signal(|| None::<&'static str>);
    let mut page_actions = use_signal(commands::PageActions::default);
    let mut command_open = use_signal(|| false);
    let writing_style = use_signal(writing_style::WritingStyle::load);
    let busy = use_signal(|| false);
    let mut page_width = use_signal(page_width::PageWidth::default);
    let mut edit_in_preview = use_signal(notebook_menu::load_preview_editing);
    let mut picker_open = use_signal(|| false);
    let saved = use_signal(notebook_picker::load);
    let mut opening = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    use_future(move || async move {
        let restored = match workspace_session::load() {
            Ok(Some(session)) => workspace::OpenNotebook::restore(session).await.map(Some),
            Ok(None) => Ok(None),
            Err(failure) => Err(failure),
        };
        match restored {
            Ok(Some((notebook, warning))) => {
                opened.set(Some(notebook));
                error.set(warning);
            }
            Ok(None) => {}
            Err(failure) => error.set(Some(format!(
                "Could not restore the last notebook. {failure}"
            ))),
        }
        opening.set(false);
    });
    let icons = asset!("/assets/hugeicons", AssetOptions::folder());
    let mono = asset!("/assets/ia-writer-mono", AssetOptions::folder());
    let writing_fonts = asset!("/assets/writing-fonts", AssetOptions::folder());
    let modal_open = use_memo(move || picker_open() || opening() || command_open());
    let commands_blocked =
        move || *picker_open.peek() || *opening.peek() || pending.peek().is_some() || *busy.peek();
    let mut activate = move |notebook: workspace::OpenNotebook| {
        let same_folder = opened
            .peek()
            .as_ref()
            .map(|current| current.root() == notebook.root());
        match same_folder {
            Some(false) => pending.set(Some(notebook)),
            Some(true) => {}
            None => opened.set(Some(notebook)),
        }
    };
    let choose = move |action| async move {
        let add = action == ChooseFolder::AddNotebook;
        if *busy.peek() || pending.peek().is_some() || *opening.peek() || *picker_open.peek() != add
        {
            return;
        }
        let (title, start_new, can_create) = match action {
            ChooseFolder::Open => ("Open Markdown folder", false, true),
            ChooseFolder::NewNote => ("Choose a folder for the note", true, true),
            ChooseFolder::NewNotebook => ("Choose or create a notebook folder", false, true),
            ChooseFolder::AddNotebook => ("Choose an existing notebook folder", false, false),
        };
        opening.set(true);
        error.set(None);
        match workspace::pick_notebook(title, start_new, can_create).await {
            Ok(Some(notebook)) => {
                if matches!(
                    action,
                    ChooseFolder::NewNotebook | ChooseFolder::AddNotebook
                ) && let Err(failure) = notebook_picker::register(saved, notebook.root())
                {
                    error.set(Some(failure.to_string()));
                } else if !add {
                    activate(notebook);
                }
            }
            Ok(None) => {}
            Err(failure) => error.set(Some(failure.to_string())),
        }
        opening.set(false);
    };
    let open_saved = move |path: PathBuf| async move {
        if *opening.peek() || *busy.peek() || pending.peek().is_some() || !*picker_open.peek() {
            return;
        }
        opening.set(true);
        error.set(None);
        match workspace::OpenNotebook::open(path.clone(), false).await {
            Ok(notebook) if notebook.root() == path => {
                document::eval("document.getElementById('notebook-picker').close();");
                activate(notebook);
            }
            Ok(_) => error.set(Some(format!("{} now points to a different folder. Add the folder again to save its current location.", path.display()))),
            Err(failure) => error.set(Some(failure.to_string())),
        }
        opening.set(false);
    };
    let mut run_app_command = move |id: &str| match id {
        notebook_menu::TOGGLE_PREVIEW_EDITING => {
            let enabled = !*edit_in_preview.peek();
            notebook_menu::set_preview_editing(enabled);
            edit_in_preview.set(enabled);
        }
        notebook_menu::NOTEBOOKS => {
            error.set(None);
            picker_open.set(true);
        }
        notebook_menu::NEW_NOTE if opened.peek().is_none() => {
            spawn(choose(ChooseFolder::NewNote));
        }
        notebook_menu::NEW_NOTEBOOK => {
            spawn(choose(ChooseFolder::NewNotebook));
        }
        notebook_menu::OPEN_FOLDER => {
            spawn(choose(ChooseFolder::Open));
        }
        _ => {}
    };
    use_muda_event_handler(move |event| {
        let id = event.id().0.as_str();
        if id == notebook_menu::COMMAND_BAR && *command_open.peek() {
            command_bar::close();
        } else if id == notebook_menu::TOGGLE_PREVIEW_EDITING
            || (!commands_blocked() && !*command_open.peek())
        {
            if id == notebook_menu::COMMAND_BAR {
                command_open.set(true);
            } else {
                run_app_command(id);
            }
        }
    });
    let palette_closed = move |choice| {
        command_open.set(false);
        if let Some(id) = choice
            && !commands_blocked()
        {
            if let Some(option) = theme::appearance_for_id(id) {
                appearance.set(option);
            } else if let Some(width) = commands::width_for_id(id) {
                page_width.set(width);
            } else if !writing_style::apply_style_command(id, writing_style) {
                let in_notebook = opened.peek().is_some();
                match id {
                    commands::ZOOM_IN => set_zoom.call(zoom::ZoomAction::In),
                    commands::ZOOM_OUT => set_zoom.call(zoom::ZoomAction::Out),
                    commands::RESET_ZOOM => set_zoom.call(zoom::ZoomAction::Reset),
                    notebook_menu::NEW_NOTE
                    | notebook_menu::NEW_FOLDER
                    | notebook_menu::HOME
                    | notebook_menu::TOGGLE_SIDEBAR
                    | notebook_menu::TOGGLE_PREVIEW
                    | notebook_menu::TOGGLE_SPLIT
                    | notebook_menu::SAVE
                    | commands::PREVIOUS_PAGE
                    | commands::NEXT_PAGE
                    | commands::INSERT_CODE_BLOCK
                        if in_notebook =>
                    {
                        palette_command.set(Some(id))
                    }
                    _ => run_app_command(id),
                }
            }
        }
    };
    let actions = [
        ("new", "N", "add-01"),
        ("open", "O", "folder-open"),
        ("settings", ",", "settings-01"),
        ("docs", "D", "book-open-01"),
    ];

    rsx! {
        document::Stylesheet { href: asset!("/assets/app.css") }
        document::Stylesheet { href: "{mono}/fonts.css" }
        document::Stylesheet { href: "{writing_fonts}/fonts.css" }
        if let Some((key, notebook)) = opened().map(|notebook| (format!("{:?}", notebook.root()), notebook)) {
            workspace::Workspace {
                key: "{key}",
                opened: notebook, page_width, zoom: scale,
                edit_in_preview: edit_in_preview(),
                writing_style: writing_style(),
                busy, pending, error, modal_open, palette_command, page_actions,
                on_switch: move |notebook| {
                    page_actions.set(commands::PageActions::default());
                    opened.set(Some(notebook));
                },
                on_home: move |_| {
                    workspace_session::clear();
                    error.set(None);
                    page_actions.set(commands::PageActions::default());
                    opened.set(None);
                },
            }
        } else {
            main { class: "landing", aria_busy: opening(),
                pre { class: "black-hole", aria_hidden: true,
                    "data-license": asset!("/assets/ascii-rest/LICENSE.txt"),
                    {include_str!("../assets/ascii-rest/black-hole.txt")}
                }
                h1 { "xuy" }
                nav { aria_label: "Notebook actions",
                    for (label, shortcut, icon) in actions {
                        button { key: "{label}",
                            disabled: opening() || !matches!(label, "new" | "open"),
                            onclick: move |_| async move { choose(if label == "new" { ChooseFolder::NewNote } else { ChooseFolder::Open }).await },
                            img { src: "{icons}/{icon}.svg", alt: "", width: "16", height: "16" }
                            span { "{label}" }
                            kbd { aria_hidden: true,
                                span { class: "command-key", "⌘" }
                                span { "{shortcut}" }
                            }
                        }
                    }
                }
                if let Some(message) = error() {
                    p { role: "alert", "{message}" }
                }
                section { aria_labelledby: "recent-heading",
                    h2 { id: "recent-heading", "Recently Opened" }
                }
            }
        }
        if command_open() {
            command_bar::CommandBar {
                commands: {
                    let mut commands = commands::notebook_commands(opened.read().is_some(), page_actions());
                    commands.extend(commands::settings(appearance(), page_width(), edit_in_preview()));
                    commands.extend(writing_style::style_commands(writing_style()));
                    commands
                },
                on_close: palette_closed,
            }
        }
        if picker_open() {
            notebook_picker::NotebookPicker {
                saved, busy: opening(), error: error(), on_open: open_saved,
                on_add: move |_| choose(ChooseFolder::AddNotebook),
                on_close: move |_| {
                    picker_open.set(false);
                    error.set(None);
                },
            }
        }
    }
}
