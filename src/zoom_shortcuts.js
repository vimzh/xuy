// Listen above the app content so the shortcuts also work when Home has no focused button.
const listener = new AbortController();

window.addEventListener("keydown", (event) => {
    if (!event.metaKey || event.ctrlKey || event.altKey || event.isComposing || event.defaultPrevented) return;

    const zoomIn = event.key === "+" || event.key === "=";
    const zoomOut = event.key === "-" || event.key === "−";
    if (!zoomIn && !zoomOut) return;

    event.preventDefault();
    dioxus.send(zoomIn);
}, { signal: listener.signal });

try {
    await dioxus.recv();
} finally {
    listener.abort();
}
