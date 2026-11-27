use crate::markdown;
use crate::markdown_paths::MarkdownPaths;
use crate::markdown_preview::{MarkdownPreview, PreviewTarget};
use dioxus::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ViewMode {
    #[default]
    Edit,
    Preview,
    Split,
}

impl ViewMode {
    pub(crate) const OPTIONS: [(Self, &'static str, &'static str); 3] = [
        (Self::Edit, "edit", "Edit"),
        (Self::Preview, "preview", "Preview"),
        (Self::Split, "split", "Side by Side"),
    ];

    pub(crate) fn toggle_preview(self) -> Self {
        match self {
            Self::Edit => Self::Preview,
            Self::Preview | Self::Split => Self::Edit,
        }
    }

    pub(crate) fn toggle_split(self) -> Self {
        match self {
            Self::Split => Self::Edit,
            Self::Edit | Self::Preview => Self::Split,
        }
    }

    pub(crate) fn preview_action(self) -> &'static str {
        match self {
            Self::Edit => "Show Preview",
            Self::Preview | Self::Split => "Edit Source",
        }
    }
}

#[component]
pub(crate) fn Editor(
    draft: Signal<String>,
    dirty: Memo<bool>,
    view_mode: Signal<ViewMode>,
    edit_in_preview: bool,
    paths: MarkdownPaths,
    preview_anchor: Signal<Option<String>>,
    on_preview_open: EventHandler<PreviewTarget>,
    mut chrome_hidden: Signal<bool>,
    busy: bool,
    on_edit: EventHandler<String>,
    on_save: EventHandler<()>,
) -> Element {
    let mode = view_mode();
    let editable = edit_in_preview && mode == ViewMode::Preview;
    use_effect(move || match view_mode() {
        ViewMode::Edit | ViewMode::Split => {
            document::eval("document.getElementById('note-source')?.focus({preventScroll: true})");
        }
        ViewMode::Preview => {
            document::eval(
                "document.querySelector('.markdown-preview')?.focus({preventScroll: true})",
            );
        }
    });
    rsx! {
        section {
            class: "note-editor",
            aria_label: "Markdown editor",
            aria_keyshortcuts: "Meta+Shift+P",
            "data-split": mode == ViewMode::Split,
            onkeydown: move |event| {
                if event.modifiers() == Modifiers::META
                    && !event.is_composing()
                    && matches!(event.key(), Key::Character(key) if key.eq_ignore_ascii_case("s"))
                {
                    event.prevent_default();
                    if !busy && !event.is_auto_repeating() && dirty() {
                        on_save.call(());
                    }
                }
            },
            span { class: "note-status", role: "status",
                if dirty() { "Unsaved changes" }
            }
            textarea {
                id: "note-source",
                class: "note-input",
                aria_label: "Markdown source",
                aria_keyshortcuts: "Meta+S",
                hidden: mode == ViewMode::Preview,
                readonly: busy,
                value: "{draft}",
                oninput: move |event| {
                    on_edit.call(event.value());
                    if !*chrome_hidden.peek() {
                        chrome_hidden.set(true);
                    }
                },
            }
            if mode != ViewMode::Edit {
                MarkdownPreview {
                        key: "{editable}", draft, paths, editable, busy, anchor: preview_anchor,
                        on_open: on_preview_open,
                        on_edit: move |text| {
                            on_edit.call(text);
                            if !*chrome_hidden.peek() { chrome_hidden.set(true); }
                        },
                    }
            }
        }
    }
}

pub(crate) fn render_preview(source: &str, paths: Option<&MarkdownPaths>) -> String {
    markdown::render(source, paths, |_| false)
}

pub(crate) fn render_editable_preview(
    source: &str,
    paths: &MarkdownPaths,
) -> (String, Vec<crate::preview_edit::EditableText>) {
    markdown::render_editable(source, Some(paths), |_| false)
}
