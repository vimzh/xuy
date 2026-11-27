//! Apply edits to the source ranges behind rendered Markdown text.

use std::ops::Range;

pub(crate) struct EditableText {
    range: Range<usize>,
    enclosing: Range<usize>,
    rendered: String,
    multiline: bool,
}

impl EditableText {
    pub(crate) fn new(
        range: Range<usize>,
        enclosing: Range<usize>,
        rendered: &str,
        multiline: bool,
    ) -> Self {
        Self {
            range,
            enclosing,
            rendered: rendered.to_owned(),
            multiline,
        }
    }
}

pub(crate) struct PreviewEdits {
    pub(crate) generation: u64,
    pub(crate) draft: String,
    ranges: Vec<Range<usize>>,
}

impl PreviewEdits {
    pub(crate) fn new(generation: u64, draft: &str, text: &[EditableText]) -> Self {
        Self {
            generation,
            draft: draft.to_owned(),
            ranges: text.iter().map(|run| run.enclosing.clone()).collect(),
        }
    }

    pub(crate) fn replace(
        &mut self,
        original: &str,
        text: &[EditableText],
        index: usize,
        value: &str,
    ) -> Result<bool, &'static str> {
        let Some(entry) = text.get(index) else {
            return Err("Could not find the text to edit.");
        };
        let original_text = &original[entry.range.clone()];
        let value = value.replace("\r\n", "\n").replace('\r', "\n");
        let value = value.trim_end_matches('\n');
        if !entry.multiline && value.contains('\n') {
            return Err("Use Markdown source to add lines inside formatted text.");
        }
        let mut replacement = if value == entry.rendered {
            original_text.to_owned()
        } else if original_text == entry.rendered {
            replace_plain_text(original_text, value)
        } else {
            literal_markdown(value, "")
        };
        if entry.enclosing != entry.range && !value.trim().is_empty() {
            // Markdown emphasis delimiters cannot touch leading or trailing spaces.
            if replacement.ends_with(' ') {
                replacement.pop();
                replacement.push_str("&#32;");
            }
            if replacement.starts_with(' ') {
                replacement.replace_range(..1, "&#32;");
            }
            replacement = format!(
                "{}{replacement}{}",
                &original[entry.enclosing.start..entry.range.start],
                &original[entry.range.end..entry.enclosing.end]
            );
        }
        if original.contains("\r\n") && replacement.contains('\n') {
            replacement = replacement.replace('\n', "\r\n");
        }
        let range = self.ranges[index].clone();
        if self.draft[range.clone()] == replacement {
            return Ok(false);
        }
        let end = range.start + replacement.len();
        self.draft.replace_range(range.clone(), &replacement);
        self.ranges[index].end = end;
        for later in &mut self.ranges[index + 1..] {
            *later = end + (later.start - range.end)..end + (later.end - range.end);
        }
        Ok(true)
    }
}

fn replace_plain_text(old: &str, new: &str) -> String {
    let prefix = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let suffix = old[prefix..]
        .chars()
        .rev()
        .zip(new[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    format!(
        "{}{}{}",
        &old[..prefix],
        literal_markdown(&new[prefix..new.len() - suffix], &old[..prefix]),
        &old[old.len() - suffix..]
    )
}

fn literal_markdown(text: &str, prefix: &str) -> String {
    let mut markdown = String::with_capacity(text.len());
    let before = prefix.rsplit('\n').next().unwrap().trim();
    let mut number = !before.is_empty() && before.bytes().all(|byte| byte.is_ascii_digit());
    let mut line_start = before.is_empty();
    for character in text.chars() {
        if character == '\n' {
            markdown.push('\n');
            line_start = true;
            number = false;
        } else if matches!(
            character,
            '\\' | '`'
                | '*'
                | '_'
                | '['
                | ']'
                | '!'
                | '<'
                | '>'
                | '#'
                | '&'
                | '|'
                | '~'
                | '+'
                | '-'
                | '='
                | ':'
        ) || number && matches!(character, '.' | ')')
        {
            markdown.push('\\');
            line_start = false;
            number = false;
        } else if character.is_ascii_digit() {
            number |= line_start;
            line_start = false;
        } else if line_start && character == ' ' {
            markdown.push_str("&#32;");
            continue;
        } else if !character.is_ascii_whitespace() || !line_start {
            line_start = false;
            number = false;
        }
        markdown.push(character);
    }
    markdown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_text_at_original_ranges_and_retains_surrounding_markdown() {
        let original = "**💬** and [link][id].\n\n[id]: https://example.org";
        let text = [
            EditableText::new(2..6, 0..8, "💬", false),
            EditableText::new(8..13, 8..13, " and ", false),
        ];
        let mut edit = PreviewEdits::new(1, original, &text);
        assert_eq!(edit.replace(original, &text, 0, "new * text"), Ok(true));
        assert_eq!(edit.replace(original, &text, 1, " then "), Ok(true));
        assert_eq!(
            edit.draft,
            "**new \\* text** then [link][id].\n\n[id]: https://example.org"
        );
        assert_eq!(edit.replace(original, &text, 0, "💬"), Ok(true));
        assert_eq!(
            edit.draft,
            "**💬** then [link][id].\n\n[id]: https://example.org"
        );
        assert_eq!(edit.replace(original, &text, 0, ""), Ok(true));
        assert_eq!(edit.draft, " then [link][id].\n\n[id]: https://example.org");
    }

    #[test]
    fn appends_paragraphs_without_introducing_markdown_punctuation_or_trailing_lines() {
        let original = "A paragraph.";
        let length = original.len();
        let runs = [EditableText::new(0..length, 0..length, original, true)];
        let mut edit = PreviewEdits::new(0, original, &runs);
        assert_eq!(
            edit.replace(
                original,
                &runs,
                0,
                "A paragraph.\nA sentence.\n1. Literal\n"
            ),
            Ok(true)
        );
        assert_eq!(edit.draft, "A paragraph.\n\nA sentence.\n\n1\\. Literal");
    }
}
