//! Persistent appearance preference and its native macOS menu.

use std::cell::RefCell;

use dioxus::desktop::{
    muda::{CheckMenuItem, IsMenuItem, Submenu},
    tao::window::Theme,
    use_muda_event_handler, use_window,
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

    #[cfg(target_os = "macos")]
    fn save(self) {
        NSUserDefaults::standardUserDefaults()
            .setInteger_forKey(self as isize, &NSString::from_str(THEME_KEY));
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

pub(crate) fn use_appearance() -> Signal<Appearance> {
    let window = use_window();
    let mut selected = use_signal(Appearance::load);
    use_muda_event_handler(move |event| {
        if let Some(appearance) = appearance_for_id(&event.id().0) {
            selected.set(appearance);
        }
    });
    use_effect(move || {
        let appearance = selected();
        window.set_theme(appearance.native());
        #[cfg(target_os = "macos")]
        appearance.save();
        ITEMS.with_borrow(|items| {
            if let Some(items) = items {
                for (item, (option, _, _)) in items.iter().zip(OPTIONS) {
                    item.set_checked(option == appearance);
                }
            }
        });
    });

    selected
}

pub(crate) fn appearance_for_id(id: &str) -> Option<Appearance> {
    OPTIONS
        .iter()
        .find_map(|(appearance, option_id, _)| (*option_id == id).then_some(*appearance))
}
