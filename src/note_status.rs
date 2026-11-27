//! The active note's heading path and word count in the workspace footer.

use dioxus::prelude::{dioxus_core::use_drop, *};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use crate::highlight;
use crate::markdown::{OPTIONS as MARKDOWN_OPTIONS, append_plain_text};

#[component]
pub(crate) fn NoteStatusBar(filename: String, summary: Memo<NoteSummary>) -> Element {
    let mut caret = use_signal(|| 0);
    let mut bridge = use_signal(|| None::<document::Eval>);
    use_effect(move || {
        let mut eval = document::eval(include_str!("note_caret.js"));
        bridge.set(Some(eval));
        spawn(async move {
            while let Ok(offset) = eval.recv::<usize>().await {
                if offset != *caret.peek() {
                    caret.set(offset);
                }
            }
        });
    });
    use_drop(move || {
        if let Some(eval) = *bridge.peek() {
            let _ = eval.send(());
        }
    });

    let heading = use_memo(move || summary.read().heading_at(caret()));
    let summary = summary.read();
    let path = summary.heading_path(heading());
    let count = summary.words;
    let unit = if count == 1 { "word" } else { "words" };

    rsx! {
        footer { class: "note-status-bar", role: "group", aria_label: "Note status",
            span { class: "note-status-name", title: "{filename}", "{filename}" }
            span { aria_hidden: true, "›" }
            span { class: "note-status-path", title: "{path}", "{path}" }
            span { class: "note-status-words", "{count} {unit}" }
        }
    }
}

#[derive(Default, PartialEq)]
pub(crate) struct NoteSummary {
    words: usize,
    pub(crate) headings: Vec<Heading>,
    pub(crate) highlights: Vec<highlight::Passage>,
    highlight_ranges: Option<String>,
}

#[derive(PartialEq)]
pub(crate) struct Heading {
    pub(crate) offset: usize,
    pub(crate) depth: usize,
    title: String,
    parent: Option<usize>,
}

impl Heading {
    pub(crate) fn title(&self) -> &str {
        let title = self.title.trim();
        if title.is_empty() {
            "Untitled heading"
        } else {
            title
        }
    }
}

impl NoteSummary {
    pub(crate) fn parse(source: &str) -> Self {
        // Textarea offsets count UTF-16 code units after line-ending normalization.
        let normalized;
        let source = if source.contains('\r') {
            normalized = source.replace("\r\n", "\n").replace('\r', "\n");
            &normalized
        } else {
            source
        };

        let mut words = 0;
        let mut in_word = false;
        let mut in_heading = false;
        let mut headings: Vec<Heading> = Vec::new();
        let mut active: [Option<usize>; 6] = [None; 6];
        let mut last_byte = 0;
        let mut offset = 0;
        let marks = highlight::marks(source);
        let mut mark_offsets = highlight::mark_offsets(&marks).peekable();
        let mut highlights = highlight::passages(source, &marks);
        let mut remaining = highlights.iter_mut();
        let mut active_highlight: Option<&mut highlight::Passage> = None;

        for (event, range) in Parser::new_ext(source, MARKDOWN_OPTIONS).into_offset_iter() {
            if matches!(event, Event::InlineHtml(_))
                && mark_offsets.next_if_eq(&range.start).is_some()
            {
                if active_highlight.take().is_none() {
                    active_highlight = remaining.next();
                }
                continue;
            }
            if let Some(passage) = active_highlight.as_deref_mut() {
                append_plain_text(&mut passage.text, &event);
            }
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    let level = level as usize - 1;
                    offset += source[last_byte..range.start].encode_utf16().count();
                    last_byte = range.start;
                    let parent = active[..level].iter().rev().flatten().next().copied();
                    active[level..].fill(None);
                    active[level] = Some(headings.len());
                    headings.push(Heading {
                        offset,
                        depth: parent.map_or(0, |index| headings[index].depth + 1),
                        title: String::new(),
                        parent,
                    });
                }
                Event::Text(text)
                | Event::Code(text)
                | Event::Html(text)
                | Event::InlineHtml(text) => {
                    if in_heading && let Some(heading) = headings.last_mut() {
                        heading.title.push_str(&text);
                    }
                    for ch in text.chars() {
                        if ch.is_whitespace() {
                            in_word = false;
                        } else if !in_word && ch.is_alphanumeric() {
                            words += 1;
                            in_word = true;
                        }
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_heading && let Some(heading) = headings.last_mut() {
                        heading.title.push(' ');
                    }
                    in_word = false;
                }
                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;
                    in_word = false;
                }
                Event::End(
                    TagEnd::Paragraph
                    | TagEnd::Item
                    | TagEnd::CodeBlock
                    | TagEnd::HtmlBlock
                    | TagEnd::TableCell,
                ) => in_word = false,
                _ => {}
            }
        }

        Self {
            words,
            headings,
            highlight_ranges: highlight::editor_ranges(&highlights),
            highlights,
        }
    }

    pub(crate) fn highlight_ranges(&self) -> &str {
        self.highlight_ranges.as_deref().unwrap_or("[]")
    }

    fn heading_at(&self, offset: usize) -> Option<usize> {
        self.headings
            .partition_point(|heading| heading.offset <= offset)
            .checked_sub(1)
    }

    fn heading_path(&self, mut index: Option<usize>) -> String {
        let mut path = Vec::new();
        while let Some(current) = index {
            let heading = &self.headings[current];
            path.push(heading.title());
            index = heading.parent;
        }
        if path.is_empty() {
            return "Body".to_owned();
        }
        path.reverse();
        path.join(" › ")
    }
}

#[cfg(test)]
mod tests {
    use super::NoteSummary;

    #[test]
    fn follows_heading_hierarchy_at_normalized_utf16_offsets() {
        let source = "Intro 🧭\r\n\r\n# **Main**\r\n\r\nPart\r\n----\r\n\r\n#### Detail\r\n\r\n```md\r\n# Fake\r\n```\r\n\r\n## Next\r\n";
        let summary = NoteSummary::parse(source);
        let path_at = |marker| {
            let offset = source[..source.find(marker).unwrap()]
                .replace("\r\n", "\n")
                .encode_utf16()
                .count();
            summary.heading_path(summary.heading_at(offset))
        };

        assert_eq!(path_at("Intro"), "Body");
        assert_eq!(path_at("# **Main**"), "Main");
        assert_eq!(path_at("Part"), "Main › Part");
        assert_eq!(path_at("#### Detail"), "Main › Part › Detail");
        assert_eq!(path_at("# Fake"), "Main › Part › Detail");
        assert_eq!(path_at("## Next"), "Main › Next");
    }

    #[test]
    fn counts_markdown_text_across_inline_and_block_boundaries() {
        let source = "# First title\n\nA**B** [linked words](https://example.org) and `inline code`.\n\n- [x] done\n\n| One | Two |\n| --- | --- |\n| A | B |\n";
        assert_eq!(NoteSummary::parse(source).words, 13);
        assert_eq!(NoteSummary::parse("#\n\n---\n\n🧭").words, 0);
    }

    #[test]
    fn excludes_highlight_markup_from_heading_titles_and_words() {
        let source = "# <mark class=xuy-rose>Main title</mark>\n\n### Detail\n\n<mark class=xuy-yellow>Two words</mark>\n";
        let summary = NoteSummary::parse(source);
        assert_eq!(summary.words, 5);
        assert_eq!(summary.headings[0].title(), "Main title");
        assert_eq!(summary.headings[1].depth, 1);
    }

    #[test]
    fn shares_validated_highlight_marks_with_editor_at_normalized_utf16_offsets() {
        let source = "Intro\r\n\r\n# 🧭 <mark class=xuy-mint>🦀 Hi</mark> and <mark class=xuy-rose onclick=\"danger()\">extra</mark>\r\r## <mark class=xuy-sky>Incomplete\r\n\r\nLast <mark class=xuy-yellow>pair</mark>.";
        let summary = NoteSummary::parse(source);

        assert_eq!(
            summary.highlight_ranges(),
            r#"[[33,38,"xuy-mint"],[167,171,"xuy-yellow"]]"#
        );
        assert_eq!(summary.highlights.len(), 2);
        assert_eq!(summary.highlights[0].range, 33..38);
        assert_eq!(summary.highlights[0].text, "🦀 Hi");
        assert_eq!(summary.highlights[1].text, "pair");
        assert_eq!(
            summary.headings[0].title(),
            r#"🧭 🦀 Hi and <mark class=xuy-rose onclick="danger()">extra</mark>"#
        );
        assert_eq!(
            summary.headings[1].title(),
            "<mark class=xuy-sky>Incomplete"
        );
        assert_eq!(summary.words, 10);
        assert_eq!(NoteSummary::parse("Plain note").highlight_ranges(), "[]");
    }

    #[test]
    fn uses_readable_highlight_passages_in_note_order() {
        let source = concat!(
            "One <mark class=xuy-mint>**Review** &amp; keep\nthis</mark>.\n\n",
            "Two <mark class=xuy-sky>**Review** &amp; keep\nthis</mark>.\n\n",
            "~~~\n<mark class=xuy-rose>Example</mark>\n~~~",
        );
        let summary = NoteSummary::parse(source);
        let [first, second] = summary.highlights.as_slice() else {
            panic!("expected two highlighted passages");
        };
        assert_eq!(first.text, "Review & keep this");
        assert_eq!(second.text, first.text);
        assert_eq!(first.color.class(), "xuy-mint");
        assert_eq!(second.color.class(), "xuy-sky");
        assert!(first.range.end < second.range.start);
    }
}
