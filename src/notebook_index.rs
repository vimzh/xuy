//! The open folder's sidebar entries and modification times.

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
    pub(crate) local: Option<OffsetDateTime>,
}

impl NoteTime {
    pub(crate) fn read(root: &Path, path: &Path) -> Self {
        Self {
            local: notebook::note_modified(root, path)
                .ok()
                .and_then(local_timestamp),
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
        indexed_rows(&index.entries.read(), times, &index.saved_times.read())
    });
    let error = use_memo(move || {
        details
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().err().map(ToString::to_string))
    });
    (rows, error)
}

fn indexed_rows(
    entries: &[Entry],
    loaded: Option<&NoteTimes>,
    saved: &NoteTimes,
) -> Vec<IndexedEntry> {
    entries
        .iter()
        .cloned()
        .map(|entry| {
            let time = saved
                .get(&entry.path)
                .or_else(|| loaded.and_then(|times| times.get(&entry.path)))
                .copied();
            IndexedEntry { entry, time }
        })
        .collect()
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
    fn updates_timestamps_without_reordering_pages() {
        let entries = ["folder", "a.md", "b.md", "c.md"].map(|name| Entry {
            path: name.into(),
            is_dir: name == "folder",
        });
        let timestamp = |seconds| NoteTime {
            local: OffsetDateTime::from_unix_timestamp(seconds).ok(),
        };
        let loaded = HashMap::from([("a.md".into(), timestamp(2)), ("b.md".into(), timestamp(1))]);
        let saved = HashMap::from([("b.md".into(), timestamp(3))]);
        for times in [None, Some(&loaded)] {
            let rows = indexed_rows(&entries, times, &saved);
            assert!(rows.iter().map(|row| &row.entry).eq(&entries));
            assert_eq!(rows[1].time, times.map(|_| timestamp(2)));
            assert_eq!(rows[2].time, Some(timestamp(3)));
        }
    }
}
