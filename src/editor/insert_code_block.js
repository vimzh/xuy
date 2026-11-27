// Insert standard Markdown fences as a single native undo step.
const editor = document.getElementById("note-source");
await new Promise(requestAnimationFrame);

let failure = null;
if (!editor?.isConnected || editor.readOnly || editor.dataset.composing) {
    failure = "Place the cursor in the Markdown editor before inserting a code block.";
} else {
    const { value, selectionStart: start, selectionEnd: end } = editor;
    const before = value.slice(0, start);
    const selected = value.slice(start, end);
    const after = value.slice(end);
    const prefix = !before || before.endsWith("\n\n") ? "" : before.endsWith("\n") ? "\n" : "\n\n";
    const suffix = !after || after.startsWith("\n\n") ? "" : after.startsWith("\n") ? "\n" : "\n\n";
    let fenceLength = 3;
    for (const match of selected.matchAll(/`{3,}/g)) {
        fenceLength = Math.max(fenceLength, match[0].length + 1);
    }
    const fence = "`".repeat(fenceLength);
    const opening = prefix + fence + "\n";
    const replacement = opening + selected + (selected.endsWith("\n") ? "" : "\n") + fence + suffix;
    const expected = before + replacement + after;

    editor.focus({ preventScroll: true });
    editor.setSelectionRange(start, end);
    const suppressInput = event => event.stopImmediatePropagation();
    editor.addEventListener("input", suppressInput, true);
    try {
        document.execCommand("insertText", false, replacement);
        if (editor.value !== expected) throw new Error("WebKit rejected the Markdown insertion");
    } catch (error) {
        failure = "Could not insert the code block. " + error.message;
        if (editor.value !== value) document.execCommand("undo");
        editor.setSelectionRange(start, end);
    } finally {
        editor.removeEventListener("input", suppressInput, true);
    }

    if (!failure) {
        const bodyStart = start + opening.length;
        editor.setSelectionRange(bodyStart, bodyStart + selected.length);
        editor.dispatchEvent(new Event("input", { bubbles: true }));
    }
}
dioxus.send(failure);
await dioxus.recv();
