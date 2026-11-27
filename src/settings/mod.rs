//! App Settings for appearance and the editor.

pub(crate) mod cursor;
pub(crate) mod font_size;
pub(crate) mod page_width;
pub(crate) mod scene;
pub(crate) mod theme;
pub(crate) mod toggle;
pub(crate) mod writing_style;
pub(crate) mod zoom;

use dioxus::prelude::*;

use self::scene::{ScenePreferences, SceneSelect};
use crate::settings::cursor::{CursorOptions, CursorSettings};
use crate::settings::font_size::{FontPreference, FontSizeSelect};
use crate::settings::page_width::{PageWidth, PageWidthSelect};
use crate::settings::theme::{Appearance, ThemeSelect};
use crate::settings::writing_style::{WritingStyle, WritingStyleSettings};
use crate::ui::{Button, ButtonKind, Dialog, DialogScrim};

#[component]
pub(crate) fn Settings(
    cursor: Signal<Option<CursorOptions>>,
    page_width: Signal<PageWidth>,
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
        Dialog { id: "app-settings", class: "app-dialog", aria_labelledby: "settings-title",
            aria_modal: true,
            style: "font-size: {font_size}px",
            scrim: DialogScrim { id: "settings-scrim", focusable: "select:enabled, button:enabled" },
            on_close,
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
                    on_change: move |width: PageWidth| width.set(page_width),
                }
            }
            CursorSettings { cursor }
            form { method: "dialog",
                Button { kind: ButtonKind::Submit, "Done" }
            }
        }
    }
}
