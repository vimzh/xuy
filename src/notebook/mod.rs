//! Filesystem operations for a local folder of Markdown notes.

pub(crate) mod index;
pub(crate) mod locations;
pub(crate) mod picker;

use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    pub(crate) path: PathBuf,
    pub(crate) is_dir: bool,
}

impl Entry {
    pub(crate) fn name(&self) -> &OsStr {
        if self.is_dir {
            self.path.file_name()
        } else {
            self.path.file_stem()
        }
        .unwrap_or(self.path.as_os_str())
    }
}

pub(crate) fn display_name(path: &Path) -> Cow<'_, str> {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
}

pub(crate) fn open_root(path: &Path) -> io::Result<(PathBuf, Vec<Entry>)> {
    let root = fs::canonicalize(path).map_err(|err| at(err, "Open folder", path))?;
    let entries = list(&root, &root)?;
    Ok((root, entries))
}

pub(crate) fn list(root: &Path, folder: &Path) -> io::Result<Vec<Entry>> {
    check_folder(root, folder)?;
    let children = fs::read_dir(folder).map_err(|err| at(err, "List folder", folder))?;
    let mut entries = Vec::new();
    for child in children {
        let child = child.map_err(|err| at(err, "List folder", folder))?;
        if hidden(&child.file_name()) {
            continue;
        }

        let path = child.path();
        let kind = child
            .file_type()
            .map_err(|err| at(err, "Read entry", &path))?;
        if kind.is_dir() || (kind.is_file() && markdown(&path)) {
            entries.push(Entry {
                path,
                is_dir: kind.is_dir(),
            });
        }
    }
    entries.sort_by_cached_key(|entry| {
        (
            !entry.is_dir,
            entry.name().to_string_lossy().to_lowercase(),
            entry.path.clone(),
        )
    });
    Ok(entries)
}

pub(crate) fn create(root: &Path, folder: &Path, name: &str) -> io::Result<PathBuf> {
    let name = valid_name(name, "note")?;
    check_folder(root, folder)?;
    let path = if markdown(Path::new(name)) {
        folder.join(name)
    } else {
        folder.join(format!("{name}.md"))
    };
    fs::File::options()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|err| at(err, "Create note", &path))?;
    Ok(path)
}

pub(crate) fn create_folder(root: &Path, folder: &Path, name: &str) -> io::Result<()> {
    let name = valid_name(name, "folder")?;
    check_folder(root, folder)?;
    let path = folder.join(name);
    fs::create_dir(&path).map_err(|error| at(error, "Create folder", &path))
}

fn valid_name<'a>(name: &'a str, kind: &str) -> io::Result<&'a str> {
    let name = name.trim();
    if name.is_empty()
        || hidden(name.as_ref())
        || name
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Enter a visible {kind} name without path separators or control characters"),
        ));
    }
    Ok(name)
}

pub(crate) fn read(root: &Path, path: &Path) -> io::Result<String> {
    check_note(root, path)?;
    fs::read_to_string(path).map_err(|err| at(err, "Read note", path))
}

pub(crate) fn note_modified(root: &Path, path: &Path) -> io::Result<SystemTime> {
    let (_, metadata) = check_note(root, path)?;
    metadata
        .modified()
        .map_err(|err| at(err, "Read modified date of", path))
}

pub(crate) fn save_draft_copy(root: &Path, note: &Path, text: &str) -> io::Result<PathBuf> {
    let folder = note
        .parent()
        .ok_or_else(|| io::Error::other("The note has no folder"))?;
    check_folder(root, folder)?;
    let mut pending = tempfile::Builder::new()
        .prefix(".xuy-draft-")
        .tempfile_in(folder)?;
    pending.write_all(text.as_bytes())?;
    pending.as_file().sync_all()?;
    let suffix = pending
        .path()
        .file_name()
        .and_then(OsStr::to_str)
        .and_then(|name| name.strip_prefix(".xuy-draft-"))
        .ok_or_else(|| io::Error::other("Could not name draft copy"))?;
    let stem = note
        .file_stem()
        .unwrap_or(note.as_os_str())
        .to_string_lossy();
    // Limit the prefix so a valid source filename still leaves room for the copy suffix.
    let stem = stem
        .char_indices()
        .take_while(|(byte, ch)| byte + ch.len_utf8() <= 190)
        .last()
        .map_or("Note", |(byte, ch)| &stem[..byte + ch.len_utf8()]);
    let path = folder.join(format!("{stem} (xuy draft {suffix}).md"));
    pending
        .persist_noclobber(&path)
        .map_err(|error| at(error.error, "Save draft as copy", &path))?;
    Ok(path)
}

pub(crate) fn save(root: &Path, path: &Path, original: &str, text: &str) -> io::Result<()> {
    let (folder, metadata) = check_note(root, path)?;
    let permissions = metadata.permissions();
    if permissions.readonly() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Note {} is read-only", path.display()),
        ));
    }

    let mut pending = tempfile::NamedTempFile::with_prefix_in(".xuy-", folder)
        .map_err(|err| at(err, "Prepare save for", path))?;
    pending
        .write_all(text.as_bytes())
        .map_err(|err| at(err, "Write note", path))?;
    pending
        .as_file()
        .set_permissions(permissions)
        .map_err(|err| at(err, "Preserve permissions for", path))?;
    pending
        .as_file()
        .sync_all()
        .map_err(|err| at(err, "Sync note", path))?;

    // shortcut: a write can race the final check and rename, use coordinated writes if needed across editors.
    if read(root, path)? != original {
        return Err(io::Error::other(format!(
            "Note {} changed on disk. Reopen it before saving",
            path.display()
        )));
    }
    pending
        .persist(path)
        .map_err(|err| at(err.error, "Replace note", path))?;
    Ok(())
}

fn check_folder(root: &Path, folder: &Path) -> io::Result<()> {
    if !folder.strip_prefix(root).is_ok_and(|relative| {
        relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Folder {} is outside the notebook", folder.display()),
        ));
    }

    let resolved = fs::canonicalize(folder).map_err(|err| at(err, "Open folder", folder))?;
    if resolved != folder {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Folder {} resolves through a symlink", folder.display()),
        ));
    }
    Ok(())
}

pub(crate) fn check_note<'a>(root: &Path, path: &'a Path) -> io::Result<(&'a Path, fs::Metadata)> {
    let invalid_note = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Expected a Markdown file at {}", path.display()),
        )
    };
    if !markdown(path) || path.file_name().is_some_and(hidden) {
        return Err(invalid_note());
    }
    let folder = path.parent().ok_or_else(&invalid_note)?;
    check_folder(root, folder)?;
    let metadata =
        fs::symlink_metadata(path).map_err(|err| at(err, "Read note metadata for", path))?;
    if !metadata.is_file() {
        return Err(invalid_note());
    }
    Ok((folder, metadata))
}

fn markdown(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

fn hidden(name: &OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b".")
}

fn at(err: io::Error, action: &str, path: &Path) -> io::Error {
    io::Error::new(err.kind(), format!("{action} {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_modification_time_even_for_invalid_markdown_contents() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let (root, _) = open_root(temp.path())?;
        let note = root.join("note.md");
        fs::write(&note, b"invalid \xFF")?;
        assert_eq!(
            note_modified(&root, &note)?,
            fs::metadata(&note)?.modified()?
        );
        Ok(())
    }

    #[test]
    fn lists_one_level_with_folders_first() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        fs::create_dir(temp.path().join("Projects"))?;
        fs::create_dir(temp.path().join("archive"))?;
        fs::create_dir(temp.path().join(".hidden"))?;
        for name in [
            "z.md",
            "B.MarkDown",
            "a-long.md",
            "a.MarkDown",
            ".hidden.md",
            "skip.txt",
            "Projects/inside.MD",
        ] {
            fs::write(temp.path().join(name), "")?;
        }

        let (root, entries) = open_root(temp.path())?;
        assert_eq!(root, fs::canonicalize(temp.path())?);
        let names: Vec<_> = entries
            .iter()
            .map(|entry| (entry.path.file_name().unwrap(), entry.is_dir))
            .collect();
        assert_eq!(
            names,
            [
                ("archive", true),
                ("Projects", true),
                ("a.MarkDown", false),
                ("a-long.md", false),
                ("B.MarkDown", false),
                ("z.md", false)
            ]
            .map(|(name, is_dir)| (OsStr::new(name), is_dir))
        );
        assert_eq!(
            list(&root, &root.join("Projects"))?,
            [Entry {
                path: root.join("Projects/inside.MD"),
                is_dir: false
            }]
        );
        Ok(())
    }

    #[test]
    fn creates_notes_exclusively_under_the_root() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let (root, _) = open_root(temp.path())?;
        let folder = root.join("projects");
        fs::create_dir(&folder)?;
        let note = create(&root, &folder, " First draft ")?;
        assert_eq!(note, folder.join("First draft.md"));
        assert_eq!(
            create(&root, &folder, "Index.MARKDOWN")?,
            folder.join("Index.MARKDOWN")
        );

        fs::write(&note, "original")?;
        assert_eq!(
            create(&root, &folder, "First draft.md").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(read(&root, &note)?, "original");

        for invalid in [
            "",
            " ",
            "..",
            "../outside",
            "part/name",
            "part\\name",
            "part:name",
            ".hidden",
            "a\0b",
        ] {
            assert_eq!(
                create(&root, &folder, invalid).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        assert_eq!(
            list(&root, &root.join("projects/..")).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            create(&root, root.parent().unwrap(), "outside")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        Ok(())
    }

    #[test]
    fn creates_named_folders_without_overwriting_or_leaving_the_root() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let (root, _) = open_root(temp.path())?;
        create_folder(&root, &root, " Drafts ")?;
        create_folder(&root, &root.join("Drafts"), "Nested")?;
        assert!(root.join("Drafts/Nested").is_dir());
        assert_eq!(
            create_folder(&root, &root, "Drafts").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(create_folder(&root, &root, "../Outside").is_err());
        assert!(create_folder(&root, root.parent().unwrap(), "Outside").is_err());
        Ok(())
    }

    #[test]
    fn preserves_external_edits_on_save_conflict() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let (root, _) = open_root(temp.path())?;
        let note = create(&root, &root, "Draft")?;
        assert_eq!(read(&root, &note)?, "");
        save(&root, &note, "", "# Draft\n")?;
        assert_eq!(read(&root, &note)?, "# Draft\n");

        fs::write(&note, "outside change")?;
        let err = save(&root, &note, "# Draft\n", "my change").unwrap_err();
        assert!(err.to_string().contains("changed on disk"));
        assert_eq!(read(&root, &note)?, "outside change");
        assert_eq!(fs::read_dir(&root)?.count(), 1);
        Ok(())
    }

    #[test]
    fn draft_copy_preserves_original_and_allocates_a_new_path_each_time() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let (root, _) = open_root(temp.path())?;
        let note = create(&root, &root, "Page")?;
        fs::write(&note, "external")?;
        let first = save_draft_copy(&root, &note, "draft one")?;
        let second = save_draft_copy(&root, &note, "draft two")?;
        assert_ne!(first, second);
        assert_eq!(read(&root, &note)?, "external");
        assert_eq!(read(&root, &first)?, "draft one");
        assert_eq!(read(&root, &second)?, "draft two");
        assert!(
            save_draft_copy(&root, &root.parent().unwrap().join("outside.md"), "copy").is_err()
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn ignores_symlinks_and_rejects_direct_access_through_them() -> io::Result<()> {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let (root, _) = open_root(temp.path())?;
        let outside_note = outside.path().join("original.md");
        fs::write(&outside_note, "outside")?;
        let folder_link = root.join("folder-link");
        let note_link = root.join("note-link.md");
        symlink(outside.path(), &folder_link)?;
        symlink(&outside_note, &note_link)?;

        assert!(list(&root, &root)?.is_empty());
        assert!(list(&root, &folder_link).is_err());
        assert!(create(&root, &folder_link, "other").is_err());
        assert!(read(&root, &note_link).is_err());
        assert!(read(&root, &folder_link.join("original.md")).is_err());
        assert!(note_modified(&root, &note_link).is_err());
        assert!(save(&root, &note_link, "outside", "replacement").is_err());
        assert_eq!(fs::read_to_string(&outside_note)?, "outside");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn save_preserves_file_permissions() -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir()?;
        let (root, _) = open_root(temp.path())?;
        let note = create(&root, &root, "Draft")?;
        fs::set_permissions(&note, fs::Permissions::from_mode(0o640))?;
        save(&root, &note, "", "new text")?;
        assert_eq!(fs::metadata(&note)?.permissions().mode() & 0o777, 0o640);

        fs::set_permissions(&note, fs::Permissions::from_mode(0o440))?;
        assert_eq!(
            save(&root, &note, "new text", "replacement")
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(read(&root, &note)?, "new text");
        Ok(())
    }
}
