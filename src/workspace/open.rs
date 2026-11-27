//! Opening and restoring a notebook before mounting its workspace.

use std::io;
use std::path::{Path, PathBuf};

use rfd::AsyncFileDialog;

use crate::notebook::{self, Entry};
use crate::workspace_session::WorkspaceSession;

use super::on_disk;

#[derive(Clone, PartialEq)]
pub(crate) struct OpenNotebook {
    pub(super) root: PathBuf,
    pub(super) folder: PathBuf,
    pub(super) entries: Vec<Entry>,
    pub(super) note: Option<(PathBuf, String)>,
    pub(super) start_new: bool,
    pub(super) restored: bool,
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

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

pub(crate) async fn pick_notebook(
    title: &str,
    start_new: bool,
    can_create: bool,
) -> io::Result<Option<OpenNotebook>> {
    match AsyncFileDialog::new()
        .set_title(title)
        .set_can_create_directories(can_create)
        .pick_folder()
        .await
    {
        Some(folder) => OpenNotebook::open(folder.path().to_owned(), start_new)
            .await
            .map(Some),
        None => Ok(None),
    }
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
