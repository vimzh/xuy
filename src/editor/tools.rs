//! The secondary sidebar for navigating headings and highlighted passages.

use std::ops::Range;

use dioxus::prelude::*;

use super::{highlight::Color, status::NoteSummary};
use crate::ui::Button;

#[component]
pub(crate) fn WritingSidebar(
    open: bool,
    summary: Memo<NoteSummary>,
    preview: bool,
    busy: bool,
    feedback: Signal<Option<&'static str>>,
    on_jump: EventHandler<(usize, usize)>,
    on_jump_highlight: EventHandler<Range<usize>>,
    on_highlight: EventHandler<Option<Color>>,
) -> Element {
    let mut show_highlights = use_signal(|| false);
    rsx! {
        aside { id: "writing-sidebar", class: "writing-sidebar", aria_label: "Note tools", hidden: !open,
            div { class: "writing-modes", role: "group", aria_label: "Note tools view",
                Button { aria_pressed: !show_highlights(),
                    aria_controls: "writing-tools-view", onclick: move |_| show_highlights.set(false), "Outline" }
                Button { aria_pressed: show_highlights(),
                    aria_controls: "writing-tools-view", onclick: move |_| show_highlights.set(true), "Highlights" }
            }
            div { id: "writing-tools-view", class: "writing-tools-view",
                if show_highlights() {
                    if summary.read().highlights.is_empty() {
                        p { class: "writing-hint", "No highlighted text." }
                    } else {
                        ol { class: "note-highlights", role: "list", aria_label: "Highlighted passages",
                            for passage in &summary.read().highlights {
                                {
                                    let text = passage.text.trim();
                                    let label = if text.is_empty() { "Highlighted text" } else { text };
                                    let range = passage.range.clone();
                                    let color = passage.color.class();
                                    rsx! {
                                        li { key: "{range.start}",
                                            Button { class: "highlight-jump", disabled: busy, title: "{label}",
                                                onclick: move |_| on_jump_highlight.call(range.clone()),
                                                span { class: "highlight-swatch {color}", aria_hidden: true }
                                                span { class: "highlight-excerpt", "{label}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !preview {
                        details { class: "highlight-tools",
                            summary { "Highlight tools" }
                            p { class: "writing-hint", "Select text, then choose a color." }
                            div { class: "highlight-colors", role: "group", aria_label: "Highlight color",
                                for color in Color::ALL {
                                    Button { key: "{color.class()}", disabled: busy,
                                        onclick: move |_| on_highlight.call(Some(color)),
                                        span { class: "highlight-swatch {color.class()}", aria_hidden: true }
                                        "{color.name()}"
                                    }
                                }
                            }
                            Button { class: "highlight-clear", disabled: busy,
                                onclick: move |_| on_highlight.call(None), "Clear highlight" }
                            if let Some(message) = feedback() {
                                p { class: "writing-hint", role: "status", "{message}" }
                            }
                        }
                    }
                } else if summary.read().headings.is_empty() {
                    p { class: "writing-hint", "No headings" }
                } else {
                    ol { class: "note-outline", role: "list", aria_label: "Headings",
                        for (index, heading) in summary.read().headings.iter().enumerate() {
                            li { key: "{index}",
                                style: "--heading-depth: {heading.depth}",
                                "data-child": heading.depth > 0,
                                Button { disabled: busy,
                                    onclick: { let offset = heading.offset; move |_| on_jump.call((index, offset)) },
                                    "{heading.title()}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
