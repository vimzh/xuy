// WebKit can hang resolving editor accessibility when a native modal opens.
const previousFocus = document.activeElement;
const scrim = document.getElementById("command-scrim");
const background = [...dialog.parentElement.children].filter(
    element => element !== dialog && element !== scrim && !element.inert,
);
dialog.addEventListener("keydown", event => {
    if (event.isComposing) return;
    if (event.key === "Escape") {
        event.preventDefault();
        dialog.close();
        return;
    }
    if (event.key !== "Tab") return;
    const controls = dialog.querySelectorAll("input, button");
    const first = controls[0];
    const last = controls[controls.length - 1];
    if (document.activeElement === (event.shiftKey ? first : last)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
    }
});

dialog.show();
try {
    for (const element of background) element.inert = true;
    await closed;
} finally {
    for (const element of background) element.inert = false;
    previousFocus?.focus({ preventScroll: true });
}
