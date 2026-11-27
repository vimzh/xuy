//! Copies Finder drops beside the note and inserts relative Markdown image links.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use dioxus::desktop::DesktopContext;
use dioxus::prelude::*;
use tempfile::TempPath;

use crate::markdown_paths::{MarkdownPaths, read_image};

pub(super) async fn insert(
    paths: MarkdownPaths,
    sources: Vec<PathBuf>,
    client_y: f64,
    desktop: DesktopContext,
) -> Result<(), String> {
    let mut command = document::eval(include_str!("image_drop.js"));
    let unavailable = || "The note changed. Drop the images again in Edit.".to_owned();
    command.send(client_y).map_err(|_| unavailable())?;
    if !matches!(command.recv::<bool>().await, Ok(true)) {
        return Err(unavailable());
    }

    let images = tokio::task::spawn_blocking(move || copy_images(&paths, &sources))
        .await
        .map_err(|error| format!("Could not import images. {error}"))
        .and_then(|result| result.map_err(|error| format!("Could not add images. {error}")));
    let (mut files, markdown) = match images {
        Ok(images) => images,
        Err(error) => {
            let _ = command.send(());
            return Err(error);
        }
    };

    #[cfg(target_os = "macos")]
    let undo_manager = {
        use dioxus::desktop::wry::WebViewExtMacOS;
        desktop.webview.webview().undoManager()
    };
    #[cfg(target_os = "macos")]
    let _undo_group = undo_manager
        .as_deref()
        .and_then(super::EditorUndoGroup::begin);
    #[cfg(target_os = "macos")]
    if _undo_group.is_none() {
        let _ = command.send(());
        return Err("Could not start an undo step. Drop the images again.".to_owned());
    }
    #[cfg(not(target_os = "macos"))]
    let _ = desktop;

    command.send(markdown).map_err(|_| unavailable())?;
    if !matches!(command.recv::<bool>().await, Ok(true)) {
        return Err(unavailable());
    }
    for file in &mut files {
        file.disable_cleanup(true);
    }
    // Let the input event reach autosave only once the image files are retained.
    command.send(true).map_err(|_| unavailable())
}

fn copy_images(paths: &MarkdownPaths, sources: &[PathBuf]) -> io::Result<(Vec<TempPath>, String)> {
    let mut files = Vec::with_capacity(sources.len());
    let mut links = Vec::with_capacity(sources.len());
    for source in sources {
        let (file, link) = copy_image(paths, source).map_err(|error| {
            let name = source.file_name().unwrap_or(source.as_os_str());
            io::Error::new(error.kind(), format!("{}: {error}", name.to_string_lossy()))
        })?;
        files.push(file);
        links.push(link);
    }
    Ok((files, links.join("\n\n")))
}

fn copy_image(paths: &MarkdownPaths, source: &Path) -> io::Result<(TempPath, String)> {
    if !source.is_absolute() {
        return Err(io::Error::other("Drop a local image file from Finder"));
    }
    let mut options = fs::File::options();
    options.read(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let (kind, bytes) = read_image(options.open(source)?)?;
    let folder = paths.attachment_folder()?;
    let mut pending = tempfile::Builder::new()
        .prefix("xuy-image-")
        .suffix(kind.extension())
        .tempfile_in(folder)?;
    pending.write_all(&bytes)?;
    pending.as_file().sync_all()?;

    let label = source.file_stem().unwrap_or_default().to_string_lossy();
    let label: String = label
        .chars()
        .take(80)
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '-' {
                ch
            } else {
                ' '
            }
        })
        .collect();
    let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    let label = if label.is_empty() { "Image" } else { &label };
    let name = pending
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::other("Could not name the image copy"))?;
    let link = format!("![{label}](./{name})");
    Ok((pending.into_temp_path(), link))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_drops_beside_nested_note_and_renders_each_on_its_own_block() -> io::Result<()> {
        let notebook = tempfile::tempdir()?;
        let root = notebook.path().canonicalize()?;
        let folder = root.join("notes");
        fs::create_dir(&folder)?;
        let note = folder.join("page.md");
        fs::write(&note, "")?;
        let source = root.join("Example [1].gif");
        fs::write(&source, b"GIF89a")?;
        let paths = MarkdownPaths::new(root, note);

        let (copies, markdown) = copy_images(&paths, &[source.clone(), source.clone()])?;
        assert_ne!(&*copies[0], &*copies[1]);
        for copy in &copies {
            assert_eq!(copy.parent(), Some(folder.as_path()));
            assert_eq!(fs::read(copy)?, fs::read(&source)?);
        }
        let html = super::super::render_preview(&markdown, Some(&paths));
        assert_eq!(html.matches("<p><img ").count(), 2);
        assert_eq!(html.matches(r#"src="data:image/gif;base64,"#).count(), 2);
        assert!(html.contains(r#"alt="Example 1""#));
        Ok(())
    }

    #[test]
    fn removes_partial_copies_if_any_dropped_file_is_unsupported() -> io::Result<()> {
        let notebook = tempfile::tempdir()?;
        let root = notebook.path().canonicalize()?;
        let note = root.join("page.md");
        let valid = root.join("valid.gif");
        let invalid = root.join("invalid.png");
        fs::write(&note, "")?;
        fs::write(&valid, b"GIF89a")?;
        fs::write(&invalid, b"not an image")?;
        let paths = MarkdownPaths::new(root.clone(), note);

        let error = copy_images(&paths, &[valid, invalid]).unwrap_err();
        assert!(error.to_string().contains("invalid.png"));
        assert_eq!(fs::read_dir(root)?.count(), 3);
        Ok(())
    }
}
