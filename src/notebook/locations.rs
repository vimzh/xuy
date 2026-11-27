//! Saved and recently opened notebook folders in macOS preferences.

use std::io;
use std::path::{Path, PathBuf};

use dioxus::prelude::*;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSString, NSUserDefaults};

const SAVED_KEY: &str = "xuy.notebooks";
const RECENT_KEY: &str = "xuy.recent-notebooks";
const RECENT_LIMIT: usize = 5;

pub(crate) fn load_saved() -> Vec<PathBuf> {
    load_paths(SAVED_KEY)
}

pub(crate) fn load_recent() -> Vec<PathBuf> {
    load_paths(RECENT_KEY)
}

fn load_paths(_key: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    if let Some(array) =
        NSUserDefaults::standardUserDefaults().arrayForKey(&NSString::from_str(_key))
    {
        return (0..array.len())
            .filter_map(
                |index| match array.objectAtIndex(index).downcast::<NSString>() {
                    Ok(value) => Some(PathBuf::from(value.to_string())),
                    Err(_) => {
                        eprintln!(
                            "Could not restore notebook at position {index}: expected a path"
                        );
                        None
                    }
                },
            )
            .collect();
    }
    Vec::new()
}

pub(crate) fn register(saved: Signal<Vec<PathBuf>>, path: &Path) -> io::Result<()> {
    if saved.peek().iter().any(|existing| existing == path) {
        return Ok(());
    }
    let mut next = saved.peek().clone();
    next.push(path.to_owned());
    update(saved, SAVED_KEY, next)
}

pub(crate) fn record_open(recent: Signal<Vec<PathBuf>>, path: &Path) -> io::Result<()> {
    if recent.peek().first().is_some_and(|first| first == path) {
        return Ok(());
    }
    let next = recent_paths(&recent.peek(), path);
    update(recent, RECENT_KEY, next)
}

fn update(mut folders: Signal<Vec<PathBuf>>, key: &str, next: Vec<PathBuf>) -> io::Result<()> {
    save_paths(
        key,
        next.iter().map(PathBuf::as_path),
        "Cannot save notebook path",
    )?;
    folders.set(next);
    Ok(())
}

fn recent_paths(previous: &[PathBuf], path: &Path) -> Vec<PathBuf> {
    std::iter::once(path.to_owned())
        .chain(
            previous
                .iter()
                .filter(|existing| *existing != path)
                .cloned(),
        )
        .take(RECENT_LIMIT)
        .collect()
}

pub(crate) fn save_paths<'a>(
    _key: &str,
    _paths: impl IntoIterator<Item = &'a Path>,
    _error_context: &str,
) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let strings = _paths
            .into_iter()
            .map(|path| {
                path.to_str().map(NSString::from_str).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{_error_context} {} as text", path.display()),
                    )
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        let array = NSArray::from_retained_slice(&strings);
        // NSUserDefaults accepts arrays of property-list strings.
        unsafe {
            NSUserDefaults::standardUserDefaults()
                .setObject_forKey(Some(&array), &NSString::from_str(_key))
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_notebooks_move_to_the_front_and_stay_bounded() {
        let previous: Vec<_> = ["a", "b", "c", "d", "e"].map(PathBuf::from).into();
        assert_eq!(
            recent_paths(&previous, Path::new("c")),
            ["c", "a", "b", "d", "e"].map(PathBuf::from)
        );
        assert_eq!(
            recent_paths(&previous, Path::new("f")),
            ["f", "a", "b", "c", "d"].map(PathBuf::from)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn path_preferences_keep_their_ordered_strings_after_an_invalid_write() -> io::Result<()> {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let temporary = tempfile::tempdir()?;
        let key = format!("xuy.test.paths.{}", temporary.path().display());
        let paths = [Path::new("/Notebook"), Path::new("/Notebook/Page.md")];
        save_paths(&key, paths, "Cannot save path")?;

        let invalid_path = Path::new(OsStr::from_bytes(b"\xff"));
        let invalid_write = save_paths(&key, [paths[0], invalid_path], "Cannot save path");
        let stored = load_paths(&key);
        NSUserDefaults::standardUserDefaults().removeObjectForKey(&NSString::from_str(&key));

        assert_eq!(
            invalid_write.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(stored, paths.map(Path::to_owned));
        Ok(())
    }
}
