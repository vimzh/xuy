//! Renders Markdown with safe links, images, and stable heading anchors.

pub(crate) mod edits;
pub(crate) mod paths;
mod syntax;

use std::{collections::HashMap, ops::Range, path::Path};

use linkify::{LinkFinder, LinkKind};
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use pulldown_cmark::utils::TextMergeWithOffset;
use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag, TagEnd, html};

use crate::markdown::edits::EditableText;
use crate::markdown::paths::MarkdownPaths;

pub(crate) const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_FOOTNOTES)
    .union(Options::ENABLE_GFM)
    .union(Options::ENABLE_DEFINITION_LIST)
    .union(Options::ENABLE_WIKILINKS);

const NOTE_LINK: &str = "#xuy-note:";
const WIKI_LINK: &str = "#xuy-wiki:";

pub(crate) fn render(
    source: &str,
    paths: Option<&MarkdownPaths>,
    allow_inline: impl FnMut(usize) -> bool,
) -> String {
    render_inner(source, paths, allow_inline, None)
}

pub(crate) fn render_editable(
    source: &str,
    paths: Option<&MarkdownPaths>,
    allow_inline: impl FnMut(usize) -> bool,
) -> (String, Vec<EditableText>) {
    let mut editable = Vec::new();
    let mut html = render_inner(source, paths, allow_inline, Some(&mut editable));
    if html.is_empty() && source.trim().is_empty() {
        html.push_str("<p>");
        let range = 0..source.len();
        let span = editable_span("", range.clone(), range, true, &mut editable);
        html.push_str(&span);
        html.push_str("</p>");
    }
    (html, editable)
}

fn render_inner(
    source: &str,
    paths: Option<&MarkdownPaths>,
    mut allow_inline: impl FnMut(usize) -> bool,
    mut editable: Option<&mut Vec<EditableText>>,
) -> String {
    let mut input = TextMergeWithOffset::new(Parser::new_ext(source, OPTIONS).into_offset_iter());
    let mut finder = LinkFinder::new();
    finder.url_must_have_scheme(false);
    let mut events = Vec::new();
    let mut heading: Option<(usize, String)> = None;
    let mut heading_ids = HashMap::new();
    let mut code_block = None;
    let mut in_link = false;
    let mut emit_link = false;
    let mut depth = 0;
    let mut paragraph = None::<Range<usize>>;
    let mut formats = Vec::new();

    while let Some((event, range)) = input.next() {
        if editable.is_some() {
            match &event {
                Event::Start(Tag::Paragraph) if depth == 0 => {
                    paragraph = Some(range.clone());
                    depth += 1;
                }
                Event::Start(Tag::Image { .. }) => {}
                Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough) => {
                    formats.push(range.clone());
                    depth += 1;
                }
                Event::Start(_) => depth += 1,
                Event::End(tag) => {
                    if matches!(
                        tag,
                        TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough
                    ) {
                        formats.pop();
                    }
                    depth -= 1;
                    if depth == 0 {
                        paragraph = None;
                    }
                }
                _ => {}
            }
        }
        let allowed = matches!(event, Event::InlineHtml(_)) && allow_inline(range.start);
        if !allowed && let Some((_, label)) = &mut heading {
            append_plain_text(label, &event);
        }

        let event = match event {
            Event::Start(Tag::Heading { .. }) => {
                heading = Some((events.len(), String::new()));
                event
            }
            Event::End(TagEnd::Heading(_)) => {
                let (index, label) = heading.take().unwrap();
                let Event::Start(Tag::Heading { id, .. }) = &mut events[index] else {
                    unreachable!("the heading started at this event");
                };
                *id = Some(heading_id(&label, &mut heading_ids).into());
                event
            }
            Event::Start(Tag::CodeBlock(ref kind)) => {
                code_block = Some((kind.clone(), String::new()));
                event
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((kind, text)) = code_block.take() {
                    events.push(syntax::render_code(&kind, text.into()));
                }
                event
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                in_link = true;
                let dest_url = link_destination(link_type, dest_url);
                emit_link = dest_url.is_some();
                let Some(dest_url) = dest_url else { continue };
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                })
            }
            Event::End(TagEnd::Link) => {
                in_link = false;
                if !emit_link {
                    continue;
                }
                event
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                let alt = image_alt(&mut input, &mut allow_inline);
                if let Some((_, label)) = &mut heading {
                    label.push_str(&alt);
                }
                match if is_web_url(&dest_url) {
                    Some(dest_url)
                } else {
                    paths
                        .and_then(|paths| paths.image_source(&dest_url))
                        .map(Into::into)
                } {
                    Some(src) => Event::Html(image_html(src, title, alt).into()),
                    None => Event::Text(alt.into()),
                }
            }
            Event::Start(Tag::FootnoteDefinition(name)) => {
                Event::Start(Tag::FootnoteDefinition(footnote_id(&name).into()))
            }
            Event::FootnoteReference(name) => Event::FootnoteReference(footnote_id(&name).into()),
            Event::SoftBreak => Event::HardBreak,
            Event::InlineHtml(text) if !allowed => Event::Text(text),
            Event::Html(text) => Event::Text(text),
            Event::Text(text) if !in_link => {
                if let Some((_, code)) = &mut code_block {
                    // Quoted and listed fences can arrive as several parser events.
                    code.push_str(&text);
                    continue;
                }
                let edit = editable.as_deref_mut().map(|runs| (range, runs));
                push_autolinks(
                    &mut events,
                    &finder,
                    text,
                    source,
                    edit,
                    paragraph.as_ref(),
                    &formats,
                );
                continue;
            }
            event => event,
        };
        events.push(event);
    }

    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    output
}

fn starts_with_ignoring_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

fn is_web_url(value: &str) -> bool {
    starts_with_ignoring_case(value, "https://") || starts_with_ignoring_case(value, "http://")
}

fn link_destination(link_type: LinkType, destination: CowStr<'_>) -> Option<CowStr<'_>> {
    let wiki = matches!(link_type, LinkType::WikiLink { .. });
    if !wiki {
        if link_type == LinkType::Email
            || is_web_url(&destination)
            || starts_with_ignoring_case(&destination, "mailto:")
        {
            return Some(destination);
        }
        if destination.starts_with('#') {
            return (!destination.starts_with(NOTE_LINK) && !destination.starts_with(WIKI_LINK))
                .then_some(destination);
        }
    }

    let path = destination.split('#').next().unwrap();
    let path = percent_decode_str(path).decode_utf8().ok()?;
    if path.starts_with(['/', '\\']) || path.split('/').next().unwrap().contains(':') {
        return None;
    }
    let markdown = Path::new(path.as_ref())
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"));
    if !markdown && (!wiki || destination.is_empty()) {
        return None;
    }
    let marker = if wiki { WIKI_LINK } else { NOTE_LINK };
    Some(
        format!(
            "{marker}{}",
            utf8_percent_encode(&destination, NON_ALPHANUMERIC)
        )
        .into(),
    )
}

fn push_autolinks<'a>(
    events: &mut Vec<Event<'a>>,
    finder: &LinkFinder,
    text: CowStr<'a>,
    source: &str,
    mut edit: Option<(Range<usize>, &mut Vec<EditableText>)>,
    paragraph: Option<&Range<usize>>,
    formats: &[Range<usize>],
) {
    let preserve_original = edit.is_none();
    let mut push_text = |events: &mut Vec<Event<'a>>, bytes: Range<usize>| {
        let value = &text[bytes.clone()];
        let range_for = |range: &Range<usize>| {
            if bytes.start == 0 && bytes.end == text.len() {
                Some(range.clone())
            } else if source.get(range.clone()) == Some(text.as_ref()) {
                Some(range.start + bytes.start..range.start + bytes.end)
            } else {
                None
            }
        };
        if let Some((original, runs)) = &mut edit
            && let Some(range) = range_for(original)
        {
            let multiline = paragraph.is_some_and(|p| {
                range.start == p.start
                    && source
                        .get(range.end..p.end)
                        .is_some_and(|rest| rest.trim().is_empty())
            });
            let enclosing = formats
                .iter()
                .rev()
                .take_while(|format| {
                    format.start < range.start
                        && range.end < format.end
                        && [
                            &source[format.start..range.start],
                            &source[range.end..format.end],
                        ]
                        .into_iter()
                        .all(|part| part.bytes().all(|byte| matches!(byte, b'*' | b'_' | b'~')))
                })
                .last()
                .cloned()
                .unwrap_or_else(|| range.clone());
            events.push(Event::InlineHtml(
                editable_span(value, range, enclosing, multiline, runs).into(),
            ));
        } else {
            events.push(Event::Text(value.to_owned().into()));
        }
    };
    let mut written = 0;
    for link in finder.links(&text) {
        let label = link.as_str();
        let (link_type, dest_url) = match link.kind() {
            LinkKind::Email => (LinkType::Email, label.to_owned()),
            LinkKind::Url if is_web_url(label) => (LinkType::Autolink, label.to_owned()),
            LinkKind::Url if starts_with_ignoring_case(label, "www.") => {
                (LinkType::Autolink, format!("http://{label}"))
            }
            _ => continue,
        };
        if written < link.start() {
            push_text(events, written..link.start());
        }
        events.extend([
            Event::Start(Tag::Link {
                link_type,
                dest_url: dest_url.into(),
                title: "".into(),
                id: "".into(),
            }),
            Event::Text(label.to_owned().into()),
            Event::End(TagEnd::Link),
        ]);
        written = link.end();
    }
    if written == 0 && preserve_original {
        events.push(Event::Text(text));
    } else if written < text.len() {
        push_text(events, written..text.len());
    }
}

fn editable_span(
    text: &str,
    range: Range<usize>,
    enclosing: Range<usize>,
    multiline: bool,
    editable: &mut Vec<EditableText>,
) -> String {
    let index = editable.len();
    editable.push(EditableText::new(range, enclosing, text, multiline));
    let mut html = format!(
        r#"<span data-md-text="{index}" contenteditable="plaintext-only" role="textbox" aria-label="Markdown text" aria-multiline="{multiline}">"#
    );
    html::push_html(&mut html, std::iter::once(Event::Text(text.into())));
    html.push_str("</span>");
    html
}

pub(crate) fn append_plain_text(output: &mut String, event: &Event<'_>) {
    match event {
        Event::Text(text) | Event::Code(text) | Event::InlineHtml(text) | Event::Html(text) => {
            output.push_str(text);
        }
        Event::SoftBreak | Event::HardBreak => output.push(' '),
        _ => {}
    }
}

fn image_alt<'a>(
    input: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>,
    allow_inline: &mut impl FnMut(usize) -> bool,
) -> String {
    let mut depth = 1;
    let mut alt = String::new();
    for (event, range) in input {
        match event {
            Event::Start(Tag::Image { .. }) => depth += 1,
            Event::End(TagEnd::Image) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::InlineHtml(_) if allow_inline(range.start) => {}
            event => append_plain_text(&mut alt, &event),
        }
    }
    alt
}

fn image_html(src: CowStr<'_>, title: CowStr<'_>, alt: String) -> String {
    let mut output = String::new();
    html::push_html(
        &mut output,
        [
            Event::Start(Tag::Image {
                link_type: LinkType::Inline,
                dest_url: src,
                title,
                id: "".into(),
            }),
            Event::Text(alt.into()),
            Event::End(TagEnd::Image),
        ]
        .into_iter(),
    );
    output.insert_str(
        "<img".len(),
        " loading=\"lazy\" referrerpolicy=\"no-referrer\"",
    );
    output
}

fn heading_id(label: &str, used: &mut HashMap<String, usize>) -> String {
    let base = heading_slug(label);
    let Some(mut suffix) = used.insert(base.clone(), 1) else {
        return base;
    };
    loop {
        let id = format!("{base}-{suffix}");
        suffix += 1;
        if let std::collections::hash_map::Entry::Vacant(entry) = used.entry(id.clone()) {
            entry.insert(1);
            used.insert(base, suffix);
            return id;
        }
    }
}

pub(crate) fn heading_slug(label: &str) -> String {
    let mut slug: String = label
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|ch| {
            if ch.is_whitespace() {
                Some('-')
            } else {
                (ch.is_alphanumeric() || matches!(ch, '-' | '_')).then_some(ch)
            }
        })
        .collect();
    if slug.is_empty() {
        slug.push_str("heading");
    }
    slug
}

fn footnote_id(name: &str) -> String {
    // Keep footnote labels away from heading IDs and reserved navigation fragments.
    format!(
        "xuy-footnote:{}",
        utf8_percent_encode(name, NON_ALPHANUMERIC)
    )
    .replace('%', "-")
}

#[cfg(test)]
mod tests {
    use super::{render, render_editable};

    #[test]
    fn edit_ranges_preserve_nested_formatting_and_reference_links() {
        use crate::markdown::edits::PreviewEdits;

        let source = "**_Bold_** with [label][id].\n\n[id]: https://example.org";
        let (html, runs) = render_editable(source, None, |_| false);
        assert!(html.contains("<strong><em><span data-md-text=\"0\""));
        assert!(html.contains(">label</a>"));
        let mut edit = PreviewEdits::new(0, source, &runs);
        edit.replace(source, &runs, 0, "Changed").unwrap();
        assert_eq!(
            edit.draft,
            "**_Changed_** with [label][id].\n\n[id]: https://example.org"
        );
        edit.replace(source, &runs, 0, "").unwrap();
        assert_eq!(
            edit.draft,
            " with [label][id].\n\n[id]: https://example.org"
        );
    }

    fn preview(source: &str) -> String {
        render(source, None, |_| false)
    }

    #[test]
    fn previews_source_line_breaks_within_paragraphs() {
        use crate::markdown::edits::PreviewEdits;

        let source = "First line\nSecond line  \nThird line\n\nAnother paragraph.\n";
        assert_eq!(
            preview(source),
            "<p>First line<br />\nSecond line<br />\nThird line</p>\n<p>Another paragraph.</p>\n"
        );
        let (editable, runs) = render_editable(source, None, |_| false);
        assert_eq!(editable.matches("<br />").count(), 2);

        let mut edit = PreviewEdits::new(0, source, &runs);
        assert_eq!(edit.replace(source, &runs, 1, "Changed line"), Ok(true));
        assert_eq!(
            edit.draft,
            "First line\nChanged line  \nThird line\n\nAnother paragraph.\n"
        );
    }

    #[test]
    fn renders_commonmark_and_extended_blocks() {
        let html = preview(
            "# Heading\n\n- [x] **Done**\n\n`<tag>` and ~~old~~[^a]\n\n[^a]: Footnote\n\n| Key | Value |\n| --- | --- |\n| A | B |\n\n> [!TIP]\n> An alert.\n\nTerm\n: Definition\n",
        );
        for expected in [
            "<h1 id=\"heading\">Heading</h1>",
            "type=\"checkbox\" checked=\"\"",
            "<strong>Done</strong>",
            "<code>&lt;tag&gt;</code>",
            "<del>old</del>",
            "href=\"#xuy-footnote:a\"",
            "id=\"xuy-footnote:a\"",
            "<table>",
            "class=\"markdown-alert-tip\"",
            "<dt>Term</dt>",
            "<dd>Definition</dd>",
        ] {
            assert!(html.contains(expected), "missing {expected} in {html}");
        }
    }

    #[test]
    fn sanitizes_raw_markup_unsafe_urls_and_reserved_fragments() {
        let html = preview(
            "<script>alert(1)</script>\n\nInline <em>text</em>. [code](java&#x73;cript:alert) [file](file:///note.md) [protocol](//example.org/note.md) [marker](#xuy-note:note.md) [wiki marker](#xuy-wiki:note) ![*unsafe* `image`](data:image/svg+xml,example) ![local image](missing.png).[^xuy-note:file.md]\n\n[^xuy-note:file.md]: Footnote",
        );
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("Inline &lt;em&gt;text&lt;/em&gt;. code file protocol marker wiki marker unsafe image local image."));
        assert_eq!(html.matches("href=").count(), 1);
        assert!(html.contains("href=\"#xuy-footnote:xuy-2Dnote-3Afile-2Emd\""));
        assert!(!html.contains("<img"));
    }

    #[test]
    fn renders_links_and_routes_note_destinations() {
        let html = preview(
            "[web](HTTPS://example.org) [email](mailto:user@example.org) [peer](<../Caf%C3%A9%2Emd#other>) [[another note|Alias]] [[#here]] [here](#here)",
        );
        for (href, label) in [
            ("HTTPS://example.org", "web"),
            ("mailto:user@example.org", "email"),
            ("#xuy-note:%2E%2E%2FCaf%25C3%25A9%252Emd%23other", "peer"),
            ("#xuy-wiki:another%20note", "Alias"),
            ("#xuy-wiki:%23here", "#here"),
            ("#here", "here"),
        ] {
            assert!(
                html.contains(&format!("<a href=\"{href}\">{label}</a>")),
                "{html}"
            );
        }
    }

    #[test]
    fn creates_unique_heading_ids_from_visible_heading_text() {
        let source = "# *Café* `item`\n\n## Café item-1\n\n### <mark class=xuy-sky>Café</mark> item\n\n# ![Image label](missing.png)\n\n# 🧭";
        let html = render(source, None, |offset| {
            source[offset..].starts_with("<mark class=xuy-sky>")
                || source[offset..].starts_with("</mark>")
        });
        assert!(html.contains("<h1 id=\"café-item\"><em>Café</em> <code>item</code></h1>"));
        assert!(html.contains("<h2 id=\"café-item-1\">Café item-1</h2>"));
        assert!(html.contains("<h3 id=\"café-item-2\"><mark class=xuy-sky>Café</mark> item</h3>"));
        assert!(html.contains("<h1 id=\"image-label\">Image label</h1>"));
        assert!(html.contains("<h1 id=\"heading\">🧭</h1>"));
    }

    #[test]
    fn detects_bare_web_urls_and_email_outside_links_and_code() {
        let html = preview(
            "See (https://example.org/?first=1&amp;second=2), www.example.com and a+b@example.org. plain.example.com\n\n`https://code.example.org` [https://label.example.org](#top)\n\n```txt\nhttps://block.example.org\n```",
        );
        assert!(html.contains("(<a href=\"https://example.org/?first=1&amp;second=2\">https://example.org/?first=1&amp;second=2</a>),"), "{html}");
        assert!(html.contains("<a href=\"http://www.example.com\">www.example.com</a>"));
        assert!(
            html.contains(
                "<a href=\"mailto:a+b@example.org\">a+b@example.org</a>. plain.example.com"
            )
        );
        assert!(html.contains("<a href=\"#top\">https://label.example.org</a>"));
        assert_eq!(html.matches("href=").count(), 4);
    }

    #[test]
    fn highlights_tagged_fences_inside_markdown_blocks() {
        let html = preview(
            "> ```rust example\n> /* first\n>    second */\n> if ready { let link = \"https://example.org\"; }\n> ```\n\nAfter the quote.",
        );
        assert!(html.contains("<blockquote>\n<pre><code class=\"language-rust\">"));
        for scope in ["syntax-comment", "syntax-keyword", "syntax-string"] {
            assert!(html.contains(scope), "missing {scope} in {html}");
        }
        assert!(html.contains(" first\n   second "), "{html}");
        assert!(html.contains("</code></pre>\n</blockquote>\n<p>After the quote.</p>"));
        assert!(!html.contains("<a "));
    }

    #[test]
    fn treats_html_in_highlighted_and_plain_fences_as_code() {
        let html = preview(
            "```html\n<img src=x onerror=alert(1)>\n```\n\n```xuy-unknown\n<img src=x>\n```\n\n```\n<img src=x>\n```\n\n    <img src=x>\n",
        );
        assert!(!html.contains("<img"));
        assert!(html.contains("<pre><code class=\"language-html\"><span"));
        assert!(html.contains(
            "<pre><code class=\"language-xuy-unknown\">&lt;img src=x&gt;\n</code></pre>"
        ));
        assert_eq!(
            html.matches("<pre><code>&lt;img src=x&gt;\n</code></pre>")
                .count(),
            2
        );
    }

    #[test]
    fn highlighting_preserves_the_final_line_of_an_unterminated_fence() {
        let html = preview("```rust\n// final");
        assert!(html.contains(" final</span>"), "{html}");
    }

    #[test]
    fn renders_images_with_plain_escaped_alt_text_and_safe_attributes() {
        let html = preview(
            "![*Caption* `code` \"quote\" <b>tag</b> ![nested](inner.png)](https://example.org/image.png?x=1&y=2 \"description\")",
        );
        assert_eq!(
            html,
            "<p><img loading=\"lazy\" referrerpolicy=\"no-referrer\" src=\"https://example.org/image.png?x=1&amp;y=2\" alt=\"Caption code &quot;quote&quot; &lt;b&gt;tag&lt;/b&gt; nested\" title=\"description\" /></p>\n"
        );
    }

    #[test]
    fn embeds_relative_raster_images_and_uses_alt_for_missing_files() {
        let notebook = tempfile::tempdir().unwrap();
        let root = notebook.path().canonicalize().unwrap();
        std::fs::write(root.join("picture.png"), b"\x89PNG\r\n\x1a\n").unwrap();
        let paths = crate::markdown::paths::MarkdownPaths::new(root.clone(), root.join("note.md"));

        let html = render(
            "![Picture](picture.png) ![*Missing*](missing.png)",
            Some(&paths),
            |_| false,
        );
        assert!(
            html.contains("src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"Picture\" /> Missing")
        );
    }
}
