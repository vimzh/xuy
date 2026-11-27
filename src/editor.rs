use crate::markdown;
use crate::markdown_paths::MarkdownPaths;
use crate::markdown_preview::{MarkdownPreview, PreviewTarget};
use dioxus::prelude::*;

#[component]
pub(crate) fn Editor(
    draft: Signal<String>,
    dirty: Memo<bool>,
    preview: Signal<bool>,
    paths: MarkdownPaths,
    preview_anchor: Signal<Option<String>>,
    on_preview_open: EventHandler<PreviewTarget>,
    mut chrome_hidden: Signal<bool>,
    busy: bool,
    on_edit: EventHandler<String>,
    on_save: EventHandler<()>,
) -> Element {
    rsx! {
        section {
            class: "note-editor",
            aria_label: "Markdown editor",
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
            if preview() {
                MarkdownPreview { draft, paths, anchor: preview_anchor, on_open: on_preview_open }
            } else {
                textarea {
                    class: "note-input",
                    aria_label: "Markdown source",
                    aria_keyshortcuts: "Meta+S",
                    readonly: busy,
                    value: "{draft}",
                    onmounted: move |event| async move { let _ = event.set_focus(true).await; },
                    oninput: move |event| {
                        on_edit.call(event.value());
                        if !*chrome_hidden.peek() {
                            chrome_hidden.set(true);
                        }
                    },
                }
            }
        }
    }
}

pub(crate) fn render_preview(source: &str, paths: Option<&MarkdownPaths>) -> String {
    markdown::render(source, paths, |_| false)
}
