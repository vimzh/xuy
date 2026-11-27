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
    rsx! {
        document::Stylesheet { href: asset!("/assets/app.css") }
        main {
            h1 { "xuy" }
            nav { aria_label: "Notebook actions",
                button { disabled: true, "new" }
                button { disabled: true, "open" }
                button { disabled: true, "settings" }
                button { disabled: true, "docs" }
            }
            section { aria_labelledby: "recent-heading",
                h2 { id: "recent-heading", "Recently Opened" }
            }
        }
    }
}
