//! The folder browser and its active Markdown note.

use std::io;
use std::path::{Path, PathBuf};

use dioxus::desktop::use_muda_event_handler;
use dioxus::prelude::{dioxus_core::use_drop, *};
use rfd::AsyncFileDialog;
use tokio::{
    sync::oneshot,
    time::{Duration, Instant, sleep_until},
};

use crate::chrome::NotebookChrome;
use crate::editor::Editor;
use crate::note_history::NoteHistory;
use crate::notebook::{self, Entry};
use crate::notebook_menu;
use crate::page_width::PageWidth;
use crate::workspace_session::{self, WorkspaceSession};
use crate::workspace_sidebar::{SidebarDivider, WorkspaceSidebar};

#[derive(Clone, PartialEq)]
pub(crate) struct OpenNotebook {
    root: PathBuf,
    folder: PathBuf,
    entries: Vec<Entry>,
    note: Option<(PathBuf, String)>,
    start_new: bool,
    restored: bool,
}

impl OpenNotebook {
    pub(crate) async fn open(path: PathBuf, start_new: bool) -> io::Result<Self> {
        let (root, entries) = on_disk(move || notebook::open_root(&path)).await?;
        Ok(Self {
            folder: root.clone(),
            root,
            entries,
            note: None,
            start_new,
            restored: false,
        })
    }

    pub(crate) async fn restore(session: WorkspaceSession) -> io::Result<(Self, Option<String>)> {
        on_disk(move || Self::restore_on_disk(session)).await
    }

    fn restore_on_disk(session: WorkspaceSession) -> io::Result<(Self, Option<String>)> {
        let (root, mut entries) = notebook::open_root(&session.root)?;
        if root != session.root {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{} now points to a different folder. Open it again to confirm its location.",
                    session.root.display()
                ),
            ));
        }
        let mut folder = root.clone();
        let mut warnings = Vec::new();
        if session.folder != root {
            match notebook::list(&root, &session.folder) {
                Ok(children) => {
                    folder = session.folder;
                    entries = children;
                }
                Err(error) => warnings.push(format!("Could not restore sidebar folder. {error}")),
            }
        }
        let note = session
            .note
            .and_then(|path| match notebook::read(&root, &path) {
                Ok(text) => Some((path, text)),
                Err(error) => {
                    warnings.push(format!("Could not restore the last page. {error}"));
                    None
                }
            });

        Ok((
            Self {
                root,
                folder,
                entries,
                note,
                start_new: false,
                restored: true,
            },
            (!warnings.is_empty()).then(|| warnings.join(" ")),
        ))
    }
}

pub(crate) async fn pick_notebook(start_new: bool) -> io::Result<Option<OpenNotebook>> {
    let Some(folder) = AsyncFileDialog::new()
        .set_title("Open Markdown folder")
        .pick_folder()
        .await
    else {
        return Ok(None);
    };
    OpenNotebook::open(folder.path().to_owned(), start_new)
        .await
        .map(Some)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NewEntry {
    Note,
    Folder,
}

#[component]
pub(crate) fn Workspace(
    opened: OpenNotebook,
    page_width: Signal<PageWidth>,
    zoom: f64,
    mut error: Signal<Option<String>>,
    on_home: EventHandler<()>,
) -> Element {
    let root = use_signal(|| opened.root);
    let mut directory = use_signal(|| opened.folder);
    let mut entries = use_signal(|| opened.entries);
    let mut selected = use_signal(|| opened.note.as_ref().map(|(path, _)| path.clone()));
    let mut draft = use_signal(|| opened.note.map_or_else(String::new, |(_, text)| text));
    let mut saved = use_signal(|| draft.peek().clone());
    let mut last_edit = use_signal(|| None::<Instant>);
    let mut failed_edit = use_signal(|| None::<Instant>);
    let mut saving = use_signal(|| false);
    let mut preview = use_signal(|| false);
    let mut sidebar_open = use_signal(|| true);
    let mut chrome_hidden = use_signal(|| false);
    let mut history = use_signal(|| {
        let mut history = NoteHistory::default();
        if let Some(path) = selected.peek().as_deref() {
            history.visit(path);
        }
        history
    });
    let mut naming = use_signal(|| opened.start_new.then_some(NewEntry::Note));
    let mut name = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let confirmation = use_signal(|| None::<oneshot::Sender<bool>>);
    let dirty = use_memo(move || *draft.read() != *saved.read());
    // Keep unavailable paths until the user opens a new location.
    let mut remember_location = use_signal(|| !opened.restored);
    use_effect(move || {
        if remember_location()
            && let Err(failure) =
                workspace_session::save(&root.peek(), &directory.read(), selected.read().as_deref())
        {
            error.set(Some(failure.to_string()));
        }
    });

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
                    remember_location.set(true);
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
                    remember_location.set(true);
                    preview.set(false);
                    chrome_hidden.set(false);
                    naming.set(None);
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
            let Some(kind) = *naming.peek() else { return };
            if *busy.peek() {
                return;
            }
            busy.set(true);
            error.set(None);
            if kind == NewEntry::Folder || allow_leave(draft, saved, confirmation).await {
                let root = root.peek().clone();
                let folder = directory.peek().clone();
                let title = name.peek().clone();
                let result = on_disk(move || {
                    let note = match kind {
                        NewEntry::Note => Some(notebook::create(&root, &folder, &title)?),
                        NewEntry::Folder => {
                            notebook::create_folder(&root, &folder, &title)?;
                            None
                        }
                    };
                    Ok((note, notebook::list(&root, &folder)))
                })
                .await;
                match result {
                    Ok((note, children)) => {
                        match children {
                            Ok(children) => entries.set(children),
                            Err(failure) => {
                                error.set(Some(format!("Created successfully. {failure}")))
                            }
                        }
                        if let Some(path) = note {
                            saved.set(String::new());
                            draft.set(String::new());
                            history.write().visit(&path);
                            selected.set(Some(path));
                            remember_location.set(true);
                            preview.set(false);
                        }
                        chrome_hidden.set(false);
                        naming.set(None);
                    }
                    Err(failure) => error.set(Some(failure.to_string())),
                }
            }
            busy.set(false);
        }
    };

    let save = move |expected_edit: Option<Instant>| async move {
        let edited = *last_edit.peek();
        if *busy.peek()
            || *draft.peek() == *saved.peek()
            || expected_edit.is_some_and(|expected| {
                edited != Some(expected) || *failed_edit.peek() == Some(expected)
            })
        {
            return;
        }
        let Some(path) = selected.peek().clone() else {
            return;
        };
        saving.set(true);
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
                failed_edit.set(None);
                // Refresh the saved note's modification time in the sidebar.
                drop(entries.write());
            }
            Err(failure) => {
                failed_edit.set(edited);
                error.set(Some(failure.to_string()));
            }
        }
        saving.set(false);
        busy.set(false);
    };
    use_resource(move || async move {
        let Some(edited) = last_edit() else { return };
        if busy() || !dirty() || failed_edit() == Some(edited) {
            return;
        }
        sleep_until(edited + Duration::from_secs(1)).await;
        // Cancel superseded timers without interrupting a write already in progress.
        spawn(save(Some(edited)));
    });

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
    let mut begin = move |kind| {
        if !*busy.peek() && *naming.peek() != Some(kind) {
            chrome_hidden.set(false);
            error.set(None);
            name.set(String::new());
            naming.set(Some(kind));
        }
    };
    use_muda_event_handler(move |event| match event.id().0.as_str() {
        notebook_menu::NEW_NOTE => begin(NewEntry::Note),
        notebook_menu::NEW_FOLDER => begin(NewEntry::Folder),
        notebook_menu::HOME => {
            spawn(go_home());
        }
        notebook_menu::TOGGLE_SIDEBAR => toggle_sidebar(()),
        notebook_menu::TOGGLE_PREVIEW => toggle_preview(()),
        notebook_menu::SAVE => {
            spawn(save(None));
        }
        _ => {}
    });
    use_effect(move || {
        notebook_menu::update(true, selected.read().is_some(), preview(), dirty(), busy())
    });
    use_drop(|| notebook_menu::update(false, false, false, false, false));

    let filename = match selected.read().as_ref() {
        Some(path) => display_name(path),
        None if naming() == Some(NewEntry::Note) => "Untitled".to_owned(),
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
            style: "--editor-page-width: {columns}ch; --ui-zoom: {zoom}",
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
                    if let Some(kind) = naming() {
                        form { key: "{kind as u8}", class: "new-note", onsubmit: create,
                            label { r#for: "entry-name",
                                if kind == NewEntry::Note { "Note name" } else { "Folder name" }
                            }
                            input {
                                id: "entry-name",
                                value: "{name}",
                                required: true,
                                disabled: busy(),
                                onmounted: move |event| async move { let _ = event.set_focus(true).await; },
                                oninput: move |event| name.set(event.value()),
                            }
                            button { r#type: "submit", disabled: busy(), "Create" }
                            button { r#type: "button", disabled: busy(), onclick: move |_| naming.set(None), "Cancel" }
                        }
                    }
                    if let Some(path) = selected() {
                        Editor { key: "{path:?}",
                            draft, dirty, preview, chrome_hidden, busy: busy() && !saving(),
                            on_edit: move |text| {
                                draft.set(text);
                                last_edit.set(Some(Instant::now()));
                            },
                            on_save: move |_| save(None),
                        }
                    } else if naming().is_none() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn restores_the_sidebar_folder_and_active_page_independently() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().canonicalize()?;
        let folder = root.join("Drafts");
        let note = root.join("Active.md");
        fs::create_dir(&folder)?;
        fs::write(&note, "# Saved page")?;
        let (opened, warning) = OpenNotebook::restore_on_disk(WorkspaceSession {
            root: root.clone(),
            folder: folder.clone(),
            note: Some(note.clone()),
        })?;

        assert_eq!(opened.root, root);
        assert_eq!(opened.folder, folder);
        assert!(opened.entries.is_empty());
        assert_eq!(opened.note, Some((note, "# Saved page".to_owned())));
        assert!(warning.is_none());
        Ok(())
    }

    #[test]
    fn reports_missing_paths_and_opens_the_available_notebook() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().canonicalize()?;
        let folder = root.join("Missing folder");
        let note = folder.join("Missing page.md");
        let (opened, warning) = OpenNotebook::restore_on_disk(WorkspaceSession {
            root: root.clone(),
            folder: folder.clone(),
            note: Some(note),
        })?;

        assert_eq!(opened.folder, root);
        assert!(opened.note.is_none());
        let warning = warning.expect("missing paths should be reported");
        assert!(warning.contains("Could not restore sidebar folder"));
        assert!(warning.contains("Could not restore the last page"));
        let failure = OpenNotebook::restore_on_disk(WorkspaceSession {
            root: folder.clone(),
            folder,
            note: None,
        });
        assert!(matches!(failure, Err(error) if error.kind() == io::ErrorKind::NotFound));
        Ok(())
    }
}
