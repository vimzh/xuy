//! Saved notebook folders and the quick-access dialog.

use std::io;
use std::path::{Path, PathBuf};

use dioxus::prelude::*;

use crate::modal::show_dialog;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSString, NSUserDefaults};

#[cfg(target_os = "macos")]
const KEY: &str = "xuy.notebooks";

pub(crate) fn load() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    if let Some(array) =
        NSUserDefaults::standardUserDefaults().arrayForKey(&NSString::from_str(KEY))
    {
        return (0..array.len())
            .filter_map(
                |index| match array.objectAtIndex(index).downcast::<NSString>() {
                    Ok(value) => Some(PathBuf::from(value.to_string())),
                    Err(_) => {
                        eprintln!(
                            "Could not restore notebook at position {index}: expected a path"
                        );
                        None
                    }
                },
            )
            .collect();
    }
    Vec::new()
}

pub(crate) fn register(mut saved: Signal<Vec<PathBuf>>, path: &Path) -> io::Result<()> {
    if saved.peek().iter().any(|existing| existing == path) {
        return Ok(());
    }
    let mut next = saved.peek().clone();
    next.push(path.to_owned());

    #[cfg(target_os = "macos")]
    {
        let strings = next
            .iter()
            .map(|path| {
                path.to_str().map(NSString::from_str).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Cannot save notebook path {} as text", path.display()),
                    )
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        let array = NSArray::from_retained_slice(&strings);
        // NSUserDefaults accepts arrays of property-list strings.
        unsafe {
            NSUserDefaults::standardUserDefaults()
                .setObject_forKey(Some(&array), &NSString::from_str(KEY))
        };
    }

    saved.set(next);
    Ok(())
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
        dialog { id: "notebook-picker", class: "app-dialog", aria_labelledby: "notebook-picker-title",
            aria_busy: busy,
            onmounted: move |_| show_dialog("notebook-picker", on_close),
            h1 { id: "notebook-picker-title", "Notebooks" }
            if saved.read().is_empty() {
                p { "No saved notebooks." }
            } else {
                ul {
                    for path in saved() {
                        li { key: "{path:?}",
                            button { r#type: "button", disabled: busy,
                                onclick: move |_| on_open.call(path.clone()),
                                span { "{path.file_name().unwrap_or(path.as_os_str()).to_string_lossy()}" }
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
                button { r#type: "button", disabled: busy, onclick: move |_| on_add.call(()), "Add Folder…" }
                button { r#type: "submit", disabled: busy, "Done" }
            }
        }
    }
}
