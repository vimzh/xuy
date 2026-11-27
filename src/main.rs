mod appearance;
mod chrome;
mod clock;
mod command_bar;
mod commands;
mod cursor;
mod dropdown;
mod editor;
mod external_edit;
mod font_size;
mod guide;
mod highlight;
mod markdown;
mod markdown_paths;
mod markdown_preview;
mod modal;
mod note_history;
mod note_status;
mod notebook;
mod notebook_index;
mod notebook_menu;
mod notebook_picker;
mod page_width;
mod performance;
mod preview_edit;
mod settings;
mod terminal;
mod theme;
mod ui;
mod workspace;
mod workspace_session;
mod workspace_sidebar;
mod writing_sidebar;
mod writing_style;
mod zoom;

use dioxus::desktop::{Config, WindowBuilder, tao::dpi::LogicalSize, use_muda_event_handler};
use dioxus::prelude::*;
use font_size::FontPreference;
use std::path::PathBuf;

use crate::ui::Button;

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
        .with_min_inner_size(LogicalSize::new(440, 320))
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
    let (mut appearance, dark) = theme::use_appearance();
    let scenes = use_signal(appearance::ScenePreferences::load);
    let (zoom, set_zoom) = zoom::use_zoom_shortcuts();
    let scale = f64::from(zoom()) / 100.0;
    let mut opened = use_signal(|| None::<workspace::OpenNotebook>);
    let mut pending = use_signal(|| None::<workspace::OpenNotebook>);
    let mut palette_command = use_signal(|| None::<&'static str>);
    let mut page_actions = use_signal(commands::PageActions::default);
    let mut terminal_state = use_signal(terminal::State::default);
    let mut command_open = use_signal(|| false);
    let busy = use_signal(|| false);
    let mut page_width = use_signal(page_width::PageWidth::default);
    let mut edit_in_preview = use_signal(notebook_menu::load_preview_editing);
    let cursor = cursor::use_cursor_options();
    let settings_font = use_signal(|| FontPreference::Settings.load());
    let markdown_font = use_signal(|| FontPreference::Markdown.load());
    let writing_style = use_signal(writing_style::WritingStyle::load);
    let mut settings_open = use_signal(|| false);
    let mut guide_open = use_signal(|| false);
    let mut picker_open = use_signal(|| false);
    let saved = use_signal(notebook_picker::load);
    let mut show_performance = use_signal(|| false);
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
    let modal_open = use_memo(move || {
        settings_open() || guide_open() || picker_open() || opening() || command_open()
    });
    let commands_blocked = move || {
        *settings_open.peek()
            || *guide_open.peek()
            || *picker_open.peek()
            || *opening.peek()
            || pending.peek().is_some()
            || *busy.peek()
    };
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
        if *busy.peek()
            || pending.peek().is_some()
            || *opening.peek()
            || *settings_open.peek()
            || *guide_open.peek()
            || *picker_open.peek() != add
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
        notebook_menu::SETTINGS => settings_open.set(true),
        notebook_menu::DOCS => guide_open.set(true),
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
        if id == notebook_menu::TOGGLE_PERFORMANCE {
            show_performance.toggle();
        } else if id == notebook_menu::COMMAND_BAR && *command_open.peek() {
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
            } else if !writing_style::apply_style_command(id, writing_style)
                && !FontPreference::Settings.apply_command(id, settings_font)
                && !FontPreference::Markdown.apply_command(id, markdown_font)
                && !cursor::apply_cursor_command(id, cursor)
                && !appearance::apply_scene_command(id, *dark.peek(), scenes)
            {
                let in_notebook = opened.peek().is_some();
                match id {
                    commands::ZOOM_IN => set_zoom.call(zoom::ZoomAction::In),
                    commands::ZOOM_OUT => set_zoom.call(zoom::ZoomAction::Out),
                    commands::RESET_ZOOM => set_zoom.call(zoom::ZoomAction::Reset),
                    terminal::OPEN if in_notebook => terminal_state.set(terminal::State::Visible),
                    terminal::MINIMIZE if in_notebook => {
                        terminal_state.set(terminal::State::Minimized)
                    }
                    terminal::CLOSE if in_notebook => terminal_state.set(terminal::State::Closed),
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
        ("notebooks", "P", "notebook-01"),
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
                opened: notebook, page_width, cursor, show_performance: show_performance(), zoom: scale,
                edit_in_preview: edit_in_preview(),
                markdown_font: markdown_font(), writing_style: writing_style(),
                busy, pending, error, modal_open, palette_command, page_actions, terminal_state,
                on_switch: move |notebook| {
                    terminal_state.set(terminal::State::Closed);
                    page_actions.set(commands::PageActions::default());
                    opened.set(Some(notebook));
                },
                on_home: move |_| {
                    terminal_state.set(terminal::State::Closed);
                    workspace_session::clear();
                    error.set(None);
                    page_actions.set(commands::PageActions::default());
                    opened.set(None);
                },
            }
        } else {
            appearance::LandingScene { dark: dark(), preferences: scenes() }
            div { class: "home-readouts",
                clock::StarterClock {}
                if show_performance() {
                    performance::PerformancePanel {}
                }
            }
            main { class: "landing", aria_label: "xuy", aria_busy: opening(),
                nav { aria_label: "Notebook actions",
                    for (label, shortcut, icon) in actions {
                        Button { key: "{label}",
                            aria_keyshortcuts: "Meta+{shortcut}",
                            disabled: opening(),
                            onclick: move |_| async move {
                                match label {
                                    "notebooks" => run_app_command(notebook_menu::NOTEBOOKS),
                                    "settings" => settings_open.set(true),
                                    "docs" => guide_open.set(true),
                                    "new" => choose(ChooseFolder::NewNote).await,
                                    _ => choose(ChooseFolder::Open).await,
                                }
                            },
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
                    if opened.read().is_some() { commands.extend(commands::terminal_commands(terminal_state())); }
                    commands.extend([
                        command_bar::Command::new(notebook_menu::SETTINGS, "Settings…", "settings-01", "⌘,", "preferences"),
                        command_bar::Command::new(notebook_menu::DOCS, "Using xuy", "book-open-01", "⌘D", "help docs guide"),
                    ]);
                    commands.extend(commands::settings(appearance(), page_width(), edit_in_preview()));
                    commands.extend(writing_style::style_commands(writing_style()));
                    commands.extend(FontPreference::Settings.commands(settings_font()));
                    commands.extend(FontPreference::Markdown.commands(markdown_font()));
                    commands.extend(cursor::cursor_commands(cursor().unwrap_or_default()));
                    commands.extend(appearance::scene_commands(dark(), scenes()));
                    commands
                },
                on_close: palette_closed,
            }
        }
        if settings_open() {
            settings::Settings {
                cursor, page_width, appearance, dark, scenes, settings_font, markdown_font, writing_style,
                on_close: move |_| settings_open.set(false),
            }
        }
        if guide_open() {
            guide::UserGuide { on_close: move |_| guide_open.set(false) }
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
