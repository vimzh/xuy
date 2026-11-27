//! Render a page off the UI thread and handle notebook links in the preview.

use dioxus::prelude::{dioxus_core::use_drop, *};

use crate::editor::render_preview;
use crate::markdown_paths::MarkdownPaths;

pub(crate) type PreviewTarget = (String, bool);

#[component]
pub(crate) fn MarkdownPreview(
    draft: Signal<String>,
    paths: MarkdownPaths,
    anchor: Signal<Option<String>>,
    on_open: EventHandler<PreviewTarget>,
) -> Element {
    let rendered = use_resource(move || {
        let paths = paths.clone();
        let source = draft();
        async move { tokio::task::spawn_blocking(move || render_preview(&source, Some(&paths))).await }
    });
    let mut bridge = use_signal(|| None::<document::Eval>);
    use_drop(move || {
        if let Some(eval) = *bridge.peek() {
            let _ = eval.send(None::<String>);
        }
    });
    use_effect(move || {
        if let (Some(eval), Some(fragment)) = (bridge(), anchor())
            && let Err(error) = eval.send(fragment)
        {
            eprintln!("Could not scroll to Markdown section: {error}");
        }
    });
    let render_state = rendered.state();

    rsx! {
        document::Stylesheet { href: asset!("/assets/markdown-preview.css") }
        article { class: "markdown-preview", aria_label: "Markdown preview", tabindex: "0",
            match (render_state(), &*rendered.read_unchecked()) {
                (UseResourceState::Ready, Some(Ok(html))) => rsx! {
                    div { id: "markdown-preview-content", dangerous_inner_html: "{html}",
                        onmounted: move |_| async move {
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
                (UseResourceState::Ready, Some(Err(_))) => rsx! {
                    p { role: "alert", "Could not render the preview." }
                },
                _ => rsx! { p { role: "status", "Rendering preview…" } },
            }
        }
    }
}
