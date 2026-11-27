//! Independent text-size preferences for Settings and Markdown.

use dioxus::prelude::*;

use crate::commands::palette::Command;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, NSUserDefaults};

const SIZES: [(u8, &str, [&str; 2]); 8] = [
    (12, "12 px", ["settings-font-12", "markdown-font-12"]),
    (14, "14 px", ["settings-font-14", "markdown-font-14"]),
    (15, "15 px", ["settings-font-15", "markdown-font-15"]),
    (16, "16 px", ["settings-font-16", "markdown-font-16"]),
    (18, "18 px", ["settings-font-18", "markdown-font-18"]),
    (20, "20 px", ["settings-font-20", "markdown-font-20"]),
    (22, "22 px", ["settings-font-22", "markdown-font-22"]),
    (24, "24 px", ["settings-font-24", "markdown-font-24"]),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FontPreference {
    Settings,
    Markdown,
}

impl FontPreference {
    fn label(self) -> &'static str {
        match self {
            Self::Settings => "Settings font size",
            Self::Markdown => "Markdown font size",
        }
    }

    #[cfg(target_os = "macos")]
    fn key(self) -> &'static str {
        match self {
            Self::Settings => "xuy.settings-font-size",
            Self::Markdown => "xuy.markdown-font-size",
        }
    }

    pub(crate) fn load(self) -> u8 {
        #[cfg(target_os = "macos")]
        {
            let saved = NSUserDefaults::standardUserDefaults()
                .integerForKey(&NSString::from_str(self.key()));
            if let Ok(size) = u8::try_from(saved)
                && SIZES.iter().any(|(option, _, _)| *option == size)
            {
                return size;
            }
        }
        15
    }

    fn set(self, mut size: Signal<u8>, value: u8) {
        size.set(value);
        #[cfg(target_os = "macos")]
        NSUserDefaults::standardUserDefaults()
            .setInteger_forKey(value.into(), &NSString::from_str(self.key()));
    }

    pub(crate) fn commands(self, current: u8) -> impl Iterator<Item = Command> {
        SIZES.into_iter().map(move |(size, label, ids)| {
            Command::new(
                ids[self as usize],
                self.label(),
                "settings-01",
                "",
                "text size",
            )
            .setting_value(label, current == size)
        })
    }

    pub(crate) fn apply_command(self, id: &str, size: Signal<u8>) -> bool {
        let Some((value, _, _)) = SIZES.iter().find(|(_, _, ids)| ids[self as usize] == id) else {
            return false;
        };
        self.set(size, *value);
        true
    }
}

#[component]
pub(crate) fn FontSizeSelect(preference: FontPreference, size: Signal<u8>) -> Element {
    let current = size();
    rsx! {
        label {
            span { "{preference.label()}" }
            select {
                onchange: move |event| {
                    if let Ok(value) = event.value().parse::<u8>()
                        && SIZES.iter().any(|(option, _, _)| *option == value)
                    {
                        preference.set(size, value);
                    }
                },
                for (option, label, _) in SIZES {
                    option { value: option.to_string(), selected: current == option, "{label}" }
                }
            }
        }
    }
}
