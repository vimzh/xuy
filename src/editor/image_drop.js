// Insert a drop below its original line and report success before triggering autosave.
const editor = document.getElementById("note-source");
const clientY = await dioxus.recv();
const target = { clientY };
if (canInsertImages(editor)) editor.dispatchEvent(new CustomEvent("xuy-image-drop-target", { detail: target }));
const { lineEnd } = target;
const source = editor?.value;
const ready = Number.isInteger(lineEnd);
dioxus.send(ready);
if (!ready) return;

const links = await dioxus.recv();
if (!links) return;
const beforeBreak = lineEnd < source.length;
const position = lineEnd + Number(beforeBreak);
const lineIsBlank = lineEnd === 0 || source[lineEnd - 1] === "\n";
// Blank lines keep consecutive images and adjacent text in separate Markdown blocks.
const before = lineEnd === 0 || (lineIsBlank && beforeBreak) ? "" :
    beforeBreak || lineIsBlank ? "\n" : "\n\n";
const after = source[position] && source[position] !== "\n" ? "\n\n" : "\n";
const insertion = before + links + after;
const applied = applyImageEdit(editor, source, position, position, insertion);
dioxus.send(applied);
if (applied && await dioxus.recv()) editor.dispatchEvent(new Event("input", { bubbles: true }));
