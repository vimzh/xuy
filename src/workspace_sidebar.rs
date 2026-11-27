//! The notebook's folder navigation and compact Markdown note index.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use time::{Date, OffsetDateTime, UtcOffset};

use crate::notebook::{self, Entry, NoteDetails};

#[derive(Clone, Copy, PartialEq)]
enum SortOrder {
    Date,
    Name,
}

type FolderDetails = HashMap<PathBuf, io::Result<(NoteDetails, Option<Date>)>>;

#[component]
pub(crate) fn WorkspaceSidebar(
    root: PathBuf,
    directory: PathBuf,
    entries: Signal<Vec<Entry>>,
    selected: Option<PathBuf>,
    busy: bool,
    on_browse: EventHandler<Entry>,
) -> Element {
    let relative = directory
        .strip_prefix(&root)
        .expect("directory must be in the notebook");
    let location = format!("/{}", relative.display());
    let parent = directory.parent().filter(|_| directory != root);
    let mut sort_order = use_signal(|| SortOrder::Date);
    let details = use_resource(move || {
        let root = root.clone();
        let listed = entries();
        async move {
            let active = Arc::new(());
            let active_worker = Arc::downgrade(&active);
            let details = tokio::task::spawn_blocking(move || {
                listed
                    .into_iter()
                    .filter(|entry| !entry.is_dir)
                    // Stop between files if a new folder replaces this resource.
                    .take_while(|_| active_worker.strong_count() != 0)
                    .map(|entry| {
                        let details = notebook::note_details(&root, &entry.path).map(|note| {
                            let date = local_date(note.modified);
                            (note, date)
                        });
                        (entry.path, details)
                    })
                    .collect::<FolderDetails>()
            })
            .await;
            drop(active);
            details
        }
    });
    let details = details.read();
    let details_error = details.as_ref().and_then(|value| value.as_ref().err());
    let folder_details = details.as_ref().and_then(|value| value.as_ref().ok());
    let entries = entries.read();
    let mut ordered: Vec<_> = entries.iter().collect();
    let sort = sort_order();
    if sort == SortOrder::Date
        && let Some(details) = folder_details
    {
        sort_by_date(&mut ordered, details);
    }

    rsx! {
        document::Stylesheet { href: asset!("/assets/sidebar.css") }
        aside { class: "notebook-sidebar", aria_label: "Notebook files",
            div { class: "sidebar-toolbar",
                if let Some(parent) = parent.map(Path::to_owned) {
                    button {
                        class: "sidebar-up",
                        r#type: "button",
                        title: "Up to parent folder",
                        aria_label: "Up to parent folder",
                        disabled: busy,
                        onclick: move |_| on_browse.call(Entry { path: parent.clone(), is_dir: true }),
                        img { src: asset!("/assets/hugeicons/arrow-up-01.svg"), alt: "" }
                    }
                }
                span { class: "sidebar-path", title: "{location}", "{location}" }
                div { class: "sidebar-sort", role: "group", aria_label: "Sort notes",
                    button {
                        r#type: "button",
                        title: "Sort by last modified",
                        aria_pressed: sort == SortOrder::Date,
                        onclick: move |_| sort_order.set(SortOrder::Date),
                        "Date"
                    }
                    button {
                        r#type: "button",
                        title: "Sort by name",
                        aria_pressed: sort == SortOrder::Name,
                        onclick: move |_| sort_order.set(SortOrder::Name),
                        "Name"
                    }
                }
            }
            if let Some(error) = details_error {
                p { class: "sidebar-status", role: "status", title: "{error}", "Note details unavailable." }
            }
            if ordered.is_empty() {
                p { class: "sidebar-status", "No Markdown notes or folders." }
            } else {
                ul { class: "sidebar-entries",
                    {ordered.into_iter().map(|entry| {
                        let active = selected.as_ref() == Some(&entry.path);
                        let note = folder_details.and_then(|details| details.get(&entry.path));
                        let name = entry.path.file_name().unwrap_or(entry.path.as_os_str()).to_string_lossy();
                        let icon = if entry.is_dir {
                            asset!("/assets/hugeicons/folder-01.svg")
                        } else {
                            asset!("/assets/hugeicons/file-02.svg")
                        };
                        rsx! {
                            li { key: "{entry.path:?}",
                                button {
                                    class: if entry.is_dir { "sidebar-entry sidebar-folder" } else { "sidebar-entry" },
                                    r#type: "button",
                                    disabled: busy,
                                    aria_current: active.then_some("true"),
                                    title: "{entry.path.display()}",
                                    onclick: { let entry = entry.clone(); move |_| on_browse.call(entry.clone()) },
                                    if let Some(note) = note {
                                        if let Some(date) = note.as_ref().ok().and_then(|(_, date)| *date) {
                                            time {
                                                class: "sidebar-date",
                                                datetime: "{date}",
                                                title: "Modified {date}",
                                                "{date_label(date)}"
                                                span { "{date.year()}" }
                                            }
                                        } else {
                                            span { class: "sidebar-date", title: "Modified date unavailable", "—" }
                                        }
                                    }
                                    img { class: "sidebar-entry-icon", src: icon, alt: "" }
                                    span { class: "sidebar-entry-text",
                                        span { class: "sidebar-entry-name", "{name}" }
                                        if let Some(note) = note {
                                            match note.as_ref().and_then(|(note, _)| {
                                                note.preview.as_deref()
                                            }) {
                                                Ok(preview) => rsx! { span { class: "sidebar-excerpt", "{preview}" } },
                                                Err(error) => rsx! { span { class: "sidebar-excerpt", title: "{error}", "Preview unavailable" } },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    })}
                }
            }
        }
    }
}

fn sort_by_date(entries: &mut [&Entry], details: &FolderDetails) {
    // Stable sort keeps the notebook's name order for folders and equal dates.
    entries.sort_by_cached_key(|entry| {
        let modified = details
            .get(&entry.path)
            .and_then(|result| result.as_ref().ok())
            .map(|(note, _)| note.modified);
        (!entry.is_dir, Reverse(modified))
    });
}

fn local_date(modified: SystemTime) -> Option<Date> {
    let nanos = match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as i128,
        Err(error) => -(error.duration().as_nanos() as i128),
    };
    let utc = OffsetDateTime::from_unix_timestamp_nanos(nanos).ok()?;
    let offset = UtcOffset::local_offset_at(utc).ok()?;
    utc.checked_to_offset(offset).map(OffsetDateTime::date)
}

fn date_label(date: Date) -> String {
    format!("{:.3} {}", date.month(), date.day())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn dates_sort_newest_first_after_folders_with_name_order_for_ties() {
        let [folder, first, second, newest, missing] = ["folder", "a.md", "b.md", "c.md", "d.md"]
            .map(|path| Entry {
                path: path.into(),
                is_dir: path == "folder",
            });
        let details = [(&first, 1), (&second, 1), (&newest, 2)]
            .into_iter()
            .map(|(entry, seconds)| {
                (
                    entry.path.clone(),
                    Ok((
                        NoteDetails {
                            modified: UNIX_EPOCH + Duration::from_secs(seconds),
                            preview: Ok(String::new()),
                        },
                        None,
                    )),
                )
            })
            .collect();
        let mut entries = [&folder, &first, &second, &newest, &missing];
        sort_by_date(&mut entries, &details);
        assert_eq!(entries, [&folder, &newest, &first, &second, &missing]);
    }
}
