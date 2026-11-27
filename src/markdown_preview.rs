//! Render a page off the UI thread and handle notebook links in the preview.

use dioxus::prelude::{dioxus_core::use_drop, *};
use tokio::time::{Duration, sleep};

use crate::editor::{render_editable_preview, render_preview};
use crate::markdown_paths::MarkdownPaths;
use crate::preview_edit::{EditableText, PreviewEdits};

pub(crate) type PreviewTarget = (String, bool);

struct RenderedPreview {
    html: String,
    editable: Option<(String, Vec<EditableText>)>,
    generation: u64,
}

#[component]
pub(crate) fn MarkdownPreview(
    draft: Signal<String>,
    paths: MarkdownPaths,
    editable: bool,
    busy: bool,
    mut anchor: Signal<Option<String>>,
    on_open: EventHandler<PreviewTarget>,
    on_edit: EventHandler<String>,
) -> Element {
    let mut edits = use_signal(|| None::<PreviewEdits>);
    let mut focused = use_signal(|| false);
    let mut generation = use_signal(|| 0u64);
    let mut edit_bridge = use_signal(|| None::<document::Eval>);
    let mut feedback = use_signal(|| None::<&'static str>);
    let rendered = use_resource(move || {
        let current = draft.read();
        // Keep the native caret and undo stack while editing this version of the note.
        let defer = focused()
            && edits
                .peek()
                .as_ref()
                .is_some_and(|edit| edit.draft == *current);
        let paths = paths.clone();
        async move {
            if defer {
                std::future::pending::<()>().await;
            }
            sleep(Duration::from_millis(120)).await;
            let source = draft.peek().clone();
            let revision = *generation.peek() + 1;
            generation.set(revision);
            tokio::task::spawn_blocking(move || {
                let (html, editable) = if editable {
                    let (html, runs) = render_editable_preview(&source, &paths);
                    (html, Some((source, runs)))
                } else {
                    (render_preview(&source, Some(&paths)), None)
                };
                RenderedPreview {
                    html,
                    editable,
                    generation: revision,
                }
            })
            .await
        }
    });
    let render_state = rendered.state();
    let mut bridge = use_signal(|| None::<document::Eval>);
    use_drop(move || {
        if let Some(eval) = *bridge.peek() {
            let _ = eval.send(None::<String>);
        }
        if let Some(eval) = *edit_bridge.peek() {
            let _ = eval.send(());
        }
    });
    use_effect(move || {
        if render_state() == UseResourceState::Ready
            && matches!(&*rendered.read(), Some(Ok(_)))
            && let (Some(eval), Some(fragment)) = (bridge(), anchor())
        {
            if let Err(error) = eval.send(fragment) {
                eprintln!("Could not scroll to Markdown section: {error}");
            }
            anchor.set(None);
        }
    });
    let receive_edit = move |(kind, revision, index, value): (String, u64, usize, String)| {
        if kind == "blur" {
            focused.set(false);
            return;
        }
        let latest = rendered.peek();
        let Some(Ok(preview)) = latest.as_ref() else {
            return;
        };
        let Some((original, runs)) = preview.editable.as_ref() else {
            return;
        };
        if preview.generation != revision {
            if kind == "input" {
                feedback.set(Some("The note changed. Try editing the refreshed preview."));
            }
            return;
        }
        let mut active = edits.write();
        if active
            .as_ref()
            .is_none_or(|edit| edit.generation != revision)
        {
            *active = Some(PreviewEdits::new(revision, original, runs));
        }
        let edit = active
            .as_mut()
            .expect("an edit buffer is initialized for this preview");
        if edit.draft != *draft.peek() {
            focused.set(false);
            feedback.set(Some("The note changed. Try editing the refreshed preview."));
            return;
        }
        focused.set(true);
        if kind == "input" {
            match edit.replace(original, runs, index, &value) {
                Ok(true) => {
                    let updated = edit.draft.clone();
                    drop(active);
                    on_edit.call(updated);
                    feedback.set(None);
                }
                Ok(false) => {}
                Err(error) => {
                    feedback.set(Some(error));
                    focused.set(false);
                }
            }
        }
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/markdown-preview.css") }
        article {
            class: "markdown-preview",
            aria_label: if editable { "Editable Markdown preview" } else { "Markdown preview" },
            tabindex: "0",
            "inert": (editable && busy).then_some(""),
            onmounted: move |event| async move {
                if editable { let _ = event.set_focus(true).await; }
            },
            if let Some(message) = feedback() {
                p { role: "alert", "{message}" }
            }
            match &*rendered.read_unchecked() {
                Some(Ok(preview)) => rsx! {
                    div {
                        id: "markdown-preview-content",
                        "data-preview-generation": "{preview.generation}",
                        dangerous_inner_html: "{preview.html}",
                        onmounted: move |_| async move {
                            if editable {
                                let mut receive_edit = receive_edit;
                                spawn(async move {
                                    let mut eval = document::eval(include_str!("markdown_preview_edit.js"));
                                    if let Some(previous) = edit_bridge.write().replace(eval) {
                                        let _ = previous.send(());
                                    }
                                    while let Ok(change) = eval.recv().await { receive_edit(change); }
                                });
                            }
                            let mut eval = document::eval(include_str!("markdown_preview_links.js"));
                            if let Some(previous) = bridge.write().replace(eval) {
                                let _ = previous.send(None::<String>);
                            }
                            while let Ok(target) = eval.recv::<PreviewTarget>().await {
                                on_open.call(target);
                            }
                        },
                    }
                },
                Some(Err(_)) => rsx! {
                    p { role: "alert", "Could not render the preview." }
                },
                None => rsx! { p { role: "status", "Rendering preview…" } },
            }
        }
    }
}
