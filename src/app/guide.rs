//! The bundled usage guide for xuy.

use dioxus::prelude::*;

use crate::ui::{Button, ButtonKind, Dialog};

#[component]
pub(crate) fn UserGuide(on_close: EventHandler<()>) -> Element {
    let content = use_hook(|| crate::editor::render_markdown(include_str!("../../docs/usage.md")));
    rsx! {
        Dialog { id: "app-guide", class: "app-dialog", aria_label: "Using xuy", on_close,
            div { class: "guide-content", tabindex: "0", autofocus: true,
                dangerous_inner_html: "{content}"
            }
            form { method: "dialog",
                Button { kind: ButtonKind::Submit, "Done" }
            }
        }
    }
}
