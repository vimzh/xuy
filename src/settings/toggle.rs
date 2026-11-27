//! Persistent boolean preferences for workspace layout controls.

use dioxus::prelude::*;

#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, NSUserDefaults};

pub(crate) fn use_saved_toggle(_key: &'static str, initial: bool) -> (Signal<bool>, Callback<()>) {
    let mut value = use_signal(move || {
        #[cfg(target_os = "macos")]
        {
            let defaults = NSUserDefaults::standardUserDefaults();
            let key = NSString::from_str(_key);
            if defaults.objectForKey(&key).is_some() {
                return defaults.boolForKey(&key);
            }
        }
        initial
    });
    let toggle = use_callback(move |()| {
        let next = !*value.peek();
        #[cfg(target_os = "macos")]
        NSUserDefaults::standardUserDefaults().setBool_forKey(next, &NSString::from_str(_key));
        value.set(next);
    });
    (value, toggle)
}
