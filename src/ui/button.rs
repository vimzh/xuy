//! Shared native buttons for actions and form submission.

use dioxus::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ButtonKind {
    #[default]
    Action,
    Submit,
}

#[component]
pub(crate) fn Button(
    #[props(default)] kind: ButtonKind,
    #[props(extends = GlobalAttributes, extends = button)] attributes: Vec<Attribute>,
    #[props(default)] autofocus: bool,
    onclick: Option<EventHandler<MouseEvent>>,
    onmounted: Option<EventHandler<MountedEvent>>,
    children: Element,
) -> Element {
    let button_type = match kind {
        ButtonKind::Action => "button",
        ButtonKind::Submit => "submit",
    };

    let mut attributes = attributes;
    attributes.extend(
        onclick.map(|handler| dioxus_elements::events::onclick(move |event| handler.call(event))),
    );
    attributes.extend(
        onmounted
            .map(|handler| dioxus_elements::events::onmounted(move |event| handler.call(event))),
    );

    rsx! {
        button { r#type: button_type, autofocus, ..attributes, {children} }
    }
}
