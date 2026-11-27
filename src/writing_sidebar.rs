//! The secondary sidebar for heading navigation and the highlight palette.

use dioxus::prelude::*;

use crate::{highlight::Color, note_status::NoteSummary};

#[component]
pub(crate) fn WritingSidebar(
    open: bool,
    summary: Memo<NoteSummary>,
    preview: bool,
    busy: bool,
    feedback: Signal<Option<&'static str>>,
    on_jump: EventHandler<(usize, usize)>,
    on_highlight: EventHandler<Option<Color>>,
) -> Element {
    let mut show_highlights = use_signal(|| false);
    rsx! {
        aside { id: "writing-sidebar", class: "writing-sidebar", aria_label: "Note tools", hidden: !open,
            div { class: "writing-modes", role: "group", aria_label: "Note tools view",
                button { r#type: "button", aria_pressed: !show_highlights(),
                    aria_controls: "writing-tools-view", onclick: move |_| show_highlights.set(false), "Outline" }
                button { r#type: "button", aria_pressed: show_highlights(),
                    aria_controls: "writing-tools-view", onclick: move |_| show_highlights.set(true), "Highlights" }
            }
            div { id: "writing-tools-view", class: "writing-tools-view",
                if show_highlights() {
                    p { class: "writing-hint",
                        if preview { "Switch to Edit to highlight text." }
                        else { "Select text, then choose a color." }
                    }
                    div { class: "highlight-colors", role: "group", aria_label: "Highlight color",
                        for color in Color::ALL {
                            button { key: "{color.class()}", r#type: "button", disabled: preview || busy,
                                onclick: move |_| on_highlight.call(Some(color)),
                                span { class: "highlight-swatch {color.class()}", aria_hidden: true }
                                "{color.name()}"
                            }
                        }
                    }
                    button { class: "highlight-clear", r#type: "button", disabled: preview || busy,
                        onclick: move |_| on_highlight.call(None), "Clear highlight" }
                    if let Some(message) = feedback() {
                        p { class: "writing-hint", role: "status", "{message}" }
                    }
                } else if summary.read().headings.is_empty() {
                    p { class: "writing-hint", "No headings" }
                } else {
                    ol { class: "note-outline", role: "list", aria_label: "Headings",
                        for (index, heading) in summary.read().headings.iter().enumerate() {
                            li { key: "{index}",
                                style: "--heading-depth: {heading.depth}",
                                "data-child": heading.depth > 0,
                                button { r#type: "button", disabled: busy,
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
