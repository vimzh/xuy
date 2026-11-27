//! The notebook title bar and its native macOS window controls.

use dioxus::desktop::use_window;
use dioxus::prelude::*;
use dioxus_elements::input_data::MouseButton;

#[component]
pub(crate) fn NotebookChrome(
    filename: String,
    mut hidden: Signal<bool>,
    sidebar_open: bool,
    can_back: bool,
    can_forward: bool,
    on_toggle: EventHandler<()>,
    on_back: EventHandler<()>,
    on_forward: EventHandler<()>,
) -> Element {
    let window = use_window();
    #[cfg(target_os = "macos")]
    macos::use_titlebar(window.window.clone(), hidden);

    let icons = asset!("/assets/hugeicons", AssetOptions::folder());
    let sidebar_label = if sidebar_open {
        "Hide sidebar"
    } else {
        "Show sidebar"
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/chrome.css") }
        header { class: "notebook-chrome",
            if hidden() {
                div { class: "chrome-reveal", onmousemove: move |_| hidden.set(false) }
            } else {
                div { class: "chrome-strip",
                    div { class: "chrome-drag", onmousedown: move |event| {
                        if event.trigger_button() == Some(MouseButton::Primary) {
                            window.drag();
                        }
                    } }
                    if sidebar_open {
                        span { class: "chrome-sidebar-rule", aria_hidden: true }
                    }
                    div { class: "chrome-left",
                        button {
                            r#type: "button",
                            aria_label: sidebar_label,
                            aria_expanded: sidebar_open,
                            aria_controls: "workspace-sidebar",
                            title: sidebar_label,
                            onclick: move |_| on_toggle.call(()),
                            img { src: "{icons}/sidebar-left.svg", alt: "", width: "18", height: "18" }
                        }
                        div { class: "chrome-history",
                            button {
                                r#type: "button",
                                aria_label: "Back",
                                title: "Back",
                                disabled: !can_back,
                                onclick: move |_| on_back.call(()),
                                img { src: "{icons}/arrow-left-01.svg", alt: "", width: "18", height: "18" }
                            }
                            button {
                                r#type: "button",
                                aria_label: "Forward",
                                title: "Forward",
                                disabled: !can_forward,
                                onclick: move |_| on_forward.call(()),
                                img { src: "{icons}/arrow-right-01.svg", alt: "", width: "18", height: "18" }
                            }
                        }
                    }
                    span { class: "chrome-filename", "{filename}" }
                    button {
                        class: "chrome-settings",
                        r#type: "button",
                        aria_label: "Page configuration",
                        title: "Page configuration",
                        disabled: true,
                        img { src: "{icons}/settings-01.svg", alt: "", width: "18", height: "18" }
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
