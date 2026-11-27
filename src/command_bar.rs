//! Search and keyboard selection for xuy’s command bar.

use dioxus::prelude::*;

use crate::ui::{Button, ButtonKind, Dialog, DialogScrim, close_dialog};

const DIALOG_ID: &str = "command-bar";

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Command {
    pub(crate) id: &'static str,
    label: &'static str,
    value: &'static str,
    icon: &'static str,
    shortcut: &'static str,
    keywords: &'static str,
    setting: bool,
    current: bool,
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
            value: "",
            icon,
            shortcut,
            keywords,
            setting: false,
            current: false,
        }
    }

    pub(crate) const fn setting(mut self, current: bool) -> Self {
        self.setting = true;
        self.current = current;
        self
    }

    pub(crate) const fn setting_value(mut self, value: &'static str, current: bool) -> Self {
        self.value = value;
        self.setting(current)
    }

    fn title(self) -> String {
        if self.value.is_empty() {
            self.label.to_owned()
        } else {
            format!("{}: {}", self.label, self.value)
        }
    }
}

fn matching_commands(commands: &[Command], query: &str) -> Vec<Command> {
    let query = query.to_lowercase().replace(':', " ");
    let mut matches: Vec<_> = commands
        .iter()
        .copied()
        .filter(|command| {
            let label = command.label.to_lowercase();
            let value = command.value.to_lowercase();
            query.split_whitespace().all(|word| {
                label.contains(word) || value.contains(word) || command.keywords.contains(word)
            })
        })
        .collect();
    matches.sort_by_key(|command| (command.setting, command.label));
    matches
}

pub(crate) fn close() {
    close_dialog(DIALOG_ID);
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
    let selected = options
        .get(active().min(options.len().saturating_sub(1)))
        .copied();
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
        Dialog { id: DIALOG_ID, aria_label: "Command bar",
            scrim: DialogScrim { id: "command-scrim", focusable: "input, button" },
            on_close: move |_| on_close.call(*chosen.peek()),
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
                    Button { kind: ButtonKind::Submit, aria_label: "Close command bar",
                        img { src: "{icons}/cancel-01.svg", alt: "", width: "16", height: "16" }
                    }
                }
            }
            ul { id: "command-options", aria_label: "Commands",
                for (index, &command) in options.iter().enumerate() {
                    li { key: "{command.id}",
                        if index == 0 || options[index - 1].setting != command.setting {
                            div { class: "command-section",
                                if command.setting { "Settings" } else { "Actions" }
                            }
                        }
                        Button { id: "command-{command.id}",
                            "data-selected": selected == Some(command),
                            aria_current: command.current.then_some("true"),
                            onclick: move |_| choose(command),
                            img { src: "{icons}/{command.icon}.svg", alt: "", width: "18", height: "18" }
                            span {
                                "{command.label}"
                                if !command.value.is_empty() {
                                    ": "
                                    span { class: "command-value", "{command.value}" }
                                }
                            }
                            if command.current {
                                span { class: "command-current", aria_hidden: true, "✓" }
                            } else if !command.shortcut.is_empty() {
                                kbd { aria_hidden: true, "{command.shortcut}" }
                            }
                        }
                    }
                }
            }
            if let Some(command) = selected {
                span { class: "command-announcement", role: "status",
                    "{command.title()}"
                    if command.current { ", current setting" }
                }
            } else {
                p { class: "command-empty", role: "status", "No matching commands." }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands, editor::ViewMode, notebook_menu};

    #[test]
    fn filters_by_words_and_aliases_in_the_current_workspace() {
        let home = commands::notebook_commands(false, commands::PageActions::default());
        let editing = commands::notebook_commands(true, commands::PageActions::default());
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

    #[test]
    fn exposes_only_page_actions_that_can_run() {
        let blank_notebook = commands::notebook_commands(true, commands::PageActions::default());
        assert!(matching_commands(&blank_notebook, "preview").is_empty());
        assert!(matching_commands(&blank_notebook, "side by side").is_empty());
        assert!(matching_commands(&blank_notebook, "save page").is_empty());

        let previewing = commands::notebook_commands(
            true,
            commands::PageActions {
                has_page: true,
                view_mode: ViewMode::Preview,
                dirty: true,
                can_back: true,
                can_forward: false,
            },
        );
        assert_eq!(
            matching_commands(&previewing, "edit source")[0].id,
            notebook_menu::TOGGLE_PREVIEW
        );
        assert_eq!(
            matching_commands(&previewing, "save page")[0].id,
            notebook_menu::SAVE
        );
        assert_eq!(
            matching_commands(&previewing, "go back")[0].id,
            commands::PREVIOUS_PAGE
        );
        assert!(matching_commands(&previewing, "next page").is_empty());
    }

    #[test]
    fn switches_between_the_three_note_views_from_the_palette() {
        use ViewMode::{Edit, Preview, Split};

        let mut mode = Edit;
        for (query, target, shortcut, expected) in [
            (
                "show preview",
                notebook_menu::TOGGLE_PREVIEW,
                "⇧⌘P",
                Preview,
            ),
            ("edit source", notebook_menu::TOGGLE_PREVIEW, "⇧⌘P", Edit),
            ("open side by side", notebook_menu::TOGGLE_SPLIT, "", Split),
            ("edit source", notebook_menu::TOGGLE_PREVIEW, "⇧⌘P", Edit),
            ("split view", notebook_menu::TOGGLE_SPLIT, "", Split),
            ("close side by side", notebook_menu::TOGGLE_SPLIT, "", Edit),
        ] {
            let options = commands::notebook_commands(
                true,
                commands::PageActions {
                    has_page: true,
                    view_mode: mode,
                    ..Default::default()
                },
            );
            let choices = matching_commands(&options, query);
            assert_eq!(choices.len(), 1, "{query}");
            assert_eq!(choices[0].id, target);
            assert_eq!(choices[0].shortcut, shortcut);
            mode = match target {
                notebook_menu::TOGGLE_PREVIEW => mode.toggle_preview(),
                notebook_menu::TOGGLE_SPLIT => mode.toggle_split(),
                _ => unreachable!(),
            };
            assert!(mode == expected, "{query}");
        }
    }

    #[test]
    fn searches_settings_and_marks_the_active_values() {
        use crate::{page_width::PageWidth, theme::Appearance};

        let settings: Vec<_> =
            commands::settings(Appearance::Dark, PageWidth::Wide, false).collect();
        let dark = matching_commands(&settings, "appearance dark")[0];
        assert!(dark.current);
        assert!(crate::theme::appearance_for_id(dark.id) == Some(Appearance::Dark));
        let wide = matching_commands(&settings, "editor wide")[0];
        assert!(wide.current);
        assert!(commands::width_for_id(wide.id) == Some(PageWidth::Wide));
        assert!(!matching_commands(&settings, "standard width")[0].current);
        assert_eq!(
            matching_commands(&settings, "reset zoom")[0].id,
            commands::RESET_ZOOM
        );
    }

    #[test]
    fn finds_writing_settings_by_name_and_value() {
        use crate::{font_size::FontPreference, writing_style::WritingStyle};

        let mut settings: Vec<_> =
            crate::writing_style::style_commands(WritingStyle::load()).collect();
        settings.extend(FontPreference::Settings.commands(15));
        settings.extend(FontPreference::Markdown.commands(18));

        for (query, id) in [
            ("writing font jetbrains", "font-jetbrains"),
            ("relaxed line height", "spacing-relaxed"),
            ("code theme graphite", "code-theme-graphite"),
            ("Settings font size: 15", "settings-font-15"),
            ("Markdown font size:18", "markdown-font-18"),
        ] {
            let matches = matching_commands(&settings, query);
            assert_eq!(matches.len(), 1, "{query}");
            assert_eq!(matches[0].id, id, "{query}");
        }
        assert!(matching_commands(&settings, "settings font size 15")[0].current);
        assert!(matching_commands(&settings, "markdown font size 18")[0].current);
    }

    #[test]
    fn sorts_action_and_setting_labels_without_reordering_setting_choices() {
        let commands = [
            Command::new("z", "Zoom", "", "", "").setting_value("Reset", false),
            Command::new("b", "Save", "", "", ""),
            Command::new("a", "Open", "", "", ""),
            Command::new("size-9", "Size", "", "", "").setting_value("9 px", false),
            Command::new("size-12", "Size", "", "", "").setting_value("12 px", true),
        ];
        let results = matching_commands(&commands, "");
        let ids: Vec<_> = results.iter().map(|command| command.id).collect();
        assert_eq!(ids, ["a", "b", "size-9", "size-12", "z"]);
        assert_eq!(results[3].title(), "Size: 12 px");
    }

    #[test]
    fn only_offers_code_insertion_with_an_editable_source() {
        for (has_page, view_mode, can_insert) in [
            (false, ViewMode::Edit, false),
            (true, ViewMode::Edit, true),
            (true, ViewMode::Split, true),
            (true, ViewMode::Preview, false),
        ] {
            let commands = commands::notebook_commands(
                true,
                commands::PageActions {
                    has_page,
                    view_mode,
                    ..Default::default()
                },
            );
            assert_eq!(
                matching_commands(&commands, "insert code block")
                    .iter()
                    .any(|command| command.id == commands::INSERT_CODE_BLOCK),
                can_insert
            );
        }
    }

    #[test]
    fn preview_editing_command_reflects_current_state() {
        use crate::{page_width::PageWidth, theme::Appearance};

        for (enabled, label) in [
            (false, "Enable Editing in Preview"),
            (true, "Disable Editing in Preview"),
        ] {
            let settings: Vec<_> =
                commands::settings(Appearance::System, PageWidth::Standard, enabled).collect();
            let matching = matching_commands(&settings, "edit in preview");
            assert_eq!(matching.len(), 1);
            assert_eq!(matching[0].id, notebook_menu::TOGGLE_PREVIEW_EDITING);
            assert_eq!(matching[0].label, label);
        }
    }
}
