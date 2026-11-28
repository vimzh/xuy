//! Render a page off the UI thread and handle notebook links in the preview.

use dioxus::prelude::{dioxus_core::use_drop, *};
use tokio::time::{Duration, sleep};

use crate::markdown::RenderedNote;
use crate::markdown::edits::PreviewEdits;
use crate::markdown::paths::MarkdownPaths;

use super::render_note_preview;

pub(crate) type PreviewTarget = (String, bool);

struct RenderedPreview {
    note: RenderedNote,
    source: String,
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
            tokio::task::spawn_blocking(move || RenderedPreview {
                note: render_note_preview(&source, &paths, editable),
                source,
                generation: revision,
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
            return None;
        }
        let latest = rendered.peek();
        let preview = latest.as_ref()?.as_ref().ok()?;
        if preview.generation != revision {
            if kind != "focus" {
                feedback.set(Some("The note changed. Try editing the refreshed preview."));
            }
            return None;
        }
        let mut active = edits.write();
        if active
            .as_ref()
            .is_none_or(|edit| edit.generation != revision)
        {
            *active = Some(PreviewEdits::new(
                revision,
                &preview.source,
                &preview.note.text,
                &preview.note.tasks,
            ));
        }
        let edit = active
            .as_mut()
            .expect("an edit buffer is initialized for this preview");
        if edit.draft != *draft.peek() {
            focused.set(false);
            feedback.set(Some("The note changed. Try editing the refreshed preview."));
            return None;
        }
        let result = match kind.as_str() {
            "focus" => {
                focused.set(true);
                return None;
            }
            "input" if editable => {
                focused.set(true);
                edit.replace(&preview.source, &preview.note.text, index, &value)
                    .map(|changed| (changed, None))
            }
            "task" => edit
                .toggle_task(index)
                .map(|checked| (true, Some((revision, index, checked)))),
            _ => return None,
        };
        match result {
            Ok((true, reply)) => {
                let updated = edit.draft.clone();
                drop(active);
                on_edit.call(updated);
                feedback.set(None);
                reply
            }
            Ok((false, _)) => None,
            Err(error) => {
                feedback.set(Some(error));
                focused.set(false);
                None
            }
        }
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/markdown-preview.css") }
        article {
            class: "markdown-preview",
            aria_label: if editable { "Editable Markdown preview" } else { "Markdown preview" },
            tabindex: "0",
            "inert": busy.then_some(""),
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
                        dangerous_inner_html: "{preview.note.html}",
                        onmounted: move |_| async move {
                            let mut receive_edit = receive_edit;
                            spawn(async move {
                                let mut eval = document::eval(include_str!("markdown_preview_edit.js"));
                                if let Some(previous) = edit_bridge.write().replace(eval) {
                                    let _ = previous.send(());
                                }
                                while let Ok(change) = eval.recv().await {
                                    if let Some(reply) = receive_edit(change)
                                        && let Err(error) = eval.send(reply) {
                                        eprintln!("Could not update the rendered checklist: {error}");
                                    }
                                }
                            });
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
