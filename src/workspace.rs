//! The folder browser and its active Markdown note.

use std::io;
use std::path::{Path, PathBuf};

use dioxus::desktop::use_muda_event_handler;
use dioxus::prelude::{dioxus_core::use_drop, *};
use rfd::AsyncFileDialog;
use tokio::sync::oneshot;

use crate::chrome::NotebookChrome;
use crate::editor::Editor;
use crate::note_history::NoteHistory;
use crate::notebook::{self, Entry};
use crate::notebook_menu;
use crate::page_width::PageWidth;
use crate::workspace_sidebar::{SidebarDivider, WorkspaceSidebar};

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
pub(crate) fn Workspace(
    opened: OpenNotebook,
    page_width: Signal<PageWidth>,
    on_home: EventHandler<()>,
) -> Element {
    let root = use_signal(|| opened.root);
    let mut directory = use_signal(|| root.peek().clone());
    let mut entries = use_signal(|| opened.entries);
    let mut selected = use_signal(|| None::<PathBuf>);
    let mut draft = use_signal(String::new);
    let mut saved = use_signal(String::new);
    let mut preview = use_signal(|| false);
    let mut sidebar_open = use_signal(|| true);
    let mut chrome_hidden = use_signal(|| false);
    let mut history = use_signal(NoteHistory::default);
    let mut naming = use_signal(|| opened.start_new);
    let mut name = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let confirmation = use_signal(|| None::<oneshot::Sender<bool>>);
    let dirty = use_memo(move || *draft.read() != *saved.read());

    let resolve_confirmation = move |discard| {
        let mut confirmation = confirmation;
        if let Some(sender) = confirmation.write().take() {
            let _ = sender.send(discard);
        }
    };

    let browse = move |entry: Entry, history_index: Option<usize>| async move {
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
            let current = directory.peek().clone();
            let note = on_disk(move || {
                let text = notebook::read(&root, &file)?;
                let parent = file.parent().expect("a notebook note has a parent");
                let folder = if parent == current {
                    None
                } else {
                    Some((parent.to_owned(), notebook::list(&root, parent)?))
                };
                Ok((text, folder))
            })
            .await;
            match note {
                Ok((text, folder)) => {
                    if let Some((parent, children)) = folder {
                        directory.set(parent);
                        entries.set(children);
                    }
                    saved.set(text.clone());
                    draft.set(text);
                    if let Some(index) = history_index {
                        history.write().arrive(index);
                    } else {
                        history.write().visit(&path);
                    }
                    selected.set(Some(path));
                    preview.set(false);
                    chrome_hidden.set(false);
                    naming.set(false);
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
        }
        busy.set(false);
    };

    let move_history = move |offset| async move {
        let destination = history
            .peek()
            .destination(offset)
            .map(|(index, path)| (index, path.to_owned()));
        if let Some((index, path)) = destination {
            browse(
                Entry {
                    path,
                    is_dir: false,
                },
                Some(index),
            )
            .await;
        }
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
                        history.write().visit(&path);
                        selected.set(Some(path));
                        preview.set(false);
                        chrome_hidden.set(false);
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
            Ok(text) => {
                saved.set(text);
                // Refresh the saved note's modification time in the sidebar.
                drop(entries.write());
            }
            Err(failure) => error.set(Some(failure.to_string())),
        }
        busy.set(false);
    };

    let go_home = move || async move {
        if *busy.peek() {
            return;
        }
        busy.set(true);
        let confirmed = allow_leave(draft, saved, confirmation).await;
        busy.set(false);
        if confirmed {
            on_home.call(());
        }
    };
    let mut toggle_preview = move |_| {
        if !*busy.peek() && selected.peek().is_some() {
            let next = !*preview.peek();
            preview.set(next);
            chrome_hidden.set(false);
        }
    };
    let mut toggle_sidebar = move |_| sidebar_open.toggle();
    use_muda_event_handler(move |event| match event.id().0.as_str() {
        notebook_menu::HOME => {
            spawn(go_home());
        }
        notebook_menu::TOGGLE_SIDEBAR => toggle_sidebar(()),
        notebook_menu::TOGGLE_PREVIEW => toggle_preview(()),
        notebook_menu::SAVE => {
            spawn(save(()));
        }
        _ => {}
    });
    use_effect(move || {
        notebook_menu::update(true, selected.read().is_some(), preview(), dirty(), busy())
    });
    use_drop(|| notebook_menu::update(false, false, false, false, false));

    let filename = match selected.read().as_ref() {
        Some(path) => display_name(path),
        None if naming() => "Untitled".to_owned(),
        None => String::new(),
    };
    let can_back = !busy() && history.read().destination(-1).is_some();
    let can_forward = !busy() && history.read().destination(1).is_some();
    let can_preview = !busy() && selected.read().is_some();
    let columns = page_width().columns();
    let folder = directory();
    let parent = folder
        .parent()
        .filter(|_| folder != *root.peek())
        .map(Path::to_owned);

    rsx! {
        document::Stylesheet { href: asset!("/assets/workspace.css") }
        main {
            class: "workspace",
            "data-sidebar-open": sidebar_open(),
            style: "--editor-page-width: {columns}ch",
            aria_busy: busy() && confirmation.read().is_none(),
            onkeydown: move |event| {
                if event.key() == Key::Escape && !event.is_composing() {
                    if confirmation.peek().is_some() {
                        event.prevent_default();
                        resolve_confirmation(false);
                    }
                    if *chrome_hidden.peek() {
                        event.prevent_default();
                        chrome_hidden.set(false);
                    }
                }
            },
            NotebookChrome {
                filename, dirty: dirty(), hidden: chrome_hidden, sidebar_open: sidebar_open(), can_back, can_forward,
                folder, parent, busy: busy(),
                on_parent: move |path| browse(Entry { path, is_dir: true }, None),
                page_width, preview: preview(), can_preview, on_preview: toggle_preview,
                on_toggle: toggle_sidebar,
                on_back: move |_| move_history(-1),
                on_forward: move |_| move_history(1),
            }
            div { class: "workspace-body",
                div { id: "workspace-sidebar", class: "workspace-sidebar", "inert": (!sidebar_open()).then_some(""),
                    WorkspaceSidebar {
                        root: root.peek().clone(),
                        entries,
                        selected: selected(),
                        busy: busy(),
                        on_browse: move |entry| browse(entry, None),
                    },
                }
                SidebarDivider {}
                div { class: "workspace-content",
                    if let Some(message) = error() {
                        p { class: "workspace-error", role: "alert", "{message}" }
                    }
                    if confirmation.read().is_some() {
                        div { class: "unsaved-choice",
                            span { role: "alert", "Discard unsaved changes?" }
                            button {
                                onmounted: move |event| async move { let _ = event.set_focus(true).await; },
                                onclick: move |_| resolve_confirmation(false),
                                "Keep editing"
                            }
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
                            draft, dirty, preview, chrome_hidden, busy: busy(), on_save: save,
                        }
                    } else if !naming() {
                        p { class: "workspace-message", "Select a Markdown note." }
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
