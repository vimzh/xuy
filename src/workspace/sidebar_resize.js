// Keep pointer movement in WebKit so resizing never rerenders the note.
const right = await dioxus.recv();
const divider = document.getElementById(right ? "writing-sidebar-divider" : "workspace-sidebar-divider");
const sidebar = document.getElementById(divider.getAttribute("aria-controls"));
const pane = right ? sidebar : sidebar.firstElementChild;
const workspace = sidebar.closest(".workspace");
const listeners = new AbortController();
const options = { signal: listeners.signal };
const preference = right ? "writing-sidebar" : "sidebar";
const widthKey = `xuy.${preference}-width`;
const direction = right ? -1 : 1;
let drag;
let minimum;
let maximum;
let preferredWidth;
let savedWidth;

function applyWidth(width) {
    if (preferredWidth === width) return;
    preferredWidth = width;
    // Keep the user preference separate from the workspace's reactive inline styles.
    document.documentElement.style.setProperty(`--${preference}-preferred-width`, `${width}px`);
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
    if (!Number.isFinite(maximum)) return false;
    const width = pane.offsetWidth;
    divider.setAttribute("aria-valuemin", minimum);
    divider.setAttribute("aria-valuemax", Math.round(maximum));
    divider.setAttribute("aria-valuenow", width);
    divider.setAttribute("aria-valuetext", `${width} pixels`);
    return true;
}

function setResizing(active) {
    if (!right) workspace.toggleAttribute("data-sidebar-resizing", active);
}

divider.addEventListener("pointerdown", (event) => {
    if (!event.isPrimary || event.button !== 0 || drag || !syncRange()) return;
    event.preventDefault();
    drag = { pointerId: event.pointerId, x: event.clientX, width: sidebar.offsetWidth };
    divider.setPointerCapture(event.pointerId);
    setResizing(true);
}, options);

divider.addEventListener("pointermove", (event) => {
    if (event.pointerId !== drag?.pointerId) return;
    if (workspace.getAttribute(`data-${preference}-open`) === "false") {
        divider.releasePointerCapture(event.pointerId);
        return;
    }
    resizeTo(drag.width + (event.clientX - drag.x) * direction);
}, options);

divider.addEventListener("lostpointercapture", (event) => {
    if (event.pointerId !== drag?.pointerId) return;
    drag = undefined;
    setResizing(false);
    saveWidth();
}, options);

divider.addEventListener("keydown", (event) => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || !syncRange()) return;
    let width;
    switch (event.key) {
        case "ArrowLeft": width = pane.offsetWidth - 16 * direction; break;
        case "ArrowRight": width = pane.offsetWidth + 16 * direction; break;
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
if (right) observer.observe(sidebar.parentElement);
try {
    await dioxus.recv();
} finally {
    observer.disconnect();
    listeners.abort();
    if (drag) setResizing(false);
}
