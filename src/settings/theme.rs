//! Persistent appearance preference and its native macOS menu.

use std::cell::RefCell;

use dioxus::desktop::{
    muda::{CheckMenuItem, IsMenuItem, Submenu},
    tao::{
        event::{Event, WindowEvent},
        window::Theme,
    },
    use_muda_event_handler, use_window, use_wry_event_handler,
};
use dioxus::prelude::*;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, NSUserDefaults};

#[cfg(target_os = "macos")]
const THEME_KEY: &str = "xuy.appearance";

#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[repr(isize)]
pub(crate) enum Appearance {
    #[default]
    System = 0,
    Light = 1,
    Dark = 2,
}

pub(crate) const OPTIONS: [(Appearance, &str, &str); 3] = [
    (Appearance::System, "xuy-theme-system", "System"),
    (Appearance::Light, "xuy-theme-light", "Light"),
    (Appearance::Dark, "xuy-theme-dark", "Dark"),
];

thread_local! {
    static ITEMS: RefCell<Option<[CheckMenuItem; OPTIONS.len()]>> = const { RefCell::new(None) };
}

impl Appearance {
    pub(crate) fn native(self) -> Option<Theme> {
        match self {
            Self::System => None,
            Self::Light => Some(Theme::Light),
            Self::Dark => Some(Theme::Dark),
        }
    }

    pub(crate) fn load() -> Self {
        #[cfg(target_os = "macos")]
        {
            let value = NSUserDefaults::standardUserDefaults()
                .integerForKey(&NSString::from_str(THEME_KEY));
            match value {
                0 => Self::System,
                1 => Self::Light,
                2 => Self::Dark,
                _ => {
                    eprintln!("Could not restore appearance: unknown value {value}");
                    Self::System
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        Self::System
    }

    pub(crate) fn select(self, mut selected: Signal<Self>) {
        if *selected.peek() == self {
            return;
        }
        #[cfg(target_os = "macos")]
        NSUserDefaults::standardUserDefaults()
            .setInteger_forKey(self as isize, &NSString::from_str(THEME_KEY));
        selected.set(self);
    }
}

pub(crate) fn menu(initial: Appearance) -> Submenu {
    let items = OPTIONS.map(|(appearance, id, label)| {
        CheckMenuItem::with_id(id, label, true, appearance == initial, None)
    });
    let submenu = Submenu::with_items(
        "Appearance",
        true,
        &items.each_ref().map(|item| item as &dyn IsMenuItem),
    )
    .expect("build appearance menu");
    ITEMS.set(Some(items));
    submenu
}

pub(crate) fn use_appearance() -> (Signal<Appearance>, Signal<bool>) {
    let window = use_window();
    let selected = use_signal(Appearance::load);
    let mut dark = use_signal(|| window.theme() == Theme::Dark);
    let window_id = window.id();
    use_wry_event_handler(move |event, _| {
        if let Event::WindowEvent {
            window_id: source,
            event: WindowEvent::ThemeChanged(theme),
            ..
        } = event
            && *source == window_id
        {
            dark.set(*theme == Theme::Dark);
        }
    });
    use_muda_event_handler(move |event| {
        if let Some(appearance) = appearance_for_id(&event.id().0) {
            appearance.select(selected);
        }
    });
    use_effect(move || {
        let appearance = selected();
        window.set_theme(appearance.native());
        dark.set(window.theme() == Theme::Dark);
        ITEMS.with_borrow(|items| {
            if let Some(items) = items {
                for (item, (option, _, _)) in items.iter().zip(OPTIONS) {
                    item.set_checked(option == appearance);
                }
            }
        });
    });
    (selected, dark)
}

pub(crate) fn appearance_for_id(id: &str) -> Option<Appearance> {
    OPTIONS
        .iter()
        .find_map(|(appearance, option_id, _)| (*option_id == id).then_some(*appearance))
}

#[component]
pub(crate) fn ThemeSelect(appearance: Signal<Appearance>) -> Element {
    let selected = appearance();
    rsx! {
        label {
            span { "Theme" }
            select {
                onchange: move |event| {
                    if let Some(option) = appearance_for_id(&event.value()) {
                        option.select(appearance);
                    }
                },
                for (option, id, label) in OPTIONS {
                    option { value: id, selected: option == selected, "{label}" }
                }
            }
        }
    }
}
