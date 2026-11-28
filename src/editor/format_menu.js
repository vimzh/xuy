// Keep the source selection while the popover has focus.
const editor = document.getElementById("note-source");
const menu = document.getElementById("editor-format-menu");
const error = document.getElementById("editor-format-error");
const listeners = new AbortController();
const options = { signal: listeners.signal };
const available = () => editor.isConnected && !editor.readOnly &&
    !editor.dataset.composing && !editor.closest("[hidden], [inert]");
let selection;

function dismiss() {
    if (menu.matches(":popover-open")) menu.hidePopover();
}

function position(point, size, limit) {
    const gap = 3;
    const margin = 6;
    const offset = point + gap + size + margin <= limit ? point + gap : point - gap - size;
    return Math.max(margin, Math.min(offset, limit - size - margin));
}

function open(x, y) {
    const { value, selectionStart: start, selectionEnd: end, selectionDirection: direction } = editor;
    if (!available() || !value.slice(start, end).trim()) return false;
    selection = { value, start, end, direction };
    error.hidden = true;
    if (!menu.matches(":popover-open")) menu.showPopover();
    const { width, height } = menu.getBoundingClientRect();
    menu.style.left = position(x, width, document.documentElement.clientWidth) + "px";
    menu.style.top = position(y, height, document.documentElement.clientHeight) + "px";
    menu.querySelector("button").focus({ preventScroll: true });
    return true;
}

editor.addEventListener("contextmenu", event => {
    if (open(event.clientX, event.clientY)) {
        event.preventDefault();
    } else if (available()) {
        dismiss();
        // Keep WebKit's edit menu available when Dioxus suppresses document-level context menus.
        event.stopPropagation();
    }
}, options);
editor.addEventListener("keydown", event => {
    if (event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
    const { left, top } = editor.getBoundingClientRect();
    if (open(Math.max(0, left), Math.max(0, top))) event.preventDefault();
}, options);

menu.addEventListener("keydown", event => {
    if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        dismiss();
        editor.focus({ preventScroll: true });
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const buttons = Array.from(menu.querySelectorAll("button:enabled"));
        const index = buttons.indexOf(document.activeElement);
        const step = event.key === "ArrowDown" ? 1 : -1;
        buttons[(index + step + buttons.length) % buttons.length].focus();
    }
}, options);

function showError(message) {
    error.textContent = message;
    error.hidden = false;
}

function format(marker, { value, start, end, direction }) {
    const selected = value.slice(start, end);
    const body = selected.trim();
    if (/\n[^\S\n]*\n/.test(body)) {
        showError("Select text within one paragraph to apply this format.");
        return;
    }

    // Delimiters must touch the text to work when the selection includes whitespace.
    const from = start + selected.indexOf(body);
    const to = from + body.length;
    const replacement = marker + body + marker;
    const expected = value.slice(0, from) + replacement + value.slice(to);
    editor.setSelectionRange(from, to);
    const suppressInput = event => event.stopImmediatePropagation();
    editor.addEventListener("input", suppressInput, true);
    try {
        document.execCommand("insertText", false, replacement);
        if (editor.value !== expected) throw new Error("WebKit rejected the Markdown insertion");
    } catch (failure) {
        console.error(failure);
        if (editor.value !== value) document.execCommand("undo");
        editor.setSelectionRange(start, end, direction);
        showError("Could not format the selection. Select the text and try again.");
        return;
    } finally {
        editor.removeEventListener("input", suppressInput, true);
    }
    editor.setSelectionRange(from + marker.length, to + marker.length, direction);
    editor.dispatchEvent(new Event("input", { bubbles: true }));
}

menu.addEventListener("click", event => {
    const button = event.target.closest("button");
    if (!button || button.disabled) return;
    dismiss();
    if (!available() || editor.value !== selection.value) {
        event.stopPropagation();
        showError("The note changed. Select the text and try again.");
        return;
    }

    editor.focus({ preventScroll: true });
    editor.setSelectionRange(selection.start, selection.end, selection.direction);
    if (button.dataset.format) format(button.dataset.format, selection);
}, options);

function dismissFromOutside(event) {
    if (!menu.contains(event.target)) dismiss();
}

document.addEventListener("focusin", dismissFromOutside, options);
document.addEventListener("pointerdown", dismissFromOutside, { ...options, capture: true });
window.addEventListener("resize", dismiss, options);

const availability = new MutationObserver(() => {
    if (!available()) dismiss();
});
availability.observe(editor, { attributeFilter: ["readonly"] });
for (let parent = editor.parentElement; parent; parent = parent.parentElement) {
    availability.observe(parent, { attributeFilter: ["hidden", "inert"] });
}

try {
    await dioxus.recv();
} finally {
    availability.disconnect();
    listeners.abort();
}
