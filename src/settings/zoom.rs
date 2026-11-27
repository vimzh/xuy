//! Persistent native webview zoom and its keyboard shortcuts.

use dioxus::desktop::use_window;
use dioxus::prelude::{dioxus_core::use_drop, *};

#[cfg(target_os = "macos")]
use objc2_foundation::{NSString, NSUserDefaults};

#[cfg(target_os = "macos")]
const KEY: &str = "xuy.zoom-percent";
const MIN: i32 = 50;
const MAX: i32 = 200;
const DEFAULT: i32 = 100;
const STEP: i32 = 10;

#[derive(Clone, Copy)]
pub(crate) enum ZoomAction {
    In,
    Out,
    Reset,
}

impl ZoomAction {
    fn percent(self, current: i32) -> i32 {
        match self {
            Self::In => (current + STEP).min(MAX),
            Self::Out => (current - STEP).max(MIN),
            Self::Reset => DEFAULT,
        }
    }
}

fn load() -> i32 {
    #[cfg(target_os = "macos")]
    {
        let defaults = NSUserDefaults::standardUserDefaults();
        let key = NSString::from_str(KEY);
        if defaults.objectForKey(&key).is_some() {
            let saved = defaults.integerForKey(&key);
            if let Ok(percent) = i32::try_from(saved)
                && (MIN..=MAX).contains(&percent)
                && percent % STEP == 0
            {
                return percent;
            }
            eprintln!("Could not restore zoom: expected a 10% step from {MIN}% through {MAX}%");
        }
    }
    DEFAULT
}

pub(crate) fn use_zoom_shortcuts() -> (Signal<i32>, Callback<ZoomAction>) {
    let window = use_window();
    let mut bridge = use_signal(|| None::<document::Eval>);
    let mut zoom = use_signal(load);
    let set_zoom = use_callback(move |action: ZoomAction| {
        let current = *zoom.peek();
        let next = action.percent(current);
        if next != current {
            #[cfg(target_os = "macos")]
            NSUserDefaults::standardUserDefaults()
                .setInteger_forKey(next as isize, &NSString::from_str(KEY));
            zoom.set(next);
        }
    });
    use_effect(move || window.set_zoom_level(f64::from(zoom()) / 100.0));
    use_effect(move || {
        let mut keys = document::eval(include_str!("zoom_shortcuts.js"));
        bridge.set(Some(keys));
        spawn(async move {
            while let Ok(zoom_in) = keys.recv::<bool>().await {
                set_zoom.call(if zoom_in {
                    ZoomAction::In
                } else {
                    ZoomAction::Out
                });
            }
        });
    });
    use_drop(move || {
        if let Some(keys) = *bridge.peek() {
            let _ = keys.send(());
        }
    });
    (zoom, set_zoom)
}

#[cfg(test)]
mod tests {
    use super::ZoomAction;

    #[test]
    fn clamps_steps_and_resets_to_default() {
        assert_eq!(ZoomAction::In.percent(100), 110);
        assert_eq!(ZoomAction::Out.percent(100), 90);
        assert_eq!(ZoomAction::In.percent(200), 200);
        assert_eq!(ZoomAction::Out.percent(50), 50);
        assert_eq!(ZoomAction::Reset.percent(140), 100);
    }
}
