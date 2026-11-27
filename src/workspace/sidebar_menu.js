// Manage the sidebar menu without WebKit dismissing it on the opening right-button release.
const sidebar = document.getElementById("notebook-files");
const menu = document.getElementById("sidebar-context-menu");
const workspace = sidebar.closest(".workspace");
const listeners = new AbortController();
const options = { signal: listeners.signal };
const available = () => sidebar.getAttribute("aria-busy") !== "true" && !sidebar.closest("[inert]");

function dismiss() {
    if (menu.matches(":popover-open")) menu.hidePopover();
}

function position(point, size, limit) {
    const gap = 2;
    const margin = 6;
    const offset = point + gap + size + margin <= limit ? point + gap : point - gap - size;
    return Math.max(margin, Math.min(offset, limit - size - margin));
}

function open(x, y) {
    if (!available()) return;
    if (!menu.matches(":popover-open")) menu.showPopover();
    const { width, height } = menu.getBoundingClientRect();
    menu.style.left = position(x, width, document.documentElement.clientWidth) + "px";
    menu.style.top = position(y, height, document.documentElement.clientHeight) + "px";
}

function openFromKeyboard() {
    const { left, bottom } = document.activeElement.getBoundingClientRect();
    open(left, bottom);
}

sidebar.addEventListener("contextmenu", event => {
    event.preventDefault();
    if (menu.contains(event.target)) return;
    if (event.clientX === 0 && event.clientY === 0) {
        openFromKeyboard();
    } else {
        open(event.clientX, event.clientY);
    }
}, options);

sidebar.addEventListener("keydown", event => {
    if (event.isComposing) return;
    if (menu.contains(event.target)) {
        if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            dismiss();
        }
        return;
    }
    if (event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
    event.preventDefault();
    openFromKeyboard();
}, options);

function dismissFromOutside(event) {
    if (!menu.contains(event.target)) dismiss();
}

document.addEventListener("focusin", dismissFromOutside, options);
document.addEventListener("pointerdown", dismissFromOutside, { ...options, capture: true });
window.addEventListener("resize", dismiss, options);

const inertChanged = new MutationObserver(() => {
    if (!available()) dismiss();
});
inertChanged.observe(sidebar, { attributeFilter: ["aria-busy"] });
inertChanged.observe(sidebar.parentElement, { attributeFilter: ["inert"] });
inertChanged.observe(workspace, { attributeFilter: ["inert"] });

try {
    await dioxus.recv();
} finally {
    inertChanged.disconnect();
    listeners.abort();
}
