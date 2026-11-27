//! Shared writing-width presets and their native picker.

use dioxus::prelude::*;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, NSUserDefaults};

#[cfg(target_os = "macos")]
const KEY: &str = "xuy.page-width";

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum PageWidth {
    Narrow,
    #[default]
    Standard,
    Wide,
}

impl PageWidth {
    pub(crate) const OPTIONS: [(Self, &'static str, &'static str, u8); 3] = [
        (Self::Narrow, "width-narrow", "Narrow", 60),
        (Self::Standard, "width-standard", "Standard", 82),
        (Self::Wide, "width-wide", "Wide", 104),
    ];

    pub(crate) fn columns(self) -> u8 {
        let (_, _, _, columns) = Self::OPTIONS[self as usize];
        columns
    }

    pub(crate) fn for_command(id: &str) -> Option<Self> {
        Self::OPTIONS
            .iter()
            .find_map(|&(width, option_id, _, _)| (option_id == id).then_some(width))
    }

    pub(crate) fn load() -> Self {
        #[cfg(target_os = "macos")]
        if let Some(value) =
            NSUserDefaults::standardUserDefaults().objectForKey(&NSString::from_str(KEY))
        {
            if let Ok(id) = value.downcast::<NSString>()
                && let Some(width) = Self::for_command(&id.to_string())
            {
                return width;
            }
            eprintln!("Could not restore writing width: unknown preset");
        }
        Self::default()
    }

    pub(crate) fn set(self, mut width: Signal<Self>) {
        if *width.peek() == self {
            return;
        }
        #[cfg(target_os = "macos")]
        {
            let (_, id, _, _) = Self::OPTIONS[self as usize];
            // The command ID is a stable property-list string.
            unsafe {
                NSUserDefaults::standardUserDefaults()
                    .setObject_forKey(Some(&NSString::from_str(id)), &NSString::from_str(KEY));
            }
        }
        width.set(self);
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
                if let Some(&(width, _, _, _)) = PageWidth::OPTIONS.iter().find(|(_, _, label, _)| *label == value) {
                    on_change.call(width);
                }
            },
            for (option, _, label, _) in PageWidth::OPTIONS {
                option { value: label, selected: option == width, "{label}" }
            }
        }
    }
}
