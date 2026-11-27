//! Search and keyboard selection for xuy’s command bar.

use dioxus::prelude::*;

use crate::{modal::run_dialog, notebook_menu};

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Command {
    pub(crate) id: &'static str,
    label: &'static str,
    icon: &'static str,
    shortcut: &'static str,
    keywords: &'static str,
}

impl Command {
    pub(crate) const fn new(
        id: &'static str,
        label: &'static str,
        icon: &'static str,
        shortcut: &'static str,
        keywords: &'static str,
    ) -> Self {
        Self {
            id,
            label,
            icon,
            shortcut,
            keywords,
        }
    }
}

pub(crate) fn notebook_commands(in_notebook: bool) -> Vec<Command> {
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
    }
    commands
}

fn matching_commands(commands: &[Command], query: &str) -> Vec<Command> {
    let query = query.to_lowercase();
    commands
        .iter()
        .copied()
        .filter(|command| {
            let label = command.label.to_lowercase();
            query
                .split_whitespace()
                .all(|word| label.contains(word) || command.keywords.contains(word))
        })
        .collect()
}

pub(crate) fn close() {
    document::eval("document.getElementById('command-bar')?.close()");
}

#[component]
pub(crate) fn CommandBar(
    commands: Vec<Command>,
    on_close: EventHandler<Option<&'static str>>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let mut chosen = use_signal(|| None);
    let icons = asset!("/assets/hugeicons", AssetOptions::folder());
    let options = matching_commands(&commands, &query.read());
    let selected = options.get(active()).copied();
    let mut choose = move |command: Command| {
        chosen.set(Some(command.id));
        close();
    };
    let on_key = move |event: KeyboardEvent| {
        let key = event.key();
        if event.is_composing() || !matches!(key, Key::ArrowUp | Key::ArrowDown | Key::Enter) {
            return;
        }
        let keyboard_options = matching_commands(&commands, &query.peek());
        if keyboard_options.is_empty() {
            return;
        }
        let count = keyboard_options.len();
        let index = (*active.peek()).min(count - 1);
        let next = match key {
            Key::ArrowDown => (index + 1) % count,
            Key::ArrowUp => (index + count - 1) % count,
            Key::Enter if !event.is_auto_repeating() => {
                event.prevent_default();
                choose(keyboard_options[index]);
                return;
            }
            _ => return,
        };
        event.prevent_default();
        active.set(next);
        let id = keyboard_options[next].id;
        document::eval(&format!(
            "document.getElementById('command-{id}').scrollIntoView({{block: 'nearest'}})"
        ));
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/command-bar.css") }
        div { id: "command-scrim", aria_hidden: true, onclick: move |_| close() }
        dialog { id: "command-bar", aria_label: "Command bar",
            onmounted: move |_| async move {
                run_dialog("command-bar", include_str!("command_bar_dialog.js")).await;
                on_close.call(*chosen.peek());
            },
            div { class: "command-search",
                img { src: "{icons}/search-01.svg", alt: "", width: "18", height: "18" }
                input {
                    aria_label: "Search commands",
                    autofocus: true, autocomplete: "off", spellcheck: false,
                    placeholder: "Search commands…", value: "{query}",
                    oninput: move |event| {
                        query.set(event.value());
                        active.set(0);
                    },
                    onkeydown: on_key,
                }
                form { method: "dialog",
                    button { r#type: "submit", aria_label: "Close command bar",
                        img { src: "{icons}/cancel-01.svg", alt: "", width: "16", height: "16" }
                    }
                }
            }
            ul { id: "command-options", aria_label: "Commands",
                for command in options {
                    li { key: "{command.id}",
                        button { r#type: "button", id: "command-{command.id}",
                            "data-selected": selected == Some(command),
                            onclick: move |_| choose(command),
                            img { src: "{icons}/{command.icon}.svg", alt: "", width: "18", height: "18" }
                            span { "{command.label}" }
                            if !command.shortcut.is_empty() {
                                kbd { aria_hidden: true, "{command.shortcut}" }
                            }
                        }
                    }
                }
            }
            if let Some(command) = selected {
                span { class: "command-announcement", role: "status", "{command.label}" }
            } else {
                p { class: "command-empty", role: "status", "No matching commands." }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_by_words_and_aliases_in_the_current_workspace() {
        let home = notebook_commands(false);
        let editing = notebook_commands(true);
        assert_eq!(
            matching_commands(&home, " NEW  NOTEBOOK ")[0].id,
            notebook_menu::NEW_NOTEBOOK
        );
        assert!(matching_commands(&home, "go home").is_empty());
        assert_eq!(
            matching_commands(&editing, "landing")[0].id,
            notebook_menu::HOME
        );
        assert!(matching_commands(&editing, "unrelated").is_empty());
    }
}
