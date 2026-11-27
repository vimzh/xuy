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
use crate::cursor::CursorOptions;
use crate::editor::{Editor, ViewMode};
use crate::external_edit::{self, Conflict, ExternalEditNotice, Revision};
use crate::markdown_paths::MarkdownPaths;
use crate::markdown_preview::PreviewTarget;
use crate::note_history::NoteHistory;
use crate::note_status::{NoteStatusBar, NoteSummary};
use crate::notebook::{self, Entry};
use crate::notebook_index::{NoteTime, use_folder_index};
use crate::notebook_menu;
use crate::page_width::PageWidth;
use crate::performance::PerformancePanel;
use crate::terminal::{self, TerminalPanel};
use crate::workspace_session;
use crate::workspace_sidebar::{SidebarDivider, WorkspaceSidebar};
use crate::writing_style::WritingStyle;

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
    cursor: Signal<Option<CursorOptions>>,
    show_performance: bool,
    zoom: f64,
    markdown_font: u8,
    writing_style: WritingStyle,
    mut busy: Signal<bool>,
    mut error: Signal<Option<String>>,
    mut pending: Signal<Option<OpenNotebook>>,
    mut palette_command: Signal<Option<&'static str>>,
    mut page_actions: Signal<PageActions>,
    terminal_state: Signal<terminal::State>,
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
    let mut conflict = use_signal(|| None::<Conflict>);
    let mut saving = use_signal(|| false);
    let mut view_mode = use_signal(ViewMode::default);
    let mut preview_anchor = use_signal(|| None::<String>);
    let mut sidebar_open = use_signal(|| true);
    let mut writing_sidebar_open = use_signal(|| true);
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
    let (disk_revision, watch_error) = external_edit::use_folder_watch(directory, selected);
    let mut indexed_revision = use_signal(|| None);
    let mut disk_error = use_signal(|| None::<String>);
    use_resource(move || {
        let folder = directory();
        let note = selected();
        let revision = disk_revision();
        let available = !busy();
        async move {
            if !available {
                return;
            }
            let root = root.peek().clone();
            let read_folder = folder.clone();
            let read_note = note.clone();
            let refresh_index =
                indexed_revision.peek().as_ref() != Some(&(folder.clone(), revision));
            let result = on_disk(move || {
                let entries = refresh_index.then(|| notebook::list(&root, &read_folder));
                let disk = read_note
                    .as_ref()
                    .map(|path| notebook::read(&root, path).map_err(|error| error.to_string()));
                Ok((entries, disk))
            })
            .await;
            if *busy.peek()
                || *disk_revision.peek() != revision
                || *directory.peek() != folder
                || *selected.peek() != note
            {
                return;
            }
            match result {
                Ok((entries, disk)) => {
                    match entries {
                        Some(Ok(entries)) => {
                            index.replace(entries);
                            indexed_revision.set(Some((folder, revision)));
                            disk_error.set(None);
                        }
                        Some(Err(failure)) => disk_error.set(Some(failure.to_string())),
                        None => {}
                    }
                    if let Some(disk) = disk {
                        let update = external_edit::reconcile(&saved.peek(), &draft.peek(), disk);
                        match update {
                            Revision::Unchanged => {
                                if conflict.peek().is_some() {
                                    conflict.set(None);
                                    failed_edit.set(None);
                                }
                            }
                            Revision::Reload(text) => {
                                saved.set(text.clone());
                                if *draft.peek() != text {
                                    draft.set(text);
                                }
                                last_edit.set(None);
                                failed_edit.set(None);
                                conflict.set(None);
                            }
                            Revision::Conflict(next) => {
                                if conflict.peek().as_ref() != Some(&next) {
                                    conflict.set(Some(next));
                                }
                            }
                        }
                    }
                }
                Err(failure) => disk_error.set(Some(failure.to_string())),
            }
        }
    });
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
    let summary = use_memo(move || {
        if selected.read().is_some() {
            NoteSummary::parse(&draft.read())
        } else {
            NoteSummary::default()
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
                    conflict.set(None);
                    last_edit.set(None);
                    failed_edit.set(None);
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
                            conflict.set(None);
                            last_edit.set(None);
                            failed_edit.set(None);
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
            || conflict.peek().is_some()
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
            if let Err(failure) = notebook::save(&root, &path, &original, &text) {
                let observed = notebook::read(&root, &path).map_err(|error| error.to_string());
                return Ok(Err((failure, observed)));
            }
            let updated = listed.then(|| {
                let time = NoteTime::read(&root, &path);
                (path, time)
            });
            Ok(Ok((text, updated)))
        })
        .await;
        match result {
            Ok(Ok((text, updated))) => {
                saved.set(text);
                failed_edit.set(None);
                if let Some((path, time)) = updated {
                    index.note_saved(path, time);
                }
            }
            Ok(Err((failure, observed))) => {
                failed_edit.set(edited);
                match external_edit::reconcile(&saved.peek(), &draft.peek(), observed) {
                    Revision::Conflict(next) => conflict.set(Some(next)),
                    _ => error.set(Some(failure.to_string())),
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
            || conflict.read().is_some()
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

    let reload_external = move |_| async move {
        if *busy.peek() || !matches!(conflict.peek().as_ref(), Some(Conflict::Changed(_))) {
            return;
        }
        let Some(path) = selected.peek().clone() else {
            return;
        };
        busy.set(true);
        let root = root.peek().clone();
        match on_disk(move || notebook::read(&root, &path)).await {
            Ok(text) => {
                saved.set(text.clone());
                draft.set(text);
                last_edit.set(None);
                failed_edit.set(None);
                conflict.set(None);
                error.set(None);
            }
            Err(failure) => conflict.set(Some(Conflict::Unavailable(failure.to_string()))),
        }
        busy.set(false);
    };
    let save_external_copy = move |_| async move {
        if *busy.peek() || conflict.peek().is_none() {
            return;
        }
        let Some(path) = selected.peek().clone() else {
            return;
        };
        busy.set(true);
        let root = root.peek().clone();
        let text = draft.peek().clone();
        match on_disk(move || {
            let copy = notebook::save_draft_copy(&root, &path, &text)?;
            let folder = copy.parent().expect("draft copy has a folder").to_owned();
            let entries = notebook::list(&root, &folder);
            Ok((copy, folder, entries, text))
        })
        .await
        {
            Ok((copy, folder, entries, text)) => {
                match entries {
                    Ok(entries) => {
                        index.replace(entries);
                        directory.set(folder);
                        error.set(None);
                    }
                    Err(failure) => error.set(Some(format!(
                        "Draft saved to {}. {failure}",
                        copy.display()
                    ))),
                }
                history.write().visit(&copy);
                remember_location.set(true);
                selected.set(Some(copy));
                saved.set(text);
                last_edit.set(None);
                failed_edit.set(None);
                conflict.set(None);
            }
            Err(failure) => error.set(Some(failure.to_string())),
        }
        busy.set(false);
    };

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
        commands::INSERT_CODE_BLOCK if selected.peek().is_some() && !*busy.peek() => {
            spawn(async move {
                let mut edit = document::eval(include_str!("editor/insert_code_block.js"));
                let result = edit.recv::<Option<String>>().await;
                let _ = edit.send(());
                match result {
                    Ok(Some(message)) => error.set(Some(message)),
                    Ok(None) => {}
                    Err(failure) => {
                        error.set(Some(format!("Could not insert the code block. {failure}")))
                    }
                }
            });
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
            dirty: dirty() && conflict.read().is_none(),
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
    let font_family = writing_style.font();
    let code_font = writing_style.code_font();
    let line_height = writing_style.line_height();
    let folder = directory();
    let parent = folder
        .parent()
        .filter(|_| folder != *root.peek())
        .map(Path::to_owned);

    let has_note = selected.read().is_some();
    let note_tools_open = writing_sidebar_open();
    let notices = rsx! {
        if let Some(message) = error() {
            p { class: "workspace-error", role: "alert", "{message}" }
        }
        if let Some(message) = watch_error() {
            p { class: "workspace-error", role: "status", "{message}" }
        }
        if let Some(message) = disk_error() {
            p { class: "workspace-error", role: "status", "{message}" }
        }
        if let Some(current) = conflict() {
            ExternalEditNotice { conflict: current, busy: busy(), on_reload: reload_external, on_save_copy: save_external_copy }
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
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/workspace.css") }
        document::Stylesheet { href: asset!("/assets/terminal.css") }
        main {
            class: "workspace",
            "data-sidebar-open": sidebar_open(),
            "data-writing-sidebar-open": has_note && note_tools_open,
            "data-code-theme": writing_style.code_theme(),
            style: "--editor-page-width: {columns}ch; --markdown-font-size: {markdown_font}px; --writing-font: {font_family}; --writing-code-font: {code_font}; --writing-line-height: {line_height}; --ui-zoom: {zoom}",
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
                filename: filename.clone(), dirty: dirty(), hidden: chrome_hidden, sidebar_open: sidebar_open(), can_back, can_forward,
                folder, parent, busy: busy(),
                on_parent: move |path| browse(Entry { path, is_dir: true }, None, ViewMode::Edit),
                page_width, view_mode: view_mode(), can_preview, on_view_mode: change_view,
                on_toggle: toggle_sidebar,
                writing_sidebar_open: has_note.then_some(note_tools_open),
                on_toggle_writing_sidebar: move |_| writing_sidebar_open.toggle(),
                on_back: move |_| move_history(-1),
                on_forward: move |_| move_history(1),
            }
            if show_performance {
                PerformancePanel {}
            }
            div { class: "workspace-body",
                div { id: "workspace-sidebar", class: "workspace-sidebar", "inert": (!sidebar_open()).then_some(""),
                    WorkspaceSidebar {
                        root: root.peek().clone(),
                        index,
                        selected: selected(),
                        busy: busy(),
                        on_browse: move |entry| browse(entry, None, ViewMode::Edit),
                        on_context_action: run_command,
                    },
                }
                SidebarDivider {}
                div { class: "workspace-content",
                    if let Some(path) = selected() {
                        Editor { key: "{path:?}",
                            draft, dirty, view_mode, edit_in_preview, chrome_hidden, busy: (busy() && !saving()) || modal_open(), summary,
                            paths: MarkdownPaths::new(root.peek().clone(), path), preview_anchor,
                            on_preview_open: open_preview_link,
                            on_edit: move |text| {
                                draft.set(text);
                                last_edit.set(Some(Instant::now()));
                            },
                            on_save: move |_| save(None),
                            cursor: cursor().unwrap_or_default(),
                            notices, writing_sidebar_open: note_tools_open,
                        }
                    } else {
                        div { class: "workspace-writing",
                            {notices}
                            if naming().is_none() {
                                p { class: "workspace-message", "Select a Markdown note." }
                            }
                        }
                    }
                }
            }
            if let Some(path) = selected() {
                NoteStatusBar { key: "{path:?}", filename, summary }
            }
            if terminal_state() != terminal::State::Closed {
                TerminalPanel { state: terminal_state, folder: directory(), zoom }
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
