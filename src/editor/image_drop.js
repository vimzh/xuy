// Insert a drop below its original line and report success before triggering autosave.
const editor = document.getElementById("note-source");
const clientY = await dioxus.recv();
const canEdit = () => editor?.isConnected && !editor.readOnly && !editor.dataset.composing &&
    editor.getClientRects().length !== 0;
const target = { clientY };
if (canEdit()) editor.dispatchEvent(new CustomEvent("xuy-image-drop-target", { detail: target }));
const { lineEnd } = target;
const source = editor?.value;
const ready = Number.isInteger(lineEnd);
dioxus.send(ready);
if (!ready) return;

const links = await dioxus.recv();
if (!links) return;
if (!canEdit() || editor.value !== source) {
    dioxus.send(false);
    return;
}

const beforeBreak = lineEnd < source.length;
const position = lineEnd + Number(beforeBreak);
const lineIsBlank = lineEnd === 0 || source[lineEnd - 1] === "\n";
// Blank lines keep consecutive images and adjacent text in separate Markdown blocks.
const before = lineEnd === 0 || (lineIsBlank && beforeBreak) ? "" :
    beforeBreak || lineIsBlank ? "\n" : "\n\n";
const after = source[position] && source[position] !== "\n" ? "\n\n" : "\n";
const insertion = before + links + after;
const start = editor.selectionStart;
const end = editor.selectionEnd;
editor.focus({ preventScroll: true });
editor.setSelectionRange(position, position);
const suppressInput = event => event.stopImmediatePropagation();
editor.addEventListener("input", suppressInput, true);
let applied = false;
try {
    document.execCommand("insertText", false, insertion);
    applied = editor.value === source.slice(0, position) + insertion + source.slice(position);
} catch (error) {
    console.error("Could not insert the dropped images", error);
} finally {
    if (!applied) {
        if (editor.value !== source) {
            try {
                document.execCommand("undo");
            } catch (error) {
                console.error("Could not undo the rejected image insertion", error);
            }
            if (editor.value !== source) editor.value = source;
        }
        editor.setSelectionRange(start, end);
    }
    editor.removeEventListener("input", suppressInput, true);
}
dioxus.send(applied);
if (applied && await dioxus.recv()) editor.dispatchEvent(new Event("input", { bubbles: true }));
