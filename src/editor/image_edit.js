// Hold the native input event until Rust retains the image files.
function canInsertImages(editor) {
    return editor?.isConnected && !editor.readOnly && !editor.dataset.composing &&
        !editor.closest("[hidden], [inert]");
}

function applyImageEdit(editor, source, from, to, insertion) {
    if (!canInsertImages(editor) || editor.value !== source) return false;
    const { selectionStart, selectionEnd, selectionDirection } = editor;
    editor.focus({ preventScroll: true });
    editor.setSelectionRange(from, to);
    const suppressInput = event => event.stopImmediatePropagation();
    editor.addEventListener("input", suppressInput, true);
    try {
        document.execCommand("insertText", false, insertion);
        if (editor.value === source.slice(0, from) + insertion + source.slice(to)) return true;
        throw new Error("WebKit rejected the image insertion");
    } catch (error) {
        console.error("Could not insert images", error);
        if (editor.value !== source) {
            try {
                document.execCommand("undo");
            } catch (undoError) {
                console.error("Could not undo the rejected image insertion", undoError);
            }
            if (editor.value !== source) editor.value = source;
        }
        editor.setSelectionRange(selectionStart, selectionEnd, selectionDirection);
        return false;
    } finally {
        editor.removeEventListener("input", suppressInput, true);
    }
}
