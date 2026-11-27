// Share fenced Markdown insertion between the palette and source-editor shortcuts.
function replaceCodeText(editor, start, end, replacement, caretStart, caretEnd = caretStart) {
    const { value, selectionStart, selectionEnd, selectionDirection } = editor;
    const expected = value.slice(0, start) + replacement + value.slice(end);
    editor.focus({ preventScroll: true });
    editor.setSelectionRange(start, end);

    // Use WebKit's native undo stack and publish the edit once the caret is in place.
    const suppressInput = event => event.stopImmediatePropagation();
    editor.addEventListener("input", suppressInput, true);
    try {
        document.execCommand("insertText", false, replacement);
        if (editor.value !== expected) throw new Error("WebKit rejected the Markdown insertion");
    } catch (error) {
        if (editor.value !== value) document.execCommand("undo");
        editor.setSelectionRange(selectionStart, selectionEnd, selectionDirection);
        throw error;
    } finally {
        editor.removeEventListener("input", suppressInput, true);
    }
    editor.setSelectionRange(caretStart, caretEnd);
    editor.dispatchEvent(new Event("input", { bubbles: true }));
}

function insertCodeBlock(editor, start, end, selected = editor.value.slice(start, end)) {
    const before = editor.value.slice(0, start);
    const after = editor.value.slice(end);
    const prefix = !before || before.endsWith("\n\n") ? "" : before.endsWith("\n") ? "\n" : "\n\n";
    const suffix = !after || after.startsWith("\n\n") ? "" : after.startsWith("\n") ? "\n" : "\n\n";
    let fenceLength = 3;
    for (const match of selected.matchAll(/`{3,}/g)) {
        fenceLength = Math.max(fenceLength, match[0].length + 1);
    }
    const fence = "`".repeat(fenceLength);
    const opening = prefix + fence + "\n";
    const replacement = opening + selected + (selected.endsWith("\n") ? "" : "\n") + fence + suffix;
    const bodyStart = start + opening.length;
    replaceCodeText(editor, start, end, replacement, bodyStart, bodyStart + selected.length);
}

const fenceLine = /^ {0,3}(`{3,}|~{3,})([^\n]*)$/;

function closesFence(opening, marker, rest) {
    return marker[0] === opening[0] && marker.length >= opening.length && /^[ \t]*$/.test(rest);
}

function openFenceBefore(value, offset) {
    let open;
    const fences = value.slice(0, offset).matchAll(new RegExp(fenceLine, "gm"));
    for (const match of fences) {
        const [, marker, rest] = match;
        if (open) {
            if (closesFence(open.marker, marker, rest)) open = undefined;
        } else if (marker[0] !== "`" || !rest.includes("`")) {
            open = { marker, index: match.index };
        }
    }
    return open;
}

function lineStart(value, offset) {
    return offset === 0 ? 0 : value.lastIndexOf("\n", offset - 1) + 1;
}

function lineEnd(value, offset) {
    const end = value.indexOf("\n", offset);
    return end === -1 ? value.length : end;
}

function codeExit(value, offset, start, end, up, topAt) {
    if (up ? start === 0 : end === value.length) return;
    const fenceStart = up ? lineStart(value, start - 1) : end + 1;
    const fenceEnd = up ? start - 1 : lineEnd(value, fenceStart);
    const candidate = value.slice(fenceStart, fenceEnd).match(fenceLine);
    if (!candidate) return;
    const open = openFenceBefore(value, start);
    if (!open || (up ? open.index !== fenceStart : !closesFence(open.marker, candidate[1], candidate[2]))) return;

    // Let the textarea handle vertical motion within a wrapped code line.
    if (topAt(offset) !== topAt(up ? start : end)) return;
    if (up) return { target: Math.max(0, fenceStart - 1), addLine: fenceStart === 0 ? 0 : null };
    if (fenceEnd < value.length) return { target: fenceEnd + 1, addLine: null };
    return { target: fenceEnd + 1, addLine: fenceEnd };
}

function installCodeBlockKeys(editor, listen, topAt, jump) {
    const error = editor.parentElement.querySelector(".note-code-block-error");
    listen(editor, "input", () => { if (!error.hidden) error.hidden = true; });
    listen(editor, "keydown", event => {
        const confirming = event.key === "Enter" || event.key === "Tab";
        const up = event.key === "ArrowUp";
        if (!confirming && !up && event.key !== "ArrowDown") return;
        if (event.defaultPrevented || event.isComposing || editor.dataset.composing ||
            event.shiftKey || event.metaKey || event.ctrlKey || event.altKey ||
            editor.readOnly || editor.closest("[hidden], [inert]")) return;

        const { value, selectionStart: offset, selectionEnd } = editor;
        if (offset !== selectionEnd) return;
        const start = lineStart(value, offset);
        const end = lineEnd(value, offset);
        try {
            if (confirming) {
                if (offset !== end || value.slice(start, end) !== "/code" || openFenceBefore(value, start)) return;
                event.preventDefault();
                insertCodeBlock(editor, start, end, "");
                jump(editor.selectionStart, editor.selectionEnd);
            } else {
                const exit = codeExit(value, offset, start, end, up, topAt);
                if (!exit) return;
                event.preventDefault();
                if (exit.addLine !== null) replaceCodeText(editor, exit.addLine, exit.addLine, "\n", exit.target);
                jump(exit.target, exit.target);
            }
        } catch (failure) {
            error.textContent = "Could not edit the code block. " + failure.message;
            error.hidden = false;
        }
    });
}
