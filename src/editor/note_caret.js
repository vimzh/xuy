// Report the active end of the Markdown textarea selection, in UTF-16 units.
const listeners = new AbortController();
let previousEditor;
let previousOffset;

function reportCaret() {
    const editor = document.activeElement;
    if (!(editor instanceof HTMLTextAreaElement) || editor.id !== "note-source") return;
    const offset = editor.selectionDirection === "backward" ? editor.selectionStart : editor.selectionEnd;
    if (editor === previousEditor && offset === previousOffset) return;
    previousEditor = editor;
    previousOffset = offset;
    dioxus.send(offset);
}

try {
    // Listen on the document for WebKit versions that fire selectionchange there.
    for (const event of ["selectionchange", "focusin", "input"]) {
        document.addEventListener(event, reportCaret, { signal: listeners.signal });
    }
    reportCaret();
    await dioxus.recv();
} finally {
    listeners.abort();
}
