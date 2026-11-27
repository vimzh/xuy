// Keep pointer movement in WebKit so resizing never rerenders the note.
const divider = document.getElementById("workspace-sidebar-divider");
const sidebar = document.getElementById("workspace-sidebar");
const pane = sidebar.firstElementChild;
const workspace = sidebar.closest(".workspace");
const listeners = new AbortController();
const options = { signal: listeners.signal };
const widthKey = "xuy.sidebar-width";
let drag;
let minimum;
let maximum;
let preferredWidth;
let savedWidth;

function applyWidth(width) {
    if (preferredWidth === width) return;
    preferredWidth = width;
    // Keep the user preference separate from the workspace's reactive inline styles.
    document.documentElement.style.setProperty(`--sidebar-preferred-width`, `${width}px`);
}

try {
    const saved = localStorage.getItem(widthKey);
    if (saved !== null) {
        const width = Number(saved);
        if (!Number.isSafeInteger(width) || width <= 0) throw new Error("Invalid saved width");
        applyWidth(width);
        savedWidth = width;
    }
} catch (error) {
    console.error("Could not restore the sidebar width:", error);
}

function saveWidth() {
    if (preferredWidth === savedWidth) return;
    try {
        localStorage.setItem(widthKey, String(preferredWidth));
        savedWidth = preferredWidth;
    } catch (error) {
        console.error("Could not save the sidebar width:", error);
    }
}

function resizeTo(width) {
    const next = Math.round(Math.min(maximum, Math.max(minimum, width)));
    if (Number.isFinite(next)) {
        applyWidth(next);
    }
}

function syncRange() {
    const { minWidth, maxWidth } = getComputedStyle(pane);
    minimum = parseFloat(minWidth);
    maximum = parseFloat(maxWidth);
    if (!Number.isFinite(maximum)) return;
    const width = pane.offsetWidth;
    divider.setAttribute("aria-valuemin", minimum);
    divider.setAttribute("aria-valuemax", Math.round(maximum));
    divider.setAttribute("aria-valuenow", width);
    divider.setAttribute("aria-valuetext", `${width} pixels`);
}

divider.addEventListener("pointerdown", (event) => {
    if (!event.isPrimary || event.button !== 0 || drag) return;
    event.preventDefault();
    drag = { pointerId: event.pointerId, offset: event.clientX - sidebar.offsetWidth };
    divider.setPointerCapture(event.pointerId);
    workspace.setAttribute("data-sidebar-resizing", "");
}, options);

divider.addEventListener("pointermove", (event) => {
    if (event.pointerId !== drag?.pointerId) return;
    if (workspace.dataset.sidebarOpen === "false") {
        divider.releasePointerCapture(event.pointerId);
        return;
    }
    resizeTo(event.clientX - drag.offset);
}, options);

divider.addEventListener("lostpointercapture", (event) => {
    if (event.pointerId !== drag?.pointerId) return;
    drag = undefined;
    workspace.removeAttribute("data-sidebar-resizing");
    saveWidth();
}, options);

divider.addEventListener("keydown", (event) => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    let width;
    switch (event.key) {
        case "ArrowLeft": width = pane.offsetWidth - 16; break;
        case "ArrowRight": width = pane.offsetWidth + 16; break;
        case "Home": width = minimum; break;
        case "End": width = maximum; break;
        default: return;
    }
    event.preventDefault();
    resizeTo(width);
    saveWidth();
}, options);

const observer = new ResizeObserver(syncRange);
observer.observe(pane);
observer.observe(workspace);
try {
    await dioxus.recv();
} finally {
    observer.disconnect();
    listeners.abort();
}
