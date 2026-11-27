//! The notebook's folder navigation and compact Markdown note index.

use std::path::PathBuf;
use std::time::Duration;

use dioxus::prelude::{dioxus_core::use_drop, *};
use time::OffsetDateTime;

use crate::notebook::Entry;
use crate::notebook_index::{FolderIndex, local_today, modified_label, use_indexed_rows};
use crate::notebook_menu::{NEW_FOLDER, NEW_NOTE};

#[component]
pub(crate) fn WorkspaceSidebar(
    root: PathBuf,
    index: FolderIndex,
    selected: Option<PathBuf>,
    busy: bool,
    on_browse: EventHandler<Entry>,
    on_context_action: EventHandler<&'static str>,
) -> Element {
    let mut menu_bridge = use_signal(|| None::<document::Eval>);
    use_drop(move || {
        if let Some(bridge) = menu_bridge.write().take() {
            let _ = bridge.send(());
        }
    });
    let mut today = use_signal(local_today);
    use_future(move || async move {
        loop {
            let now = OffsetDateTime::now_utc();
            let elapsed = Duration::new(now.second().into(), now.nanosecond());
            tokio::time::sleep(Duration::from_secs(60) - elapsed).await;
            let date = local_today();
            if date != *today.peek() {
                today.set(date);
            }
        }
    });
    let (rows, details_error) = use_indexed_rows(root, index);
    let ordered = rows.read();
    let today = today();

    rsx! {
        document::Stylesheet { href: asset!("/assets/sidebar.css") }
        aside { id: "notebook-files", class: "notebook-sidebar", aria_label: "Notebook files",
            aria_busy: busy,
            aria_keyshortcuts: "Shift+F10",
            tabindex: if ordered.is_empty() { "0" } else { "-1" },
            onmounted: move |_| async move {
                let bridge = document::eval(include_str!("sidebar_menu.js"));
                menu_bridge.set(Some(bridge));
                if let Err(error) = bridge.join::<()>().await {
                    eprintln!("Could not initialize the sidebar menu: {error}");
                }
            },
            SidebarContextMenu { on_action: on_context_action }
            if let Some(error) = details_error() {
                p { class: "sidebar-status", role: "status", title: "{error}", "Note details unavailable." }
            }
            if ordered.is_empty() {
                p { class: "sidebar-status", "No Markdown notes or folders." }
            } else {
                ul { class: "sidebar-entries",
                    {ordered.iter().map(|row| {
                        let entry = &row.entry;
                        let active = selected.as_ref() == Some(&entry.path);
                        let note = row.time;
                        let name = if entry.is_dir {
                            entry.path.file_name()
                        } else {
                            entry.path.file_stem()
                        }.unwrap_or(entry.path.as_os_str()).to_string_lossy();
                        let icon = if entry.is_dir {
                            asset!("/assets/hugeicons/notebook-01.svg")
                        } else {
                            asset!("/assets/hugeicons/file-02.svg")
                        };
                        rsx! {
                            li { key: "{entry.path:?}",
                                button {
                                    class: "sidebar-entry",
                                    r#type: "button",
                                    disabled: busy,
                                    aria_current: active.then_some("true"),
                                    title: "{name}",
                                    onclick: { let entry = entry.clone(); move |_| on_browse.call(entry.clone()) },
                                    img { class: "sidebar-entry-icon", src: icon, alt: "" }
                                    span { class: "sidebar-entry-name", "{name}" }
                                    if let Some(note) = note {
                                        if let Some(modified) = note.local {
                                            {
                                                let date = modified.date();
                                                let hour = modified.hour();
                                                let minute = modified.minute();
                                                rsx! {
                                                    time {
                                                        class: "sidebar-date",
                                                        datetime: "{date}T{hour:02}:{minute:02}",
                                                        title: "Modified {date} at {hour:02}:{minute:02}",
                                                        "{modified_label(modified, today)}"
                                                    }
                                                }
                                            }
                                        } else {
                                            span { class: "sidebar-date", title: "Modification time unavailable", "—" }
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

#[component]
fn SidebarContextMenu(on_action: EventHandler<&'static str>) -> Element {
    rsx! {
        div { id: "sidebar-context-menu", popover: "auto", role: "group",
            aria_label: "Create in the current folder",
            button {
                r#type: "button",
                autofocus: true,
                popovertarget: "sidebar-context-menu",
                popovertargetaction: "hide",
                onclick: move |_| on_action.call(NEW_NOTE),
                img { class: "sidebar-entry-icon", src: asset!("/assets/hugeicons/file-02.svg"), alt: "" }
                "New Page…"
            }
            button {
                r#type: "button",
                popovertarget: "sidebar-context-menu",
                popovertargetaction: "hide",
                onclick: move |_| on_action.call(NEW_FOLDER),
                img { class: "sidebar-entry-icon", src: asset!("/assets/hugeicons/folder-01.svg"), alt: "" }
                "New Folder…"
            }
        }
    }
}

#[component]
pub(crate) fn SidebarDivider() -> Element {
    let mut bridge = use_signal(|| None::<document::Eval>);
    use_drop(move || {
        if let Some(bridge) = *bridge.peek() {
            let _ = bridge.send(());
        }
    });

    rsx! {
        div {
            id: "workspace-sidebar-divider",
            class: "workspace-sidebar-divider",
            role: "separator",
            tabindex: "0",
            aria_label: "Resize notebook sidebar",
            aria_orientation: "vertical",
            aria_controls: "workspace-sidebar",
            onmounted: move |_| async move {
                let resize = document::eval(include_str!("sidebar_resize.js"));
                bridge.set(Some(resize));
                if let Err(error) = resize.join::<()>().await {
                    eprintln!("Could not initialize sidebar resizing: {error}");
                }
            },
        }
    }
}
