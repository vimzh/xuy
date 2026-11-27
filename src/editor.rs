use dioxus::prelude::*;
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd, html};

#[component]
pub(crate) fn Editor(
    draft: Signal<String>,
    dirty: Memo<bool>,
    preview: Signal<bool>,
    mut chrome_hidden: Signal<bool>,
    busy: bool,
    on_edit: EventHandler<String>,
    on_save: EventHandler<()>,
) -> Element {
    rsx! {
        section {
            class: "note-editor",
            aria_label: "Markdown editor",
            onkeydown: move |event| {
                if event.modifiers() == Modifiers::META
                    && !event.is_composing()
                    && matches!(event.key(), Key::Character(key) if key.eq_ignore_ascii_case("s"))
                {
                    event.prevent_default();
                    if !busy && !event.is_auto_repeating() && dirty() {
                        on_save.call(());
                    }
                }
            },
            span { class: "note-status", role: "status",
                if dirty() { "Unsaved changes" }
            }
            if preview() {
                MarkdownPreview { draft }
            } else {
                textarea {
                    class: "note-input",
                    aria_label: "Markdown source",
                    aria_keyshortcuts: "Meta+S",
                    readonly: busy,
                    value: "{draft}",
                    onmounted: move |event| async move { let _ = event.set_focus(true).await; },
                    oninput: move |event| {
                        on_edit.call(event.value());
                        if !*chrome_hidden.peek() {
                            chrome_hidden.set(true);
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn MarkdownPreview(draft: Signal<String>) -> Element {
    let rendered = use_resource(move || async move {
        let source = draft();
        tokio::task::spawn_blocking(move || render_markdown(&source)).await
    });
    let render_state = rendered.state();

    rsx! {
        article { class: "markdown-preview", aria_label: "Markdown preview", tabindex: "0",
            match (render_state(), &*rendered.read_unchecked()) {
                (UseResourceState::Ready, Some(Ok(html))) => {
                    rsx! { div { dangerous_inner_html: "{html}" } }
                }
                (UseResourceState::Ready, Some(Err(_))) => {
                    rsx! { p { role: "alert", "Could not render the preview." } }
                }
                _ => rsx! { p { role: "status", "Rendering preview…" } },
            }
        }
    }
}

fn render_markdown(source: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut image_depth = 0;
    let mut active_link = false;
    let events = Parser::new_ext(source, options).filter_map(|event| match event {
        Event::Start(Tag::Image { .. }) => {
            image_depth += 1;
            None
        }
        Event::End(TagEnd::Image) => {
            image_depth -= 1;
            None
        }
        Event::Html(text) | Event::InlineHtml(text) => Some(Event::Text(text)),
        Event::Code(text) if image_depth > 0 => Some(Event::Text(text)),
        Event::HardBreak | Event::SoftBreak if image_depth > 0 => Some(Event::Text(" ".into())),
        _ if image_depth > 0 && !matches!(event, Event::Text(_)) => None,
        Event::Start(Tag::Link {
            link_type,
            ref dest_url,
            ..
        }) => {
            // Dioxus Desktop opens these schemes in the system browser.
            active_link = link_type == LinkType::Email
                || ["https://", "http://", "mailto:"]
                    .iter()
                    .any(|scheme| dest_url.starts_with(scheme));
            active_link.then_some(event)
        }
        Event::End(TagEnd::Link) if !active_link => None,
        event => Some(event),
    });

    let mut output = String::new();
    html::push_html(&mut output, events);
    output
}

#[cfg(test)]
mod tests {
    use super::render_markdown;

    #[test]
    fn renders_common_markdown_and_task_lists() {
        let output = render_markdown("# Heading\n\n- [x] **Done**\n\n`<tag>` and ~~removed~~");

        assert!(output.contains("<h1>Heading</h1>"));
        assert!(output.contains("<ul>"));
        assert!(output.contains(r#"type="checkbox" checked="""#));
        assert!(output.contains("<strong>Done</strong>"));
        assert!(output.contains("<code>&lt;tag&gt;</code>"));
        assert!(output.contains("<del>removed</del>"));
    }

    #[test]
    fn renders_untrusted_html_and_images_as_text() {
        let output = render_markdown(
            "<script>example</script>\n\nInline <em>HTML</em>.\n\n![*Image* `label`](https://example.org/image)",
        );

        assert!(output.contains("&lt;script&gt;example&lt;/script&gt;"));
        assert!(output.contains("Inline &lt;em&gt;HTML&lt;/em&gt;."));
        assert!(output.contains("<p>Image label</p>"));
        assert!(!output.contains("<img"));
        assert!(!output.contains("https://example.org/image"));
    }

    #[test]
    fn keeps_external_web_links_and_flattens_other_destinations() {
        let output = render_markdown(
            "[safe](https://example.org) [inert](javascript:example) [local](file:///example) [relative](other.md)",
        );

        assert_eq!(
            output,
            "<p><a href=\"https://example.org\">safe</a> inert local relative</p>\n"
        );
    }
}
