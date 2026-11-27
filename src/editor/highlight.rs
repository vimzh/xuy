//! Recognizes and edits the five inline highlight marks stored in Markdown.

use std::{fmt::Write, ops::Range};

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use crate::markdown::OPTIONS as MARKDOWN_OPTIONS;

// Unquoted class tokens keep macOS smart punctuation out of generated tags.
const PREFIX: &str = "<mark class=xuy-";
pub(crate) const CLOSE: &str = "</mark>";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Color {
    Yellow,
    Mint,
    Sky,
    Rose,
    Lavender,
}

impl Color {
    pub(crate) const ALL: [Self; 5] = [
        Self::Yellow,
        Self::Mint,
        Self::Sky,
        Self::Rose,
        Self::Lavender,
    ];

    pub(crate) fn name(self) -> &'static str {
        ["Yellow", "Mint", "Sky", "Rose", "Lavender"][self as usize]
    }

    pub(crate) fn class(self) -> &'static str {
        [
            "xuy-yellow",
            "xuy-mint",
            "xuy-sky",
            "xuy-rose",
            "xuy-lavender",
        ][self as usize]
    }

    fn open(self) -> &'static str {
        [
            "<mark class=xuy-yellow>",
            "<mark class=xuy-mint>",
            "<mark class=xuy-sky>",
            "<mark class=xuy-rose>",
            "<mark class=xuy-lavender>",
        ][self as usize]
    }
}

pub(crate) struct Mark {
    open: Range<usize>,
    close: Range<usize>,
    color: Color,
}

impl Mark {
    fn touches(&self, selection: &Range<usize>) -> bool {
        if selection.is_empty() {
            (self.open.end..=self.close.start).contains(&selection.start)
                || selection.start == self.close.end
        } else {
            selection.start < self.close.end && self.open.start < selection.end
        }
    }
}

pub(crate) fn marks(source: &str) -> Vec<Mark> {
    if !source.contains(PREFIX) {
        return Vec::new();
    }
    let mut marks = Vec::new();
    let mut opened: Option<(Color, Range<usize>, usize)> = None;
    let mut depth = 0;
    for (event, range) in Parser::new_ext(source, MARKDOWN_OPTIONS).into_offset_iter() {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if opened.as_ref().is_some_and(|(_, _, at)| *at > depth) {
                    opened = None;
                }
            }
            Event::InlineHtml(html) if html.as_ref() == CLOSE => {
                if let Some((color, open, at)) = opened.take()
                    && at == depth
                    && range.start > open.end
                {
                    marks.push(Mark {
                        color,
                        open,
                        close: range,
                    });
                }
            }
            Event::InlineHtml(html)
                if html
                    .get(..5)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<mark")) =>
            {
                opened = Color::ALL
                    .into_iter()
                    .find(|color| color.open() == html.as_ref())
                    .map(|color| (color, range, depth));
            }
            _ => {}
        }
    }
    marks
}

pub(crate) fn mark_offsets(marks: &[Mark]) -> impl Iterator<Item = usize> {
    marks
        .iter()
        .flat_map(|mark| [mark.open.start, mark.close.start])
}

#[derive(PartialEq)]
pub(crate) struct Passage {
    pub(crate) range: Range<usize>,
    pub(crate) color: Color,
    pub(crate) text: String,
}

pub(crate) fn passages(source: &str, marks: &[Mark]) -> Vec<Passage> {
    let (mut byte, mut offset) = (0, 0);
    marks
        .iter()
        .map(|mark| {
            offset += source[byte..mark.open.end].encode_utf16().count();
            let start = offset;
            offset += source[mark.open.end..mark.close.start]
                .encode_utf16()
                .count();
            byte = mark.close.start;
            Passage {
                range: start..offset,
                color: mark.color,
                text: String::new(),
            }
        })
        .collect()
}

pub(crate) fn editor_ranges(passages: &[Passage]) -> Option<String> {
    if passages.is_empty() {
        return None;
    }
    let mut result = String::from("[");
    for (index, passage) in passages.iter().enumerate() {
        if index != 0 {
            result.push(',');
        }
        write!(
            result,
            "[{},{},\"{}\"]",
            passage.range.start,
            passage.range.end,
            passage.color.class()
        )
        .unwrap();
    }
    result.push(']');
    Some(result)
}

pub(crate) fn strip_rendered_marks(mut html: String) -> String {
    for color in Color::ALL {
        html = html.replace(color.open(), "");
    }
    html.replace(CLOSE, "")
}

pub(crate) struct Edit {
    pub(crate) range: Range<usize>,
    pub(crate) replacement: String,
}

pub(crate) fn edit(
    source: &str,
    start: usize,
    end: usize,
    color: Option<Color>,
) -> Result<Option<Edit>, &'static str> {
    let selection = match (byte_at(source, start), byte_at(source, end)) {
        (Some(start), Some(end)) if start <= end => start..end,
        _ => return Err("Select the text again to change its highlight."),
    };
    let marks = marks(source);
    let mut changes = Vec::new();
    for mark in &marks {
        if mark.touches(&selection) && color != Some(mark.color) {
            changes.push((mark.open.clone(), color.map_or("", Color::open).to_owned()));
            if color.is_none() {
                changes.push((mark.close.clone(), String::new()));
            }
        }
    }

    if let Some(color) = color
        && !selection.is_empty()
    {
        let mut excluded = 0;
        let mut marks = marks.iter().peekable();
        for (event, range) in Parser::new_ext(source, MARKDOWN_OPTIONS).into_offset_iter() {
            if range.start >= selection.end && matches!(event, Event::Text(_)) {
                break;
            }
            match event {
                Event::Start(Tag::CodeBlock(_) | Tag::Link { .. } | Tag::Image { .. }) => {
                    excluded += 1
                }
                Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image) => excluded -= 1,
                Event::Text(_) if excluded == 0 => {
                    let start = range.start.max(selection.start);
                    let end = range.end.min(selection.end);
                    if start >= end {
                        continue;
                    }
                    while marks.peek().is_some_and(|mark| mark.close.end <= start) {
                        marks.next();
                    }
                    if marks.peek().is_some_and(|mark| mark.open.start < end) {
                        continue;
                    }
                    let picked = &source[start..end];
                    let trimmed = picked.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let start = start + (picked.len() - picked.trim_start().len());
                    changes.push((
                        start..start + trimmed.len(),
                        format!("{}{trimmed}{CLOSE}", color.open()),
                    ));
                }
                _ => {}
            }
        }
    }

    if changes.is_empty() {
        return if color.is_none() || marks.iter().any(|mark| mark.touches(&selection)) {
            Ok(None)
        } else {
            Err("Select regular Markdown text to highlight it.")
        };
    }
    changes.sort_unstable_by_key(|(range, _)| range.start);
    let start = changes[0].0.start;
    let end = changes.last().unwrap().0.end;
    let mut replacement = source[start..end].to_owned();
    for (range, value) in changes.into_iter().rev() {
        replacement.replace_range(range.start - start..range.end - start, &value);
    }
    Ok(Some(Edit {
        range: start..end,
        replacement,
    }))
}

fn byte_at(source: &str, target: usize) -> Option<usize> {
    let mut offset = 0;
    for (byte, ch) in source.char_indices() {
        if offset == target {
            return Some(byte);
        }
        offset += ch.len_utf16();
        if offset > target {
            return None;
        }
    }
    (offset == target).then_some(source.len())
}
