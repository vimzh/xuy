//! The open folder's sidebar entries and modification times.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use time::{Date, OffsetDateTime, UtcOffset};

use crate::notebook::{self, Entry};

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct FolderIndex {
    entries: Signal<Vec<Entry>>,
    saved_times: Signal<HashMap<PathBuf, NoteTime>>,
}

pub(crate) fn use_folder_index(initial: impl FnOnce() -> Vec<Entry>) -> FolderIndex {
    FolderIndex {
        entries: use_signal(initial),
        saved_times: use_signal(HashMap::new),
    }
}

impl FolderIndex {
    pub(crate) fn replace(mut self, entries: Vec<Entry>) {
        self.saved_times.write().clear();
        self.entries.set(entries);
    }

    pub(crate) fn note_saved(mut self, path: PathBuf, time: NoteTime) {
        self.saved_times.write().insert(path, time);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NoteTime {
    modified: Option<SystemTime>,
    pub(crate) local: Option<OffsetDateTime>,
}

impl NoteTime {
    pub(crate) fn read(root: &Path, path: &Path) -> Self {
        let modified = notebook::note_modified(root, path).ok();
        Self {
            modified,
            local: modified.and_then(local_timestamp),
        }
    }
}

#[derive(PartialEq)]
pub(crate) struct IndexedEntry {
    pub(crate) entry: Entry,
    pub(crate) time: Option<NoteTime>,
}

type NoteTimes = HashMap<PathBuf, NoteTime>;

pub(crate) fn use_indexed_rows(
    root: PathBuf,
    index: FolderIndex,
) -> (Memo<Vec<IndexedEntry>>, Memo<Option<String>>) {
    let details = use_resource(move || {
        let root = root.clone();
        let entries = (index.entries)();
        async move {
            let active = Arc::new(());
            let active_worker = Arc::downgrade(&active);
            let times = tokio::task::spawn_blocking(move || {
                entries
                    .into_iter()
                    .filter(|entry| !entry.is_dir)
                    // Stop between files when a different folder replaces the load.
                    .take_while(|_| active_worker.strong_count() != 0)
                    .map(|entry| {
                        let time = NoteTime::read(&root, &entry.path);
                        (entry.path, time)
                    })
                    .collect::<NoteTimes>()
            })
            .await;
            drop(active);
            times
        }
    });
    let rows = use_memo(move || {
        let loaded = details.read();
        let times = loaded.as_ref().and_then(|result| result.as_ref().ok());
        ordered_rows(&index.entries.read(), times, &index.saved_times.read())
    });
    let error = use_memo(move || {
        details
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().err().map(ToString::to_string))
    });
    (rows, error)
}

fn ordered_rows(
    entries: &[Entry],
    loaded: Option<&NoteTimes>,
    saved: &NoteTimes,
) -> Vec<IndexedEntry> {
    let mut rows: Vec<_> = entries
        .iter()
        .cloned()
        .map(|entry| {
            let time = saved
                .get(&entry.path)
                .or_else(|| loaded.and_then(|times| times.get(&entry.path)))
                .copied();
            IndexedEntry { entry, time }
        })
        .collect();
    if loaded.is_some() || !saved.is_empty() {
        // Keep the folder listing's name order when modification times match.
        rows.sort_by_key(|row| {
            (
                !row.entry.is_dir,
                Reverse(row.time.and_then(|time| time.modified)),
            )
        });
    }
    rows
}

pub(crate) fn local_today() -> Option<Date> {
    OffsetDateTime::now_local().ok().map(OffsetDateTime::date)
}

fn local_timestamp(modified: SystemTime) -> Option<OffsetDateTime> {
    let nanos = match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as i128,
        Err(error) => -(error.duration().as_nanos() as i128),
    };
    let utc = OffsetDateTime::from_unix_timestamp_nanos(nanos).ok()?;
    let offset = UtcOffset::local_offset_at(utc).ok()?;
    utc.checked_to_offset(offset)
}

pub(crate) fn modified_label(modified: OffsetDateTime, today: Option<Date>) -> String {
    let date = modified.date();
    if today == Some(date) {
        return format!("{:02}:{:02}", modified.hour(), modified.minute());
    }
    format!(
        "{:02}/{:02}/{:02}",
        date.day(),
        u8::from(date.month()),
        date.year().rem_euclid(100)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn shows_local_time_until_midnight_then_shows_the_date() {
        let date = Date::from_calendar_date(2026, time::Month::November, 27).unwrap();
        let modified = date
            .with_hms(0, 5, 0)
            .unwrap()
            .assume_offset(UtcOffset::from_hms(5, 30, 0).unwrap());
        assert_eq!(modified_label(modified, Some(date)), "00:05");
        assert_eq!(modified_label(modified, date.next_day()), "27/11/26");
    }

    #[test]
    fn orders_saved_pages_by_their_new_time_without_reloading_other_notes() {
        let entries = ["folder", "a.md", "b.md", "c.md", "d.md"].map(|name| Entry {
            path: name.into(),
            is_dir: name == "folder",
        });
        let timestamp = |seconds| NoteTime {
            modified: Some(UNIX_EPOCH + Duration::from_secs(seconds)),
            local: None,
        };
        let loaded = HashMap::from([
            ("a.md".into(), timestamp(1)),
            ("b.md".into(), timestamp(1)),
            ("c.md".into(), timestamp(2)),
        ]);
        let saved = HashMap::from([("b.md".into(), timestamp(3))]);
        let names = |rows: Vec<IndexedEntry>| {
            rows.into_iter()
                .map(|row| row.entry.path)
                .collect::<Vec<_>>()
        };
        let expected = |names: [&str; 5]| names.map(PathBuf::from);
        assert_eq!(
            names(ordered_rows(&entries, Some(&loaded), &NoteTimes::new())),
            expected(["folder", "c.md", "a.md", "b.md", "d.md"])
        );
        assert_eq!(
            names(ordered_rows(&entries, Some(&loaded), &saved)),
            expected(["folder", "b.md", "c.md", "a.md", "d.md"])
        );
        assert_eq!(
            names(ordered_rows(&entries, None, &saved)),
            expected(["folder", "b.md", "a.md", "c.md", "d.md"])
        );
    }
}
