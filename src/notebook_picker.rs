//! Saved notebook folders, opening history, and the quick-access dialog.

use std::io;
use std::path::{Path, PathBuf};

use dioxus::prelude::*;

use crate::ui::{Button, ButtonKind, Dialog, close_dialog};

#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSString, NSUserDefaults};

const SAVED_KEY: &str = "xuy.notebooks";
const RECENT_KEY: &str = "xuy.recent-notebooks";
const RECENT_LIMIT: usize = 5;
const DIALOG_ID: &str = "notebook-picker";

pub(crate) fn close() {
    close_dialog(DIALOG_ID);
}

pub(crate) fn load() -> Vec<PathBuf> {
    load_paths(SAVED_KEY)
}

pub(crate) fn load_recent() -> Vec<PathBuf> {
    load_paths(RECENT_KEY)
}

fn load_paths(_key: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    if let Some(array) =
        NSUserDefaults::standardUserDefaults().arrayForKey(&NSString::from_str(_key))
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
    save_paths(SAVED_KEY, &next)?;
    saved.set(next);
    Ok(())
}

pub(crate) fn record_open(mut recent: Signal<Vec<PathBuf>>, path: &Path) -> io::Result<()> {
    if recent.peek().first().is_some_and(|first| first == path) {
        return Ok(());
    }
    let next = recent_paths(&recent.peek(), path);
    save_paths(RECENT_KEY, &next)?;
    recent.set(next);
    Ok(())
}

fn recent_paths(previous: &[PathBuf], path: &Path) -> Vec<PathBuf> {
    std::iter::once(path.to_owned())
        .chain(
            previous
                .iter()
                .filter(|existing| *existing != path)
                .cloned(),
        )
        .take(RECENT_LIMIT)
        .collect()
}

fn save_paths(_key: &str, _paths: &[PathBuf]) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let strings = _paths
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
                .setObject_forKey(Some(&array), &NSString::from_str(_key))
        };
    }

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
                Button { disabled: busy, onclick: move |_| on_add.call(()), "Add Folder…" }
                Button { kind: ButtonKind::Submit, disabled: busy, "Done" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_notebooks_move_to_the_front_and_stay_bounded() {
        let previous: Vec<_> = ["a", "b", "c", "d", "e"].map(PathBuf::from).into();
        assert_eq!(
            recent_paths(&previous, Path::new("c")),
            ["c", "a", "b", "d", "e"].map(PathBuf::from)
        );
        assert_eq!(
            recent_paths(&previous, Path::new("f")),
            ["f", "a", "b", "c", "d"].map(PathBuf::from)
        );
    }
}
