//! Keyboard shortcuts for native webview page zoom.

use dioxus::desktop::use_window;
use dioxus::prelude::{dioxus_core::use_drop, *};

pub(crate) fn use_zoom_shortcuts() -> Signal<i32> {
    let window = use_window();
    let mut bridge = use_signal(|| None::<document::Eval>);
    let mut zoom = use_signal(|| 100_i32);
    use_effect(move || {
        let window = window.clone();
        let mut keys = document::eval(include_str!("zoom_shortcuts.js"));
        bridge.set(Some(keys));
        spawn(async move {
            while let Ok(zoom_in) = keys.recv::<bool>().await {
                let percent = *zoom.peek();
                let step = if zoom_in { 10 } else { -10 };
                let next = (percent + step).clamp(50, 200);
                if next != percent {
                    window.set_zoom_level(f64::from(next) / 100.0);
                    zoom.set(next);
                }
            }
        });
    });
    use_drop(move || {
        if let Some(keys) = *bridge.peek() {
            let _ = keys.send(());
        }
    });
    zoom
}
