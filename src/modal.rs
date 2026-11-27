//! Opens native HTML dialogs and reports their close event.

use dioxus::prelude::*;

pub(crate) async fn show_dialog(id: &'static str, on_close: EventHandler<()>) {
    run_dialog(id, "dialog.showModal(); await closed;").await;
    on_close.call(());
}

pub(crate) async fn run_dialog(id: &'static str, script: &str) {
    let mut dialog = document::eval(&format!(
        "try {{
             const dialog = document.getElementById('{id}');
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
