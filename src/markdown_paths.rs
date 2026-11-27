//! Resolves local Markdown destinations inside the current notebook.

use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use base64::{Engine, engine::general_purpose::STANDARD};
use percent_encoding::percent_decode_str;

const MAX_IMAGE_BYTES: u64 = 12 * 1024 * 1024;

#[derive(Clone, PartialEq)]
pub(crate) struct MarkdownPaths {
    root: PathBuf,
    note: PathBuf,
}

impl MarkdownPaths {
    pub(crate) fn new(root: PathBuf, note: PathBuf) -> Self {
        Self { root, note }
    }

    pub(crate) fn resolve_note(
        &self,
        destination: &str,
        wiki: bool,
    ) -> io::Result<(PathBuf, Option<String>)> {
        let (destination, fragment) = destination
            .split_once('#')
            .map_or((destination, None), |(path, fragment)| {
                (path, Some(fragment))
            });
        let path = if destination.is_empty() && fragment.is_some() {
            let name = self
                .note
                .file_name()
                .ok_or_else(|| invalid("The current note has no file name"))?;
            self.relative_file(Path::new(name))?
        } else {
            self.relative_file(Path::new(decode_relative(destination)?.as_ref()))?
        };
        let fragment = fragment
            .filter(|fragment| !fragment.is_empty())
            .map(decode)
            .transpose()?
            .map(Cow::into_owned);

        let extension = path.extension().and_then(OsStr::to_str);
        if extension.is_some_and(|ext| {
            ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
        }) {
            regular_file(&path)?;
            return Ok((path, fragment));
        }
        if !wiki || extension.is_some() {
            return Err(invalid("Use a .md or .markdown file for a note link"));
        }

        let md = path.with_extension("md");
        match regular_file(&md) {
            Ok(_) => Ok((md, fragment)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let markdown = path.with_extension("markdown");
                regular_file(&markdown).map_err(|error| {
                    if error.kind() == io::ErrorKind::NotFound {
                        io::Error::new(
                            error.kind(),
                            format!(
                                "No note found at {} or {}",
                                md.display(),
                                markdown.display()
                            ),
                        )
                    } else {
                        error
                    }
                })?;
                Ok((markdown, fragment))
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn image_source(&self, destination: &str) -> Option<String> {
        let path = destination
            .split_once('#')
            .map_or(destination, |(path, _)| path);
        let path = self
            .relative_file(Path::new(decode_relative(path).ok()?.as_ref()))
            .ok()?;
        if regular_file(&path).ok()?.len() > MAX_IMAGE_BYTES {
            return None;
        }

        let mut options = fs::File::options();
        options.read(true);
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // Keep path replacements from redirecting the read or blocking on a FIFO.
            options.custom_flags(libc::O_NOFOLLOW_ANY | libc::O_NONBLOCK);
        }
        let file = options.open(path).ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() || metadata.len() > MAX_IMAGE_BYTES {
            return None;
        }
        let mut image = file.take(MAX_IMAGE_BYTES + 1);
        let mut bytes = Vec::with_capacity(12);
        image.by_ref().take(12).read_to_end(&mut bytes).ok()?;
        let mime = match bytes.as_slice() {
            [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', ..] => "image/png",
            [0xff, 0xd8, 0xff, ..] => "image/jpeg",
            [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => "image/gif",
            header if header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WEBP") => {
                "image/webp"
            }
            _ => return None,
        };
        bytes.reserve((metadata.len() as usize).saturating_sub(bytes.len()));
        image.read_to_end(&mut bytes).ok()?;
        if bytes.len() as u64 > MAX_IMAGE_BYTES {
            return None;
        }
        let mut source = format!("data:{mime};base64,");
        STANDARD.encode_string(bytes, &mut source);
        Some(source)
    }

    fn relative_file(&self, relative: &Path) -> io::Result<PathBuf> {
        let mut parts = relative.components();
        let Some(Component::Normal(name)) = parts.next_back() else {
            return Err(invalid("The link must name a file"));
        };
        check_visible(name)?;
        let source_folder = self
            .note
            .parent()
            .and_then(|path| path.strip_prefix(&self.root).ok())
            .ok_or_else(|| invalid("The current note is outside the notebook"))?;
        let root = self
            .root
            .canonicalize()
            .map_err(|error| at(error, "Open notebook folder", &self.root))?;
        if root != self.root {
            return Err(invalid("The notebook folder moved. Reopen it to continue."));
        }

        let mut folder = self.root.clone();
        for component in source_folder.components().chain(parts) {
            match component {
                Component::CurDir => {}
                Component::ParentDir if folder != self.root => {
                    folder.pop();
                }
                Component::Normal(name) => {
                    check_visible(name)?;
                    folder.push(name);
                    let metadata = fs::symlink_metadata(&folder)
                        .map_err(|error| at(error, "Open linked folder", &folder))?;
                    if !metadata.is_dir() {
                        return Err(invalid(format!(
                            "Expected a folder without symlinks at {}",
                            folder.display()
                        )));
                    }
                }
                _ => return Err(invalid("The link points outside the notebook")),
            }
        }
        Ok(folder.join(name))
    }
}

fn decode(value: &str) -> io::Result<Cow<'_, str>> {
    percent_decode_str(value)
        .decode_utf8()
        .map_err(|_| invalid("The link contains invalid UTF-8"))
}

fn decode_relative(value: &str) -> io::Result<Cow<'_, str>> {
    let decoded = decode(value)?;
    if decoded.starts_with('/')
        || decoded.contains(['\\', ':'])
        || decoded.chars().any(char::is_control)
    {
        return Err(invalid("Use a relative path inside the notebook"));
    }
    if decoded.ends_with('/') || decoded.ends_with("/.") {
        return Err(invalid("The link must name a file"));
    }
    Ok(decoded)
}

fn check_visible(name: &OsStr) -> io::Result<()> {
    if name.as_encoded_bytes().starts_with(b".") {
        return Err(invalid("Links to hidden files and folders are unavailable"));
    }
    Ok(())
}

fn regular_file(path: &Path) -> io::Result<fs::Metadata> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| at(error, "Open linked file", path))?;
    if !metadata.is_file() {
        return Err(invalid(format!(
            "Expected a regular file without symlinks at {}",
            path.display()
        )));
    }
    Ok(metadata)
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn at(error: io::Error, action: &str, path: &Path) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("{action} {}: {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_encoded_note_paths_and_sections_relative_to_the_note() -> io::Result<()> {
        let notebook = tempfile::tempdir()?;
        let root = notebook.path().canonicalize()?;
        let note = root.join("folder/Active.md");
        fs::create_dir_all(root.join("folder/research"))?;
        fs::write(&note, "")?;
        fs::write(root.join("folder/research/Café #1.markdown"), "")?;
        fs::write(root.join("Literal%20Space.md"), "")?;
        let paths = MarkdownPaths::new(root.clone(), note.clone());

        assert_eq!(
            paths.resolve_note(
                "research/Caf%C3%A9%20%231#Cr%C3%A8me%20br%C3%BBl%C3%A9e",
                true
            )?,
            (
                root.join("folder/research/Café #1.markdown"),
                Some("Crème brûlée".to_owned())
            )
        );
        assert_eq!(
            paths.resolve_note("../Literal%2520Space%2Emd", false)?,
            (root.join("Literal%20Space.md"), None)
        );
        assert_eq!(
            paths.resolve_note("#Intro", false)?,
            (note, Some("Intro".to_owned()))
        );
        assert!(paths.resolve_note("Active", false).is_err());
        let missing = paths.resolve_note("research/Absent", true).unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        assert!(missing.to_string().contains("Absent.md or"));
        assert!(missing.to_string().contains("Absent.markdown"));
        Ok(())
    }

    #[test]
    fn rejects_traversal_hidden_paths_and_nonrelative_destinations() -> io::Result<()> {
        let notebook = tempfile::tempdir()?;
        let root = notebook.path().canonicalize()?;
        fs::create_dir_all(root.join("folder/.hidden"))?;
        fs::write(root.join("folder/.hidden/Note.md"), "")?;
        fs::write(root.join("folder/.hidden.md"), "")?;
        let paths = MarkdownPaths::new(root.clone(), root.join("folder/Active.md"));

        for destination in [
            "../../Note.md",
            "%2e%2e/%2e%2e/Note.md",
            "/Note.md",
            "%2F%2Fhost/Note.md",
            "https://host/Note.md",
            "file:Note.md",
            "C:%5cNote.md",
            "folder%5cNote.md",
            "%2Ehidden/Note.md",
            ".hidden.md",
        ] {
            let error = paths.resolve_note(destination, false).unwrap_err();
            assert_eq!(
                error.kind(),
                io::ErrorKind::InvalidInput,
                "{destination}: {error}"
            );
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_files_and_folders_even_inside_the_notebook() -> io::Result<()> {
        use std::os::unix::fs::symlink;

        let notebook = tempfile::tempdir()?;
        let root = notebook.path().canonicalize()?;
        fs::create_dir(root.join("folder"))?;
        fs::write(root.join("folder/Note.md"), "")?;
        fs::write(root.join("folder/image.png"), b"\x89PNG\r\n\x1a\n")?;
        symlink(root.join("folder"), root.join("alias"))?;
        symlink(root.join("folder/Note.md"), root.join("Note.md"))?;
        symlink(root.join("folder/image.png"), root.join("image.png"))?;
        let paths = MarkdownPaths::new(root.clone(), root.join("Active.md"));

        assert!(paths.resolve_note("alias/Note.md", false).is_err());
        assert!(
            paths
                .resolve_note("alias/../folder/Note.md", false)
                .is_err()
        );
        assert!(paths.resolve_note("Note.md", false).is_err());
        assert!(paths.image_source("alias/image.png").is_none());
        assert!(paths.image_source("image.png").is_none());
        Ok(())
    }

    #[test]
    fn embeds_only_supported_raster_files_within_the_size_limit() -> io::Result<()> {
        let notebook = tempfile::tempdir()?;
        let root = notebook.path().canonicalize()?;
        fs::create_dir(root.join("images"))?;
        let png = b"\x89PNG\r\n\x1a\n";
        fs::write(root.join("images/Local image.png"), png)?;
        fs::write(root.join("fake.png"), b"<svg/>")?;
        let large = root.join("large.png");
        fs::write(&large, png)?;
        fs::File::options()
            .write(true)
            .open(&large)?
            .set_len(MAX_IMAGE_BYTES + 1)?;
        let paths = MarkdownPaths::new(root.clone(), root.join("Active.md"));

        assert_eq!(
            paths.image_source("images/Local%20image.png").as_deref(),
            Some("data:image/png;base64,iVBORw0KGgo=")
        );
        assert!(paths.image_source("fake.png").is_none());
        assert!(paths.image_source("large.png").is_none());
        assert!(
            paths
                .image_source("https://example.com/image.png")
                .is_none()
        );
        Ok(())
    }
}
