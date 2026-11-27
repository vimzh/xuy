// Run the shared fenced-block insertion after the command palette closes.
const editor = document.getElementById("note-source");
await new Promise(requestAnimationFrame);

let failure = null;
if (!editor?.isConnected || editor.readOnly || editor.dataset.composing ||
    editor.closest("[hidden], [inert]")) {
    failure = "Place the cursor in the Markdown editor before inserting a code block.";
} else {
    try {
        insertCodeBlock(editor, editor.selectionStart, editor.selectionEnd);
    } catch (error) {
        failure = "Could not insert the code block. " + error.message;
    }
}
dioxus.send(failure);
await dioxus.recv();
