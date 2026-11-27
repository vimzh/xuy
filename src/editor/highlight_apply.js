// Apply one Rust-checked Markdown replacement without clearing native Undo history.
const editor = document.getElementById("note-source");
const ready = editor && !editor.readOnly && !editor.dataset.composing;
const source = ready ? editor.value : "";
const start = ready ? editor.selectionStart : 0;
const end = ready ? editor.selectionEnd : 0;
dioxus.send(ready ? [source, start, end] : null);

const edit = await dioxus.recv();
if (!edit) return;
let applied = false;
if (editor?.isConnected && !editor.readOnly && !editor.dataset.composing && editor.value === source &&
    editor.selectionStart === start && editor.selectionEnd === end) {
    const [from, to, replacement] = edit;
    editor.focus({ preventScroll: true });
    editor.setSelectionRange(from, to);
    // Commit to the draft only after checking WebKit's native text substitution.
    const suppressInput = event => event.stopImmediatePropagation();
    editor.addEventListener("input", suppressInput, true);
    try {
        document.execCommand("insertText", false, replacement);
        applied = editor.value === source.slice(0, from) + replacement + source.slice(to);
    } catch (error) {
        console.error("Could not replace the highlight selection", error);
    } finally {
        if (!applied && editor.value !== source) {
            try {
                document.execCommand("undo");
            } catch (error) {
                console.error("Could not undo the rejected highlight edit", error);
            }
            if (editor.value !== source) editor.value = source;
        }
        editor.removeEventListener("input", suppressInput, true);
    }
    if (applied) {
        editor.dispatchEvent(new Event("input", { bubbles: true }));
    } else {
        editor.setSelectionRange(start, end);
    }
}
dioxus.send(applied);
await dioxus.recv();
