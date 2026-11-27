mod chrome;
mod editor;
mod note_history;
mod notebook;
mod notebook_menu;
mod workspace;
mod workspace_sidebar;

use dioxus::desktop::{Config, WindowBuilder, tao::window::Theme};
use dioxus::prelude::*;

fn main() {
    let window = WindowBuilder::new()
        .with_title("xuy")
        .with_theme(Some(Theme::Light));

    LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_menu(notebook_menu::build()),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    let mut opened = use_signal(|| None::<workspace::OpenNotebook>);
    let mut opening = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let icons = asset!("/assets/hugeicons", AssetOptions::folder());
    let mono = asset!("/assets/ia-writer-mono", AssetOptions::folder());
    let actions = [
        ("new", "N", "add-01"),
        ("open", "O", "folder-open"),
        ("settings", ",", "settings-01"),
        ("docs", "D", "book-open-01"),
    ];
    let choose = move |start_new: bool| async move {
        if *opening.peek() {
            return;
        }
        opening.set(true);
        error.set(None);
        match workspace::pick_notebook(start_new).await {
            Ok(Some(notebook)) => opened.set(Some(notebook)),
            Ok(None) => {}
            Err(failure) => error.set(Some(failure.to_string())),
        }
        opening.set(false);
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/app.css") }
        document::Stylesheet { href: "{mono}/fonts.css" }
        if let Some(notebook) = opened() {
            workspace::Workspace { opened: notebook, on_home: move |_| opened.set(None) }
        } else {
            main { class: "landing", aria_busy: opening(),
                pre { class: "black-hole", aria_hidden: true,
                    "data-license": asset!("/assets/ascii-rest/LICENSE.txt"),
                    {include_str!("../assets/ascii-rest/black-hole.txt")}
                }
                h1 { "xuy" }
                nav { aria_label: "Notebook actions",
                    for (label, shortcut, icon) in actions {
                        button { key: "{label}",
                            disabled: opening() || !matches!(label, "new" | "open"),
                            onclick: move |_| async move { choose(label == "new").await },
                            img { src: "{icons}/{icon}.svg", alt: "", width: "16", height: "16" }
                            span { "{label}" }
                            kbd { aria_hidden: true,
                                span { class: "command-key", "⌘" }
                                span { "{shortcut}" }
                            }
                        }
                    }
                }
                if let Some(message) = error() {
                    p { role: "alert", "{message}" }
                }
                section { aria_labelledby: "recent-heading",
                    h2 { id: "recent-heading", "Recently Opened" }
                }
            }
        }
    }
}
