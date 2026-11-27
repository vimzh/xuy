use dioxus::desktop::{Config, WindowBuilder, tao::window::Theme};
use dioxus::prelude::*;

fn main() {
    let window = WindowBuilder::new()
        .with_title("xuy")
        .with_theme(Some(Theme::Light));

    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window))
        .launch(App);
}

#[component]
fn App() -> Element {
    let icons = asset!("/assets/hugeicons", AssetOptions::folder());
    let actions = [
        ("new", "⌘N", "add-01"),
        ("open", "⌘O", "folder-open"),
        ("settings", "⌘,", "settings-01"),
        ("docs", "⌘D", "book-open-01"),
    ];

    rsx! {
        document::Stylesheet { href: asset!("/assets/app.css") }
        main {
            h1 { "xuy" }
            nav { aria_label: "Notebook actions",
                for (label, shortcut, icon) in actions {
                    button { key: "{label}", disabled: true,
                        img { src: "{icons}/{icon}.svg", alt: "", width: "16", height: "16" }
                        span { "{label}" }
                        kbd { aria_hidden: true, "{shortcut}" }
                    }
                }
            }
            section { aria_labelledby: "recent-heading",
                h2 { id: "recent-heading", "Recently Opened" }
            }
        }
    }
}
