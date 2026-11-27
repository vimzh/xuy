//! Notebook terminal panel and the bridge between xterm and the local shell.

use std::path::PathBuf;

use dioxus::prelude::{dioxus_core::use_drop, *};
use tokio::sync::mpsc;

mod pty;

pub(crate) const OPEN: &str = "terminal-open";
pub(crate) const MINIMIZE: &str = "terminal-minimize";
pub(crate) const CLOSE: &str = "terminal-close";

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum State {
    #[default]
    Closed,
    Visible,
    Minimized,
}

#[component]
pub(crate) fn TerminalPanel(mut state: Signal<State>, folder: PathBuf, zoom: f64) -> Element {
    let mut bridge = use_signal(|| None::<document::Eval>);
    let mut status = use_signal(|| None::<String>);
    use_drop(move || {
        if let Some(eval) = *bridge.peek() {
            let _ = eval.send(Option::<Vec<u8>>::None);
        }
    });
    let visible = state() == State::Visible;
    rsx! {
        document::Stylesheet { href: asset!("/assets/xterm/xterm.css") }
        section { id: "terminal-panel", class: "terminal-panel", hidden: !visible, aria_label: "Notebook terminal",
            header { class: "terminal-header",
                span { "Terminal" }
                if let Some(message) = status() { span { class: "terminal-status", role: "status", "{message}" } }
                button { r#type: "button", onclick: move |_| state.set(State::Minimized), "Minimize" }
                button { r#type: "button", onclick: move |_| state.set(State::Closed), "Close" }
            }
            div { id: "notebook-terminal", class: "terminal-screen", "data-zoom": "{zoom}",
                "data-module": asset!("/assets/xterm/xterm.js"),
                "data-xterm-license": asset!("/assets/xterm/LICENSE-xterm.txt"),
                "data-fit-license": asset!("/assets/xterm/LICENSE-addon-fit.txt"),
                onmounted: move |_| {
                    let folder = folder.clone();
                    async move {
                        let eval = document::eval(include_str!("terminal.js"));
                        bridge.set(Some(eval));
                        if let Err(error) = connect(eval, folder, status).await { status.set(Some(error)); }
                    }
                },
            }
        }
    }
}

type Message = (String, String, u16, u16);

async fn connect(
    mut eval: document::Eval,
    folder: PathBuf,
    mut status: Signal<Option<String>>,
) -> Result<(), String> {
    let (kind, data, cols, rows) = eval
        .recv::<Message>()
        .await
        .map_err(|error| error.to_string())?;
    if kind != "ready" {
        return Err(data);
    }
    let (send, mut receive) = mpsc::channel(8);
    let session =
        tokio::task::spawn_blocking(move || pty::Session::start(&folder, cols, rows, send))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
    let mut writing = false;
    let mut live = true;
    loop {
        tokio::select! {
            message = eval.recv::<Message>() => {
                let (kind, data, cols, rows) = message.map_err(|error| error.to_string())?;
                match kind.as_str() {
                    "painted" => writing = false,
                    "input" if live => session.input(data.into_bytes())?,
                    "binary" if live => session.input(data.chars().map(|byte| byte as u8).collect())?,
                    "resize" if live => session.resize(cols, rows)?,
                    "error" => return Err(data),
                    _ => {}
                }
            }
            Some(output) = receive.recv(), if !writing => match output {
                pty::Output::Bytes(bytes) => {
                    eval.send(bytes).map_err(|error| error.to_string())?;
                    writing = true;
                }
                pty::Output::Exit(message) => { live = false; status.set(Some(message)); }
                pty::Output::Error(message) => return Err(message),
            },
        }
    }
}
