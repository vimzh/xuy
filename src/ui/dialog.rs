//! Shared dialog lifecycle for native modals and WebKit-safe overlays.

use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct DialogScrim {
    pub(crate) id: &'static str,
    pub(crate) focusable: &'static str,
}

#[component]
pub(crate) fn Dialog(
    id: &'static str,
    scrim: Option<DialogScrim>,
    #[props(extends = GlobalAttributes, extends = dialog)] attributes: Vec<Attribute>,
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        if let Some(scrim) = scrim {
            div { id: scrim.id, aria_hidden: true, onclick: move |_| close_dialog(id) }
        }
        dialog {
            id,
            onmounted: move |_| async move {
                run_dialog(id, scrim).await;
                on_close.call(());
            },
            ..attributes,
            {children}
        }
    }
}

pub(crate) fn close_dialog(id: &'static str) {
    document::eval(&format!("document.getElementById({id:?})?.close()"));
}

async fn run_dialog(id: &'static str, scrim: Option<DialogScrim>) {
    let script = match scrim {
        Some(scrim) => format!(
            "const scrimId = {:?}; const focusable = {:?};\n{}",
            scrim.id,
            scrim.focusable,
            include_str!("dialog.js")
        ),
        None => "dialog.showModal(); await closed;".to_owned(),
    };
    let mut dialog = document::eval(&format!(
        "try {{
             const dialog = document.getElementById({id:?});
             dialog.addEventListener('cancel', event => {{
                 if (dialog.getAttribute('aria-busy') === 'true') event.preventDefault();
             }});
             const closed = new Promise(resolve => dialog.addEventListener('close', resolve, {{ once: true }}));
             {script}
             dioxus.send(null);
         }} catch (error) {{
             dioxus.send(String(error));
         }}
         await dioxus.recv();"
    ));
    let result = dialog.recv::<Option<String>>().await;
    let _ = dialog.send(());
    match result {
        Ok(None) => {}
        Ok(Some(error)) => eprintln!("Could not show {id}: {error}"),
        Err(error) => eprintln!("Could not show {id}: {error}"),
    }
}
