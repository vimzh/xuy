mod format_menu;
mod highlight;
mod images;
pub(crate) mod preview;
pub(crate) mod status;
mod tools;

use dioxus::desktop::{DesktopContext, use_window};
use dioxus::html::HasFileData;
use dioxus::prelude::{dioxus_core::use_drop, *};

use self::highlight::Color;
use self::tools::WritingSidebar;
use crate::editor::preview::{MarkdownPreview, PreviewTarget};
use crate::editor::status::NoteSummary;
use crate::markdown;
use crate::markdown::paths::MarkdownPaths;
use crate::settings::cursor::CursorOptions;

use format_menu::FormatMenu;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ViewMode {
    #[default]
    Edit,
    Preview,
    Split,
}

impl ViewMode {
    pub(crate) const OPTIONS: [(Self, &'static str, &'static str); 3] = [
        (Self::Edit, "edit", "Edit"),
        (Self::Preview, "preview", "Preview"),
        (Self::Split, "split", "Side by Side"),
    ];

    pub(crate) fn toggle_preview(self) -> Self {
        match self {
            Self::Edit => Self::Preview,
            Self::Preview | Self::Split => Self::Edit,
        }
    }

    pub(crate) fn toggle_split(self) -> Self {
        match self {
            Self::Split => Self::Edit,
            Self::Edit | Self::Preview => Self::Split,
        }
    }

    pub(crate) fn preview_action(self) -> &'static str {
        match self {
            Self::Edit => "Show Preview",
            Self::Preview | Self::Split => "Edit Source",
        }
    }
}

#[component]
pub(crate) fn Editor(
    draft: Signal<String>,
    dirty: Memo<bool>,
    mut view_mode: Signal<ViewMode>,
    edit_in_preview: bool,
    paths: MarkdownPaths,
    mut preview_anchor: Signal<Option<String>>,
    on_preview_open: EventHandler<PreviewTarget>,
    mut chrome_hidden: Signal<bool>,
    busy: bool,
    on_edit: EventHandler<String>,
    on_save: EventHandler<()>,
    cursor: CursorOptions,
    summary: Memo<NoteSummary>,
    notices: Element,
    writing_sidebar_open: bool,
) -> Element {
    let tracker = use_signal(|| None::<document::Eval>);
    let feedback = use_signal(|| None::<&'static str>);
    let format_feedback = use_signal(|| None::<&'static str>);
    let mut applying_highlight = use_signal(|| false);
    let desktop = use_window();
    let mode = view_mode();
    let editable = edit_in_preview && mode == ViewMode::Preview;
    use_effect(move || match view_mode() {
        ViewMode::Edit | ViewMode::Split => {
            document::eval("document.getElementById('note-source')?.focus({preventScroll: true})");
        }
        ViewMode::Preview => {
            document::eval(
                "document.querySelector('.markdown-preview')?.focus({preventScroll: true})",
            );
        }
    });
    let mut highlight_selection = move |color, mut feedback: Signal<Option<&'static str>>| {
        if *applying_highlight.peek() {
            return;
        }
        applying_highlight.set(true);
        feedback.set(None);
        let desktop = desktop.clone();
        spawn(async move {
            feedback.set(apply_highlight(color, &desktop).await.err());
            applying_highlight.set(false);
        });
    };
    let mut menu_highlight = highlight_selection.clone();
    rsx! {
        document::Stylesheet { href: asset!("/assets/writing-sidebar.css") }
        div { class: "workspace-writing",
            {notices}
            section {
                class: "note-editor",
                aria_label: "Markdown editor",
                aria_keyshortcuts: "Meta+Shift+P",
                "data-split": mode == ViewMode::Split,
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
                MarkdownInput {
                    draft, chrome_hidden, busy, hidden: mode == ViewMode::Preview, cursor, tracker, on_edit, summary,
                    paths: paths.clone(),
                    format_menu: rsx! {
                        FormatMenu {
                            highlighting: applying_highlight(),
                            feedback: format_feedback,
                            on_highlight: move |color| menu_highlight(Some(color), format_feedback),
                        }
                    },
                }
                if mode != ViewMode::Edit {
                    MarkdownPreview {
                        key: "{editable}", draft, paths, editable, busy, anchor: preview_anchor,
                        on_open: on_preview_open,
                        on_edit: move |text| {
                            on_edit.call(text);
                            if !*chrome_hidden.peek() { chrome_hidden.set(true); }
                        },
                    }
                }
            }
        }
        WritingSidebar {
            open: writing_sidebar_open,
            summary, preview: mode == ViewMode::Preview, busy: busy || applying_highlight(), feedback,
            on_jump: move |(index, offset)| {
                if *view_mode.peek() == ViewMode::Preview {
                    document::eval(&format!("
                        const heading = document.querySelector('.markdown-preview')
                            ?.querySelectorAll('h1,h2,h3,h4,h5,h6')[{index}];
                        if (heading) {{
                            heading.tabIndex = -1;
                            heading.focus({{preventScroll: true}});
                            heading.scrollIntoView({{block: 'start'}});
                        }}
                    "));
                } else if let Some(tracker) = *tracker.peek() {
                    let _ = tracker.send((offset, offset));
                }
            },
            on_jump_highlight: move |range: std::ops::Range<usize>| {
                if *view_mode.peek() == ViewMode::Preview {
                    preview_anchor.set(None);
                    view_mode.set(ViewMode::Edit);
                    chrome_hidden.set(false);
                }
                if let Some(tracker) = *tracker.peek() {
                    let _ = tracker.send((range.start, range.end));
                }
            },
            on_highlight: move |color| highlight_selection(color, feedback),
        }
    }
}

#[component]
fn MarkdownInput(
    draft: Signal<String>,
    mut chrome_hidden: Signal<bool>,
    busy: bool,
    hidden: bool,
    cursor: CursorOptions,
    mut tracker: Signal<Option<document::Eval>>,
    on_edit: EventHandler<String>,
    summary: Memo<NoteSummary>,
    paths: MarkdownPaths,
    format_menu: Element,
) -> Element {
    let mut image_requests = use_signal(|| None::<document::Eval>);
    use_drop(move || {
        if let Some(tracker) = tracker.write().take() {
            let _ = tracker.send(());
        }
        if let Some(requests) = image_requests.write().take() {
            let _ = requests.send(false);
        }
    });
    let color = cursor.color();
    let summary = summary.read();
    let ranges = summary.highlight_ranges();
    let desktop = use_window();
    let mut importing_image = use_signal(|| false);
    let mut image_error = use_signal(|| None::<String>);

    rsx! {
        div { class: "note-input-frame", hidden,
            "data-caret-shape": cursor.shape.id(),
            "data-caret-blink": "{cursor.blink}",
            "data-caret-smooth": "{cursor.smooth}",
            "data-caret-blink-style": cursor.blink_style.id(),
            "data-highlight-ranges": "{ranges}",
            style: "--note-caret-color: {color}",
            div { class: "note-highlight-layer", aria_hidden: true,
                div { class: "note-highlight-marks" }
            }
            textarea {
                id: "note-source",
                class: "note-input",
                aria_label: "Markdown source",
                aria_keyshortcuts: "Meta+S Shift+F10",
                readonly: busy,
                value: "{draft}",
                onmounted: {
                    let paths = paths.clone();
                    let desktop = desktop.clone();
                    move |_| {
                        tracker.set(Some(document::eval(concat!(
                            include_str!("code_blocks.js"),
                            "\n",
                            include_str!("editor_caret.js"),
                        ))));
                        image_requests.set(Some(images::listen(paths.clone(), desktop.clone(), importing_image, image_error)));
                    }
                },
                ondrop: move |event| {
                    if event.data_transfer().files().is_empty() {
                        return;
                    }
                    event.prevent_default();
                    if busy || hidden || *importing_image.peek() {
                        return;
                    }
                    let sources = event.files().into_iter().map(|file| file.path()).collect::<Vec<_>>();
                    if sources.is_empty() {
                        image_error.set(Some("Could not read the dropped files. Drop them again from Finder.".into()));
                        return;
                    }
                    let client_y = event.client_coordinates().y;
                    let paths = paths.clone();
                    let desktop = desktop.clone();
                    importing_image.set(true);
                    spawn(async move {
                        image_error.set(images::drop_files(paths, sources, client_y, desktop).await.err());
                        importing_image.set(false);
                    });
                },
                oninput: move |event| {
                    on_edit.call(event.value());
                    if !*chrome_hidden.peek() {
                        chrome_hidden.set(true);
                    }
                },
                onblur: move |_| {
                    if *chrome_hidden.peek() {
                        chrome_hidden.set(false);
                    }
                },
            }
            div { class: "note-caret-layer", aria_hidden: true,
                span { class: "note-custom-caret", hidden: true }
            }
            {format_menu}
            p { class: "workspace-error note-code-block-error", role: "alert", hidden: true }
            if let Some(error) = image_error() {
                p { class: "workspace-error", role: "alert", "{error}" }
            }
        }
    }
}

async fn apply_highlight(
    color: Option<Color>,
    desktop: &DesktopContext,
) -> Result<(), &'static str> {
    let mut command = document::eval(include_str!("highlight_apply.js"));
    let selection = command.recv::<Option<(String, usize, usize)>>().await;
    let result = match selection {
        Ok(Some((source, start, end))) => {
            tokio::task::spawn_blocking(move || prepare_highlight(&source, start, end, color))
                .await
                .unwrap_or(Err("Could not prepare the highlight."))
        }
        _ => Err("Open a note in Edit to change its highlights."),
    };
    let Ok(Some(change)) = result else {
        let _ = command.send(());
        return result.map(|_| ());
    };
    #[cfg(target_os = "macos")]
    let undo_manager = {
        use dioxus::desktop::wry::WebViewExtMacOS;
        desktop.webview.webview().undoManager()
    };
    #[cfg(target_os = "macos")]
    let _undo_group = match undo_manager.as_deref().and_then(EditorUndoGroup::begin) {
        Some(group) => group,
        None => {
            let _ = command.send(());
            return Err("Could not start a highlight undo step. Try again.");
        }
    };
    #[cfg(not(target_os = "macos"))]
    let _ = desktop;
    command
        .send(change)
        .map_err(|_| "Could not reach the note editor.")?;
    let applied = command.recv::<bool>().await;
    let _ = command.send(());
    if matches!(applied, Ok(true)) {
        Ok(())
    } else {
        Err("Could not apply the highlight. Select the text and try again.")
    }
}

#[cfg(target_os = "macos")]
struct EditorUndoGroup<'a>(&'a objc2_foundation::NSUndoManager);

#[cfg(target_os = "macos")]
impl<'a> EditorUndoGroup<'a> {
    fn begin(manager: &'a objc2_foundation::NSUndoManager) -> Option<Self> {
        if !manager.groupsByEvent()
            || manager.groupingLevel() > 1
            || manager.isUndoing()
            || manager.isRedoing()
        {
            return None;
        }
        // Close any automatic group before opening a top-level group for this edit.
        manager.setGroupsByEvent(false);
        if manager.groupingLevel() == 1 {
            manager.endUndoGrouping();
        }
        manager.beginUndoGrouping();
        manager.setGroupsByEvent(true);
        Some(Self(manager))
    }
}

#[cfg(target_os = "macos")]
impl Drop for EditorUndoGroup<'_> {
    fn drop(&mut self) {
        let manager = self.0;
        if manager.groupingLevel() == 1 && !manager.isUndoing() && !manager.isRedoing() {
            // Restore automatic grouping before returning to ordinary editing.
            manager.setGroupsByEvent(false);
            manager.endUndoGrouping();
            manager.setGroupsByEvent(true);
        }
    }
}

fn prepare_highlight(
    source: &str,
    start: usize,
    end: usize,
    color: Option<Color>,
) -> Result<Option<(usize, usize, String)>, &'static str> {
    let Some(edit) = highlight::edit(source, start, end, color)? else {
        return Ok(None);
    };
    let mut updated = source.to_owned();
    updated.replace_range(edit.range.clone(), &edit.replacement);
    if highlight::strip_rendered_marks(render_markdown(source))
        != highlight::strip_rendered_marks(render_markdown(&updated))
    {
        return Err("Adjust the selection to keep the Markdown formatting intact.");
    }
    let start = source[..edit.range.start].encode_utf16().count();
    let end = start + source[edit.range].encode_utf16().count();
    Ok(Some((start, end, edit.replacement)))
}

pub(crate) fn render_markdown(source: &str) -> String {
    render_preview(source, None)
}

pub(crate) fn render_preview(source: &str, paths: Option<&MarkdownPaths>) -> String {
    let marks = highlight::marks(source);
    let mut offsets = highlight::mark_offsets(&marks).peekable();
    markdown::render(source, paths, |offset| {
        offsets.next_if_eq(&offset).is_some()
    })
}

pub(crate) fn render_editable_preview(
    source: &str,
    paths: &MarkdownPaths,
) -> (String, Vec<crate::markdown::edits::EditableText>) {
    let marks = highlight::marks(source);
    let mut offsets = highlight::mark_offsets(&marks).peekable();
    markdown::render_editable(source, Some(paths), |offset| {
        offsets.next_if_eq(&offset).is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::{prepare_highlight, render_markdown};
    use crate::editor::highlight::{Color, strip_rendered_marks};
    use crate::editor::status::NoteSummary;

    fn apply(source: &str, selected: &str, color: Option<Color>) -> String {
        let start = source[..source.find(selected).unwrap()]
            .encode_utf16()
            .count();
        let end = start + selected.encode_utf16().count();
        let (from, to, replacement) = prepare_highlight(source, start, end, color)
            .unwrap()
            .unwrap();
        let bytes = source.encode_utf16().collect::<Vec<_>>();
        String::from_utf16(&bytes[..from]).unwrap()
            + &replacement
            + &String::from_utf16(&bytes[to..]).unwrap()
    }

    #[test]
    fn renders_common_markdown_and_task_lists() {
        let output = render_markdown("# Heading\n\n- [x] **Done**\n\n`<tag>` and ~~removed~~");

        assert!(output.contains("<h1 id=\"heading\">Heading</h1>"));
        assert!(output.contains("<ul>"));
        assert!(output.contains(r#"type="checkbox" checked="""#));
        assert!(output.contains("<strong>Done</strong>"));
        assert!(output.contains("<code>&lt;tag&gt;</code>"));
        assert!(output.contains("<del>removed</del>"));
    }

    #[test]
    fn escapes_raw_html_and_renders_safe_markdown_images() {
        let output = render_markdown(
            "<script>example</script>\n\nInline <em>HTML</em>.\n\n![*Image* `label`](https://example.org/image)",
        );

        assert!(output.contains("&lt;script&gt;example&lt;/script&gt;"));
        assert!(output.contains("Inline &lt;em&gt;HTML&lt;/em&gt;."));
        assert!(output.contains("<img"));
        assert!(output.contains("alt=\"Image label\""));
        assert!(output.contains("https://example.org/image"));
    }

    #[test]
    fn keeps_web_and_notebook_links_and_flattens_unsafe_destinations() {
        let output = render_markdown(
            "[safe](https://example.org) [inert](javascript:example) [local](file:///example) [relative](other.md)",
        );

        assert_eq!(
            output,
            "<p><a href=\"https://example.org\">safe</a> inert local <a href=\"#xuy-note:other%2Emd\">relative</a></p>\n"
        );
    }

    #[test]
    fn creates_recolors_and_clears_highlights_without_changing_markdown_structure() {
        let source = "# 🧭 Start\n\n- **Bold** and a [link](https://example.org)\n";
        let highlighted = apply(source, "Start\n\n- **Bold** and a [link", Some(Color::Mint));
        let html = render_markdown(&highlighted);
        assert!(html.contains("<mark class=xuy-mint>Start</mark>"));
        assert!(html.contains("<strong><mark class=xuy-mint>Bold</mark></strong>"));
        assert_eq!(strip_rendered_marks(html), render_markdown(source));
        let recolored = apply(&highlighted, "Bold", Some(Color::Sky));
        assert!(render_markdown(&recolored).contains("<mark class=xuy-sky>Bold</mark>"));
        let cleared = apply(&recolored, "Bold", None);
        assert!(cleared.contains("**Bold**"));
        assert!(cleared.contains("xuy-mint"));
    }

    #[test]
    fn handles_all_five_colors_and_utf16_ranges() {
        for color in Color::ALL {
            let source = "🧭\n\n💡 Idea";
            let highlighted = apply(source, "💡 Idea", Some(color));
            let from = highlighted[..highlighted.find("💡").unwrap()]
                .encode_utf16()
                .count();
            assert_eq!(
                NoteSummary::parse(&highlighted.replace('\n', "\r\n")).highlight_ranges(),
                format!("[[{from},{},\"{}\"]]", from + 7, color.class())
            );
            assert_eq!(apply(&highlighted, "💡 Idea", None), source);
        }
    }

    #[test]
    fn preview_only_admits_balanced_highlight_tags_with_allowlisted_attributes() {
        let source = "One <mark class=xuy-mint>valid</mark>.\n\n<mark class=xuy-rose onclick=\"danger()\">modified</mark>\n\n<mark class=xuy-sky>unbalanced\n\n\x60<mark class=xuy-yellow>code</mark>\x60";
        let html = render_markdown(source);
        assert_eq!(html.matches("<mark ").count(), 1);
        assert!(html.contains("<mark class=xuy-mint>valid</mark>"));
        assert!(html.contains("&lt;mark class="));
    }
}
