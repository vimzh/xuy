//! Observes notebook folders and keeps external revisions separate from local drafts.

use std::path::PathBuf;

use dioxus::prelude::*;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use tokio::{
    sync::mpsc,
    time::{Duration, sleep},
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Conflict {
    Changed(String),
    Unavailable(String),
}

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum Revision {
    Unchanged,
    Reload(String),
    Conflict(Conflict),
}

pub(crate) fn reconcile(saved: &str, draft: &str, disk: Result<String, String>) -> Revision {
    match disk {
        Ok(text) if text == saved => Revision::Unchanged,
        Ok(text) if draft == saved || text == draft => Revision::Reload(text),
        Ok(text) => Revision::Conflict(Conflict::Changed(text)),
        Err(error) => Revision::Conflict(Conflict::Unavailable(error)),
    }
}

pub(crate) fn use_folder_watch(
    directory: Signal<PathBuf>,
    selected: Signal<Option<PathBuf>>,
) -> (Signal<u64>, Signal<Option<String>>) {
    let mut revision = use_signal(|| 0_u64);
    let mut error = use_signal(|| None::<String>);
    let parent = use_memo(move || {
        selected
            .read()
            .as_ref()
            .and_then(|path| path.parent().map(ToOwned::to_owned))
    });
    use_resource(move || {
        let directory = directory();
        let parent = parent();
        async move {
            error.set(None);
            let (send, mut receive) = mpsc::channel(1);
            let (send_error, mut receive_error) = mpsc::channel(1);
            let watcher = tokio::task::spawn_blocking(move || {
                let mut watcher =
                    notify::recommended_watcher(move |event: notify::Result<Event>| match event {
                        Ok(event) if !matches!(event.kind, EventKind::Access(_)) => {
                            let _ = send.try_send(());
                        }
                        Ok(_) => {}
                        Err(failure) => {
                            let _ = send_error.try_send(failure.to_string());
                        }
                    })?;
                watcher.watch(&directory, RecursiveMode::NonRecursive)?;
                if let Some(parent) = parent
                    && parent != directory
                {
                    watcher.watch(&parent, RecursiveMode::NonRecursive)?;
                }
                Ok::<_, notify::Error>(watcher)
            })
            .await;
            let _watcher = match watcher {
                Ok(Ok(watcher)) => watcher,
                Ok(Err(failure)) => {
                    error.set(Some(format!("Could not watch notebook folders: {failure}")));
                    return;
                }
                Err(failure) => {
                    error.set(Some(format!("Could not start notebook watch: {failure}")));
                    return;
                }
            };
            revision.with_mut(|value| *value = value.wrapping_add(1));
            loop {
                tokio::select! {
                    Some(()) = receive.recv() => {
                        // Coalesce temporary files, renames, and repeated writes from the same save.
                        sleep(Duration::from_millis(110)).await;
                        while receive.try_recv().is_ok() {}
                        revision.with_mut(|value| *value = value.wrapping_add(1));
                    }
                    Some(failure) = receive_error.recv() => {
                        error.set(Some(format!("Notebook watch reported an error: {failure}")));
                        revision.with_mut(|value| *value = value.wrapping_add(1));
                    }
                    else => break,
                }
            }
        }
    });
    (revision, error)
}

#[component]
pub(crate) fn ExternalEditNotice(
    conflict: Conflict,
    busy: bool,
    on_reload: EventHandler<()>,
    on_save_copy: EventHandler<()>,
) -> Element {
    rsx! {
        section { class: "external-edit", aria_label: "External edit",
            p { role: "status",
                match &conflict {
                    Conflict::Changed(_) => "This note changed on disk while you were editing. Autosave is paused.",
                    Conflict::Unavailable(_) => "This note is unavailable on disk. The open draft is still here.",
                }
            }
            match conflict {
                Conflict::Changed(text) => rsx! {
                    details { summary { "Review disk version" } pre { "{text}" } }
                    button { r#type: "button", disabled: busy, onclick: move |_| on_reload.call(()), "Use Disk Version" }
                },
                Conflict::Unavailable(error) => rsx! { p { "{error}" } },
            }
            button { r#type: "button", disabled: busy, onclick: move |_| on_save_copy.call(()), "Save Draft as Copy" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Conflict, Revision, reconcile};

    #[test]
    fn syncs_clean_and_already_matching_text_without_replacing_diverged_drafts() {
        assert_eq!(
            reconcile("base", "base", Ok("disk".into())),
            Revision::Reload("disk".into())
        );
        assert_eq!(
            reconcile("base", "disk", Ok("disk".into())),
            Revision::Reload("disk".into())
        );
        assert_eq!(
            reconcile("base", "local", Ok("base".into())),
            Revision::Unchanged
        );
        assert_eq!(
            reconcile("base", "local", Ok("disk".into())),
            Revision::Conflict(Conflict::Changed("disk".into()))
        );
        assert_eq!(
            reconcile("base", "local", Err("removed".into())),
            Revision::Conflict(Conflict::Unavailable("removed".into()))
        );
    }
}
