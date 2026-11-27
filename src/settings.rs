//! App Settings for appearance and the editor.

use dioxus::prelude::*;

use crate::appearance::{ScenePreferences, SceneSelect};
use crate::cursor::{CursorOptions, CursorSettings};
use crate::font_size::{FontPreference, FontSizeSelect};
use crate::modal::show_dialog;
use crate::page_width::{PageWidth, PageWidthSelect};
use crate::theme::{Appearance, ThemeSelect};
use crate::writing_style::{WritingStyle, WritingStyleSettings};

#[component]
pub(crate) fn Settings(
    cursor: Signal<Option<CursorOptions>>,
    mut page_width: Signal<PageWidth>,
    appearance: Signal<Appearance>,
    dark: Signal<bool>,
    scenes: Signal<ScenePreferences>,
    settings_font: Signal<u8>,
    markdown_font: Signal<u8>,
    writing_style: Signal<WritingStyle>,
    on_close: EventHandler<()>,
) -> Element {
    let font_size = settings_font();
    rsx! {
        document::Stylesheet { href: asset!("/assets/settings.css") }
        dialog { id: "app-settings", class: "app-dialog", aria_labelledby: "settings-title",
            style: "font-size: {font_size}px",
            onmounted: move |_| show_dialog("app-settings", on_close),
            h1 { id: "settings-title", "Settings" }
            ThemeSelect { appearance }
            SceneSelect { dark: dark(), preferences: scenes }
            FontSizeSelect { preference: FontPreference::Settings, size: settings_font }
            FontSizeSelect { preference: FontPreference::Markdown, size: markdown_font }
            WritingStyleSettings { style: writing_style }
            label {
                span { "Writing width" }
                PageWidthSelect {
                    label: "Writing width",
                    width: page_width(),
                    on_change: move |width| page_width.set(width),
                }
            }
            CursorSettings { cursor }
            form { method: "dialog",
                button { r#type: "submit", "Done" }
            }
        }
    }
}
