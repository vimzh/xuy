//! Back and forward navigation for notes opened in this notebook.

use std::path::{Path, PathBuf};

#[derive(Default)]
pub(crate) struct NoteHistory {
    paths: Vec<PathBuf>,
    current: usize,
}

impl NoteHistory {
    pub(crate) fn visit(&mut self, path: &Path) {
        if self
            .paths
            .get(self.current)
            .is_some_and(|current| current == path)
        {
            return;
        }
        self.paths.truncate(self.current + 1);
        self.current = self.paths.len();
        self.paths.push(path.to_owned());
    }

    pub(crate) fn destination(&self, offset: isize) -> Option<(usize, &Path)> {
        let index = self.current.checked_add_signed(offset)?;
        Some((index, self.paths.get(index)?.as_path()))
    }

    pub(crate) fn arrive(&mut self, index: usize) {
        self.current = index;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_notes_and_prunes_forward_history_after_a_new_visit() {
        let mut history = NoteHistory::default();
        assert!(history.destination(-1).is_none());
        assert!(history.destination(1).is_none());
        history.visit(Path::new("first.md"));
        history.visit(Path::new("second.md"));
        history.visit(Path::new("second.md"));

        assert_eq!(history.destination(-1), Some((0, Path::new("first.md"))));
        history.arrive(0);
        assert_eq!(history.destination(1), Some((1, Path::new("second.md"))));

        history.visit(Path::new("nested/third.md"));
        assert_eq!(history.destination(-1), Some((0, Path::new("first.md"))));
        assert!(history.destination(1).is_none());
    }
}
