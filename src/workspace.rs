//! The folder browser and its active Markdown note.

use std::io;
use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use rfd::AsyncFileDialog;
use tokio::sync::oneshot;

use crate::editor::Editor;
use crate::notebook::{self, Entry};

#[derive(Clone, PartialEq)]
pub(crate) struct OpenNotebook {
    root: PathBuf,
    entries: Vec<Entry>,
    start_new: bool,
}

pub(crate) async fn pick_notebook(start_new: bool) -> io::Result<Option<OpenNotebook>> {
    let Some(folder) = AsyncFileDialog::new()
        .set_title("Open Markdown folder")
        .pick_folder()
        .await
    else {
        return Ok(None);
    };
    let path = folder.path().to_owned();
    let (root, entries) = on_disk(move || notebook::open_root(&path)).await?;
    Ok(Some(OpenNotebook {
        root,
        entries,
        start_new,
    }))
}

#[component]
pub(crate) fn Workspace(opened: OpenNotebook, on_home: EventHandler<()>) -> Element {
    let root = use_signal(|| opened.root);
    let mut directory = use_signal(|| root.peek().clone());
    let mut entries = use_signal(|| opened.entries);
    let mut selected = use_signal(|| None::<PathBuf>);
    let mut draft = use_signal(String::new);
    let mut saved = use_signal(String::new);
    let mut preview = use_signal(|| true);
    let mut naming = use_signal(|| opened.start_new);
    let mut name = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let confirmation = use_signal(|| None::<oneshot::Sender<bool>>);

    let resolve_confirmation = move |discard| {
        let mut confirmation = confirmation;
        if let Some(sender) = confirmation.write().take() {
            let _ = sender.send(discard);
        }
    };

    let browse = move |entry: Entry| async move {
        if *busy.peek() || selected.peek().as_ref() == Some(&entry.path) {
            return;
        }
        busy.set(true);
        error.set(None);
        let root = root.peek().clone();
        let Entry { path, is_dir } = entry;
        if is_dir {
            let folder = path.clone();
            match on_disk(move || notebook::list(&root, &folder)).await {
                Ok(children) => {
                    entries.set(children);
                    directory.set(path);
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
        } else if allow_leave(draft, saved, confirmation).await {
            let file = path.clone();
            match on_disk(move || notebook::read(&root, &file)).await {
                Ok(text) => {
                    saved.set(text.clone());
                    draft.set(text);
                    selected.set(Some(path));
                    preview.set(true);
                    naming.set(false);
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
        }
        busy.set(false);
    };

    let create = move |event: FormEvent| {
        event.prevent_default();
        async move {
            if *busy.peek() {
                return;
            }
            busy.set(true);
            error.set(None);
            if allow_leave(draft, saved, confirmation).await {
                let root = root.peek().clone();
                let folder = directory.peek().clone();
                let title = name.peek().clone();
                let result = on_disk(move || {
                    let path = notebook::create(&root, &folder, &title)?;
                    Ok((path, notebook::list(&root, &folder)))
                })
                .await;
                match result {
                    Ok((path, children)) => {
                        match children {
                            Ok(children) => entries.set(children),
                            Err(failure) => error.set(Some(format!("Note created. {failure}"))),
                        }
                        saved.set(String::new());
                        draft.set(String::new());
                        selected.set(Some(path));
                        preview.set(false);
                        naming.set(false);
                    }
                    Err(failure) => error.set(Some(failure.to_string())),
                }
            }
            busy.set(false);
        }
    };

    let save = move |_| async move {
        if *busy.peek() || *draft.peek() == *saved.peek() {
            return;
        }
        let Some(path) = selected.peek().clone() else {
            return;
        };
        busy.set(true);
        error.set(None);
        let root = root.peek().clone();
        let original = saved.peek().clone();
        let text = draft.peek().clone();
        let result = on_disk(move || {
            notebook::save(&root, &path, &original, &text)?;
            Ok(text)
        })
        .await;
        match result {
            Ok(text) => saved.set(text),
            Err(failure) => error.set(Some(failure.to_string())),
        }
        busy.set(false);
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/workspace.css") }
        main { class: "workspace", aria_busy: busy() && confirmation.read().is_none(),
            div { class: "workspace-body",
                FolderList {
                    root: root.peek().clone(),
                    directory: directory(),
                    entries,
                    selected: selected(),
                    busy: busy(),
                    on_home: move |_| async move {
                        if *busy.peek() { return; }
                        busy.set(true);
                        let confirmed = allow_leave(draft, saved, confirmation).await;
                        busy.set(false);
                        if confirmed { on_home.call(()); }
                    },
                    on_browse: move |entry| async move { browse(entry).await },
                }
                div { class: "workspace-content",
                    if let Some(message) = error() {
                        p { class: "workspace-error", role: "alert", "{message}" }
                    }
                    if confirmation.read().is_some() {
                        div { class: "unsaved-choice",
                            span { role: "alert", "Discard unsaved changes?" }
                            button { onclick: move |_| resolve_confirmation(false), "Keep editing" }
                            button { onclick: move |_| resolve_confirmation(true), "Discard changes" }
                        }
                    }
                    if naming() {
                        form { class: "new-note", onsubmit: create,
                            label { r#for: "note-name", "Note name" }
                            input {
                                id: "note-name",
                                value: "{name}",
                                required: true,
                                disabled: busy(),
                                onmounted: move |event| async move { let _ = event.set_focus(true).await; },
                                oninput: move |event| name.set(event.value()),
                            }
                            button { r#type: "submit", disabled: busy(), "Create" }
                            button { r#type: "button", disabled: busy(), onclick: move |_| naming.set(false), "Cancel" }
                        }
                    }
                    if let Some(path) = selected() {
                        Editor { key: "{path:?}",
                            name: display_name(&path), draft, saved, preview, busy: busy(), on_save: save,
                        }
                    } else if !naming() {
                        p { class: "workspace-message", "Select a Markdown note." }
                    }
                }
            }
        }
    }
}

#[component]
fn FolderList(
    root: PathBuf,
    directory: PathBuf,
    entries: Signal<Vec<Entry>>,
    selected: Option<PathBuf>,
    busy: bool,
    on_home: EventHandler<()>,
    on_browse: EventHandler<Entry>,
) -> Element {
    let parent = (directory != root)
        .then(|| directory.parent().map(Path::to_owned))
        .flatten();
    let current = directory
        .strip_prefix(&root)
        .expect("directory must be in the notebook");
    let location = if current.as_os_str().is_empty() {
        "/".to_owned()
    } else {
        format!("/{}", current.display())
    };

    rsx! {
        aside { class: "folder-list", aria_label: "Notebook files",
            div { class: "folder-toolbar",
                button { disabled: busy, onclick: move |_| on_home.call(()), "Home" }
                if let Some(parent) = parent {
                    button { disabled: busy, onclick: move |_| on_browse.call(Entry { path: parent.clone(), is_dir: true }), "Up" }
                }
            }
            h2 { class: "folder-path", title: "{location}", "{location}" }
            if entries.read().is_empty() {
                p { class: "folder-empty", "No Markdown notes or folders." }
            } else {
                ul {
                    for entry in entries.read().iter().cloned() {
                        li { key: "{entry.path:?}",
                            button {
                                class: if selected.as_ref() == Some(&entry.path) { "folder-entry current-note" } else { "folder-entry" },
                                disabled: busy,
                                title: "{entry.path.display()}",
                                onclick: { let entry = entry.clone(); move |_| on_browse.call(entry.clone()) },
                                "{display_name(&entry.path)}"
                                if entry.is_dir { "/" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

async fn allow_leave(
    draft: Signal<String>,
    saved: Signal<String>,
    mut confirmation: Signal<Option<oneshot::Sender<bool>>>,
) -> bool {
    if *draft.peek() == *saved.peek() {
        return true;
    }
    let (sender, response) = oneshot::channel();
    confirmation.set(Some(sender));
    response.await.is_ok_and(|discard| discard)
}

async fn on_disk<T: Send + 'static>(
    operation: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(io::Error::other)?
}
