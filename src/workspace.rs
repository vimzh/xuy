//! The folder browser and its active Markdown note.

use std::io;
use std::path::Path;

use dioxus::desktop::use_muda_event_handler;
use dioxus::prelude::{dioxus_core::use_drop, *};
use tokio::{
    sync::oneshot,
    time::{Duration, Instant, sleep_until},
};

use crate::chrome::NotebookChrome;
use crate::commands::{self, PageActions};
use crate::editor::{Editor, ViewMode};
use crate::markdown_paths::MarkdownPaths;
use crate::markdown_preview::PreviewTarget;
use crate::note_history::NoteHistory;
use crate::notebook::{self, Entry};
use crate::notebook_index::{NoteTime, use_folder_index};
use crate::notebook_menu;
use crate::page_width::PageWidth;
use crate::workspace_session;
use crate::workspace_sidebar::{SidebarDivider, WorkspaceSidebar};

mod open;

pub(crate) use open::{OpenNotebook, pick_notebook};

#[derive(Clone, Copy, PartialEq, Eq)]
enum NewEntry {
    Note,
    Folder,
}

#[component]
pub(crate) fn Workspace(
    opened: OpenNotebook,
    page_width: Signal<PageWidth>,
    edit_in_preview: bool,
    zoom: f64,
    mut busy: Signal<bool>,
    mut error: Signal<Option<String>>,
    mut pending: Signal<Option<OpenNotebook>>,
    mut palette_command: Signal<Option<&'static str>>,
    mut page_actions: Signal<PageActions>,
    modal_open: Memo<bool>,
    on_switch: EventHandler<OpenNotebook>,
    on_home: EventHandler<()>,
) -> Element {
    let root = use_signal(|| opened.root);
    let mut directory = use_signal(|| opened.folder);
    let index = use_folder_index(|| opened.entries);
    let mut selected = use_signal(|| opened.note.as_ref().map(|(path, _)| path.clone()));
    let mut draft = use_signal(|| opened.note.map_or_else(String::new, |(_, text)| text));
    let mut saved = use_signal(|| draft.peek().clone());
    let mut last_edit = use_signal(|| None::<Instant>);
    let mut failed_edit = use_signal(|| None::<Instant>);
    let mut saving = use_signal(|| false);
    let mut view_mode = use_signal(ViewMode::default);
    let mut preview_anchor = use_signal(|| None::<String>);
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

    let browse = move |entry: Entry, history_index: Option<usize>, mode: ViewMode| async move {
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
                    index.replace(children);
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
                        index.replace(children);
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
                    preview_anchor.set(None);
                    view_mode.set(mode);
                    chrome_hidden.set(false);
                    naming.set(None);
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
        }
        busy.set(false);
    };

    let open_preview_link = move |(target, wiki): PreviewTarget| async move {
        if *busy.peek() || *modal_open.peek() || *view_mode.peek() == ViewMode::Edit {
            return;
        }
        let Some(origin) = selected.peek().clone() else {
            return;
        };
        let paths = MarkdownPaths::new(root.peek().clone(), origin.clone());
        let resolved = on_disk(move || paths.resolve_note(&target, wiki)).await;
        let mode = *view_mode.peek();
        if *busy.peek() || mode == ViewMode::Edit || selected.peek().as_ref() != Some(&origin) {
            return;
        }
        match resolved {
            Ok((path, fragment)) => {
                if path != origin {
                    browse(
                        Entry {
                            path: path.clone(),
                            is_dir: false,
                        },
                        None,
                        mode,
                    )
                    .await;
                }
                if selected.peek().as_ref() == Some(&path) {
                    error.set(None);
                    let fragment = fragment.map(|fragment| {
                        if wiki {
                            crate::markdown::heading_slug(&fragment)
                        } else {
                            fragment
                        }
                    });
                    preview_anchor.set(fragment);
                }
            }
            Err(failure) => error.set(Some(failure.to_string())),
        }
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
                ViewMode::Edit,
            )
            .await;
        }
    };

    let create = move |event: FormEvent| {
        event.prevent_default();
        async move {
            let Some(kind) = *naming.peek() else { return };
            if *busy.peek() || *modal_open.peek() {
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
                            Ok(children) => index.replace(children),
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
                            preview_anchor.set(None);
                            view_mode.set(ViewMode::Edit);
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
            || *modal_open.peek()
            || pending.peek().is_some()
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
        let listed = path.parent() == Some(directory.peek().as_path());
        let result = on_disk(move || {
            notebook::save(&root, &path, &original, &text)?;
            let updated = listed.then(|| {
                let time = NoteTime::read(&root, &path);
                (path, time)
            });
            Ok((text, updated))
        })
        .await;
        match result {
            Ok((text, updated)) => {
                saved.set(text);
                failed_edit.set(None);
                if let Some((path, time)) = updated {
                    index.note_saved(path, time);
                }
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
        if busy()
            || !dirty()
            || failed_edit() == Some(edited)
            || modal_open()
            || pending.read().is_some()
        {
            return;
        }
        sleep_until(edited + Duration::from_secs(1)).await;
        // Cancel superseded timers without interrupting a write already in progress.
        spawn(save(Some(edited)));
    });

    let leave = move || async move {
        if *busy.peek() {
            return false;
        }
        busy.set(true);
        let confirmed = allow_leave(draft, saved, confirmation).await;
        busy.set(false);
        confirmed
    };
    use_effect(move || {
        if pending.read().is_some() {
            spawn(async move {
                let next = pending.write().take();
                if let Some(next) = next
                    && leave().await
                {
                    on_switch.call(next);
                }
            });
        }
    });
    let mut change_view = move |mode| {
        if (!*busy.peek() || *saving.peek())
            && !*modal_open.peek()
            && selected.peek().is_some()
            && *view_mode.peek() != mode
        {
            preview_anchor.set(None);
            view_mode.set(mode);
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
    let mut run_command = move |id: &str| match id {
        _ if *modal_open.peek() || pending.peek().is_some() => {}
        notebook_menu::NEW_NOTE => begin(NewEntry::Note),
        notebook_menu::NEW_FOLDER => begin(NewEntry::Folder),
        notebook_menu::HOME => {
            spawn(async move {
                if leave().await {
                    on_home.call(());
                }
            });
        }
        notebook_menu::TOGGLE_SIDEBAR => toggle_sidebar(()),
        notebook_menu::TOGGLE_PREVIEW => {
            let next = view_mode.peek().toggle_preview();
            change_view(next);
        }
        notebook_menu::TOGGLE_SPLIT => {
            let next = view_mode.peek().toggle_split();
            change_view(next);
        }
        commands::PREVIOUS_PAGE => {
            spawn(move_history(-1));
        }
        commands::NEXT_PAGE => {
            spawn(move_history(1));
        }
        notebook_menu::SAVE => {
            spawn(save(None));
        }
        _ => {}
    };
    use_muda_event_handler(move |event| run_command(event.id().0.as_str()));
    use_effect(move || {
        if let Some(id) = palette_command() {
            palette_command.set(None);
            run_command(id);
        }
    });
    use_effect(move || {
        let history = history.read();
        let actions = PageActions {
            has_page: selected.read().is_some(),
            view_mode: view_mode(),
            dirty: dirty(),
            can_back: history.destination(-1).is_some(),
            can_forward: history.destination(1).is_some(),
        };
        if *page_actions.peek() != actions {
            page_actions.set(actions);
        }
        notebook_menu::update(
            true,
            actions.has_page && (!busy() || saving()) && !modal_open(),
            actions.view_mode,
            actions.dirty,
            busy(),
        );
    });
    use_drop(|| notebook_menu::update(false, false, ViewMode::Edit, false, false));

    let filename = match selected.read().as_ref() {
        Some(path) => display_name(path),
        None if naming() == Some(NewEntry::Note) => "Untitled".to_owned(),
        None => String::new(),
    };
    let can_back = !busy() && history.read().destination(-1).is_some();
    let can_forward = !busy() && history.read().destination(1).is_some();
    let can_preview = (!busy() || saving()) && !modal_open() && selected.read().is_some();
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
                on_parent: move |path| browse(Entry { path, is_dir: true }, None, ViewMode::Edit),
                page_width, view_mode: view_mode(), can_preview, on_view_mode: change_view,
                on_toggle: toggle_sidebar,
                on_back: move |_| move_history(-1),
                on_forward: move |_| move_history(1),
            }
            div { class: "workspace-body",
                div { id: "workspace-sidebar", class: "workspace-sidebar", "inert": (!sidebar_open()).then_some(""),
                    WorkspaceSidebar {
                        root: root.peek().clone(),
                        index,
                        selected: selected(),
                        busy: busy(),
                        on_browse: move |entry| browse(entry, None, ViewMode::Edit),
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
                            draft, dirty, view_mode, edit_in_preview, chrome_hidden, busy: (busy() && !saving()) || modal_open(),
                            paths: MarkdownPaths::new(root.peek().clone(), path), preview_anchor,
                            on_preview_open: open_preview_link,
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
