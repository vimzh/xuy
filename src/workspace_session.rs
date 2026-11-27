//! The last open notebook, sidebar folder, and page in macOS preferences.

use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSString, NSUserDefaults};

#[cfg(target_os = "macos")]
const KEY: &str = "xuy.workspace-session";

pub(crate) struct WorkspaceSession {
    pub(crate) root: PathBuf,
    pub(crate) folder: PathBuf,
    pub(crate) note: Option<PathBuf>,
}

pub(crate) fn load() -> io::Result<Option<WorkspaceSession>> {
    #[cfg(target_os = "macos")]
    {
        let Some(value) =
            NSUserDefaults::standardUserDefaults().objectForKey(&NSString::from_str(KEY))
        else {
            return Ok(None);
        };
        let invalid = || {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Saved notebook session is invalid",
            )
        };
        let paths = value.downcast::<NSArray>().map_err(|_| invalid())?;
        if !matches!(paths.len(), 2 | 3) {
            return Err(invalid());
        }
        let path = |index| {
            paths
                .objectAtIndex(index)
                .downcast::<NSString>()
                .map(|value| PathBuf::from(value.to_string()))
                .map_err(|_| invalid())
        };
        Ok(Some(WorkspaceSession {
            root: path(0)?,
            folder: path(1)?,
            note: (paths.len() == 3).then(|| path(2)).transpose()?,
        }))
    }
    #[cfg(not(target_os = "macos"))]
    Ok(None)
}

pub(crate) fn save(root: &Path, folder: &Path, note: Option<&Path>) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let paths = [root, folder]
            .into_iter()
            .chain(note)
            .map(|path| {
                path.to_str().map(NSString::from_str).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Cannot remember path {} as text", path.display()),
                    )
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        let paths = NSArray::from_retained_slice(&paths);
        // A single property-list entry keeps all three paths in step.
        unsafe {
            NSUserDefaults::standardUserDefaults()
                .setObject_forKey(Some(&paths), &NSString::from_str(KEY));
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (root, folder, note);
    Ok(())
}

pub(crate) fn clear() {
    #[cfg(target_os = "macos")]
    NSUserDefaults::standardUserDefaults().removeObjectForKey(&NSString::from_str(KEY));
}
