//! Shared icon trigger and styling for a labeled select.

use dioxus::prelude::*;

#[component]
pub(crate) fn Dropdown(icon: String, #[props(default)] active: bool, children: Element) -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/dropdown.css") }
        span { class: "dropdown", "data-active": active,
            img { src: icon, alt: "", width: "18", height: "18" }
            {children}
        }
    }
}
