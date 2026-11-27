//! Context actions for a selection in the Markdown source editor.

use dioxus::prelude::{dioxus_core::use_drop, *};

use crate::highlight::Color;

#[component]
pub(super) fn FormatMenu(
    highlighting: bool,
    on_highlight: EventHandler<Color>,
    feedback: Signal<Option<&'static str>>,
) -> Element {
    let mut bridge = use_signal(|| None::<document::Eval>);
    use_drop(move || {
        if let Some(bridge) = bridge.write().take() {
            let _ = bridge.send(());
        }
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/editor-format-menu.css") }
        div {
            id: "editor-format-menu", popover: "auto", role: "group", aria_label: "Format selected text",
            onmounted: move |_| async move {
                let script = document::eval(include_str!("format_menu.js"));
                bridge.set(Some(script));
                if let Err(error) = script.join::<()>().await {
                    eprintln!("Could not initialize the text formatting menu: {error}");
                }
            },
            button { r#type: "button", autofocus: true, "data-format": "**",
                strong { class: "format-sample", aria_hidden: true, "B" }
                "Bold"
            }
            button { r#type: "button", "data-format": "*",
                em { class: "format-sample", aria_hidden: true, "I" }
                "Italic"
            }
            button { r#type: "button", "data-format": "~~",
                s { class: "format-sample", aria_hidden: true, "S" }
                "Strikethrough"
            }
            div { class: "format-palette", role: "group", aria_label: "Highlight",
                span { class: "format-palette-label", aria_hidden: true, "Highlight" }
                for color in Color::ALL {
                    button {
                        key: "{color.class()}", r#type: "button", disabled: highlighting,
                        title: "{color.name()}", aria_label: "Highlight {color.name()}",
                        onclick: move |_| on_highlight.call(color),
                        span { class: "highlight-swatch {color.class()}", aria_hidden: true }
                    }
                }
            }
        }
        p { class: "workspace-error", role: "status", hidden: feedback().is_none(),
            {feedback().unwrap_or_default()}
        }
        p { id: "editor-format-error", class: "workspace-error", role: "status", hidden: true }
    }
}
