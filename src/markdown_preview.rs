//! Render a page off the UI thread and handle notebook links in the preview.

use dioxus::prelude::{dioxus_core::use_drop, *};
use tokio::time::{Duration, sleep};

use crate::editor::render_preview;
use crate::markdown_paths::MarkdownPaths;

pub(crate) type PreviewTarget = (String, bool);

#[component]
pub(crate) fn MarkdownPreview(
    draft: Signal<String>,
    paths: MarkdownPaths,
    mut anchor: Signal<Option<String>>,
    on_open: EventHandler<PreviewTarget>,
) -> Element {
    let rendered = use_resource(move || {
        // Subscribe before the delay so each new edit cancels the pending render.
        drop(draft.read());
        let paths = paths.clone();
        async move {
            sleep(Duration::from_millis(120)).await;
            let source = draft.peek().clone();
            tokio::task::spawn_blocking(move || render_preview(&source, Some(&paths))).await
        }
    });
    let render_state = rendered.state();
    let mut bridge = use_signal(|| None::<document::Eval>);
    use_drop(move || {
        if let Some(eval) = *bridge.peek() {
            let _ = eval.send(None::<String>);
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

    rsx! {
        document::Stylesheet { href: asset!("/assets/markdown-preview.css") }
        article { class: "markdown-preview", aria_label: "Markdown preview", tabindex: "0",
            match &*rendered.read_unchecked() {
                Some(Ok(html)) => rsx! {
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
                Some(Err(_)) => rsx! {
                    p { role: "alert", "Could not render the preview." }
                },
                None => rsx! { p { role: "status", "Rendering preview…" } },
            }
        }
    }
}
