//! The notebook title bar and its native macOS window controls.

use std::path::PathBuf;

use dioxus::desktop::use_window;
use dioxus::prelude::*;
use dioxus_elements::input_data::MouseButton;

use crate::editor::ViewMode;
use crate::notebook::display_name;
use crate::settings::page_width::{PageWidth, PageWidthSelect};
use crate::ui::{Button, Dropdown};

#[component]
pub(crate) fn NotebookChrome(
    filename: String,
    dirty: bool,
    mut hidden: Signal<bool>,
    sidebar_open: bool,
    writing_sidebar_open: Option<bool>,
    can_back: bool,
    can_forward: bool,
    folder: PathBuf,
    parent: Option<PathBuf>,
    busy: bool,
    on_parent: EventHandler<PathBuf>,
    mut page_width: Signal<PageWidth>,
    view_mode: ViewMode,
    can_preview: bool,
    on_view_mode: EventHandler<ViewMode>,
    on_toggle: EventHandler<()>,
    on_toggle_writing_sidebar: EventHandler<()>,
    on_back: EventHandler<()>,
    on_forward: EventHandler<()>,
) -> Element {
    let window = use_window();
    #[cfg(target_os = "macos")]
    macos::use_titlebar(window.window.clone(), hidden);

    let icons = asset!("/assets/hugeicons", AssetOptions::folder());
    let folder_name = display_name(&folder);
    let sidebar_label = if sidebar_open {
        "Hide sidebar"
    } else {
        "Show sidebar"
    };
    let chrome_hidden = hidden();
    let writing_sidebar_label = if writing_sidebar_open == Some(true) {
        "Hide right sidebar"
    } else {
        "Show right sidebar"
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/chrome.css") }
        header { class: "notebook-chrome",
            div { class: "chrome-strip",
                if chrome_hidden {
                    div { class: "chrome-reveal", onmousemove: move |_| hidden.set(false) }
                } else {
                    div { class: "chrome-drag", onmousedown: move |event| {
                        if event.trigger_button() == Some(MouseButton::Primary) {
                            window.drag();
                        }
                    } }
                }
                span { class: "chrome-sidebar-rule", aria_hidden: true }
                if !chrome_hidden {
                    div { class: "chrome-left",
                        Button {
                            aria_label: sidebar_label,
                            aria_expanded: sidebar_open,
                            aria_controls: "workspace-sidebar",
                            aria_keyshortcuts: "Meta+B",
                            title: sidebar_label,
                            onclick: move |_| on_toggle.call(()),
                            img { src: "{icons}/sidebar-left.svg", alt: "", width: "18", height: "18" }
                        }
                        div { class: "chrome-history",
                            Button {
                                aria_label: "Back",
                                title: "Back",
                                disabled: !can_back,
                                onclick: move |_| on_back.call(()),
                                img { src: "{icons}/arrow-left-01.svg", alt: "", width: "18", height: "18" }
                            }
                            Button {
                                aria_label: "Forward",
                                title: "Forward",
                                disabled: !can_forward,
                                onclick: move |_| on_forward.call(()),
                                img { src: "{icons}/arrow-right-01.svg", alt: "", width: "18", height: "18" }
                            }
                        }
                        div { class: "chrome-folder",
                            if let Some(parent) = parent {
                                Button {
                                    aria_label: "Up to parent folder",
                                    title: "Up to parent folder",
                                    disabled: busy,
                                    onclick: move |_| on_parent.call(parent.clone()),
                                    img { src: "{icons}/arrow-up-01.svg", alt: "", width: "14", height: "14" }
                                }
                            } else {
                                img { src: "{icons}/folder-open.svg", alt: "", width: "14", height: "14" }
                            }
                            span { title: "{folder.display()}", "{folder_name}" }
                        }
                    }
                    div { class: "chrome-right", role: "group", aria_label: "Writing controls",
                        Dropdown { icon: "{icons}/arrow-horizontal.svg",
                            PageWidthSelect {
                                label: "Writing width",
                                width: page_width(),
                                on_change: move |width| page_width.set(width),
                            }
                        }
                        Dropdown { icon: "{icons}/view.svg", active: view_mode != ViewMode::Edit,
                            select {
                                aria_label: "View mode",
                                title: "View mode",
                                disabled: !can_preview,
                                onchange: move |event| {
                                    let selected = event.value();
                                    if let Some((mode, _, _)) = ViewMode::OPTIONS
                                        .into_iter().find(|&(_, value, _)| value == selected)
                                    {
                                        on_view_mode.call(mode);
                                    }
                                },
                                for (mode, value, label) in ViewMode::OPTIONS {
                                    option { key: "{value}", value, selected: view_mode == mode, "{label}" }
                                }
                            }
                        }
                        Button {
                            aria_label: writing_sidebar_label,
                            aria_expanded: writing_sidebar_open,
                            aria_controls: writing_sidebar_open.map(|_| "writing-sidebar"),
                            title: writing_sidebar_label,
                            disabled: writing_sidebar_open.is_none(),
                            onclick: move |_| on_toggle_writing_sidebar.call(()),
                            img { class: "chrome-right-sidebar-icon", src: "{icons}/sidebar-left.svg", alt: "" }
                        }
                    }
                    div { class: "chrome-filename",
                        span { class: "chrome-filename-text", "data-dirty": dirty, "{filename}" }
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::sync::Arc;

    use dioxus::desktop::tao::{platform::macos::WindowExtMacOS, window::Window};
    use dioxus::prelude::{dioxus_core::use_hook_with_cleanup, *};
    use objc2_app_kit::{NSWindow, NSWindowButton, NSWindowTitleVisibility};

    pub(super) fn use_titlebar(window: Arc<Window>, hidden: Signal<bool>) {
        let window = use_hook_with_cleanup(
            move || {
                window.set_fullsize_content_view(true);
                window.set_titlebar_transparent(true);
                native_window(&window).setTitleVisibility(NSWindowTitleVisibility::Hidden);
                window
            },
            |window| {
                let native = native_window(&window);
                set_traffic_lights(native, false);
                native.setTitleVisibility(NSWindowTitleVisibility::Visible);
                window.set_titlebar_transparent(false);
                window.set_fullsize_content_view(false);
            },
        );
        use_effect(move || set_traffic_lights(native_window(&window), hidden()));
    }

    fn native_window(window: &Window) -> &NSWindow {
        // Tao owns this NSWindow, and Dioxus runs desktop hooks on the main thread.
        unsafe { &*window.ns_window().cast::<NSWindow>() }
    }

    fn set_traffic_lights(window: &NSWindow, hidden: bool) {
        for control in [
            NSWindowButton::CloseButton,
            NSWindowButton::MiniaturizeButton,
            NSWindowButton::ZoomButton,
        ] {
            if let Some(button) = window.standardWindowButton(control) {
                button.setHidden(hidden);
            }
        }
    }
}
