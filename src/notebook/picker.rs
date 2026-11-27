//! Dialog for adding and opening saved notebook folders.

use std::path::PathBuf;

use dioxus::prelude::*;

use crate::ui::{Button, ButtonKind, Dialog, close_dialog};

use super::display_name;

const DIALOG_ID: &str = "notebook-picker";

pub(crate) fn close() {
    close_dialog(DIALOG_ID);
}

#[component]
pub(crate) fn NotebookPicker(
    saved: Signal<Vec<PathBuf>>,
    busy: bool,
    error: Option<String>,
    on_open: EventHandler<PathBuf>,
    on_add: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/notebook-picker.css") }
        Dialog { id: DIALOG_ID, class: "app-dialog", aria_labelledby: "notebook-picker-title",
            aria_busy: busy, on_close,
            h1 { id: "notebook-picker-title", "Notebooks" }
            if saved.read().is_empty() {
                p { "No saved notebooks." }
            } else {
                ul {
                    for path in saved() {
                        li { key: "{path:?}",
                            Button { disabled: busy,
                                onclick: { let path = path.clone(); move |_| on_open.call(path.clone()) },
                                span { "{display_name(&path)}" }
                                small { "{path.display()}" }
                            }
                        }
                    }
                }
            }
            if let Some(message) = error {
                p { role: "alert", "{message}" }
            }
            form { method: "dialog",
                Button { disabled: busy, onclick: move |_| on_add.call(()), "Add Folder…" }
                Button { kind: ButtonKind::Submit, disabled: busy, "Done" }
            }
        }
    }
}
