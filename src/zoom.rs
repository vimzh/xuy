//! Keyboard shortcuts for native webview page zoom.

use dioxus::desktop::use_window;
use dioxus::prelude::{dioxus_core::use_drop, *};

#[derive(Clone, Copy)]
pub(crate) enum ZoomAction {
    In,
    Out,
    Reset,
}

impl ZoomAction {
    fn percent(self, current: i32) -> i32 {
        match self {
            Self::In => (current + 10).min(200),
            Self::Out => (current - 10).max(50),
            Self::Reset => 100,
        }
    }
}

pub(crate) fn use_zoom_shortcuts() -> (Signal<i32>, Callback<ZoomAction>) {
    let window = use_window();
    let mut bridge = use_signal(|| None::<document::Eval>);
    let mut zoom = use_signal(|| 100_i32);
    let set_zoom = use_callback(move |action: ZoomAction| {
        let current = *zoom.peek();
        let next = action.percent(current);
        if next != current {
            window.set_zoom_level(f64::from(next) / 100.0);
            zoom.set(next);
        }
    });
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
