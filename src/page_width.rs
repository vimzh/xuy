//! Shared writing-width presets and their native picker.

use dioxus::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum PageWidth {
    Narrow,
    #[default]
    Standard,
    Wide,
}

impl PageWidth {
    const OPTIONS: [(Self, &'static str, u8); 3] = [
        (Self::Narrow, "Narrow", 56),
        (Self::Standard, "Standard", 60),
        (Self::Wide, "Wide", 82),
    ];

    pub(crate) fn columns(self) -> u8 {
        Self::OPTIONS[self as usize].2
    }
}

#[component]
pub(crate) fn PageWidthSelect(
    label: &'static str,
    width: PageWidth,
    on_change: EventHandler<PageWidth>,
) -> Element {
    rsx! {
        select {
            aria_label: label,
            title: label,
            onchange: move |event| {
                let value = event.value();
                if let Some(&(width, _, _)) = PageWidth::OPTIONS.iter().find(|(_, label, _)| *label == value) {
                    on_change.call(width);
                }
            },
            for (option, label, _) in PageWidth::OPTIONS {
                option { value: label, selected: option == width, "{label}" }
            }
        }
    }
}
