// Share textarea geometry between the caret, navigation jumps, and highlight backgrounds.
const editor = document.getElementById("note-source");
const frame = editor.parentElement;
const layer = frame.querySelector(".note-caret-layer");
const caret = layer.querySelector(".note-custom-caret");
const highlightLayer = frame.querySelector(".note-highlight-marks");
const mirror = document.createElement("div");
mirror.className = "note-caret-mirror";
const text = mirror.appendChild(document.createTextNode(""));
layer.appendChild(mirror);

const controller = new AbortController();
const segmenter = new Intl.Segmenter();
const range = document.createRange();
let graphemes;
let value;
let layoutDirty = true;
let composing = false;
let nextFrame = 0;
let restartBlink = false;
let snapNext = true;
let highlightRanges = [];
let encodedRanges;
let highlightsDirty = true;
let pendingJump;

function canDraw() {
    return document.hasFocus() && !document.hidden && document.activeElement === editor &&
        !editor.readOnly && !composing && editor.selectionStart === editor.selectionEnd &&
        (frame.dataset.caretShape !== "bar" || frame.dataset.caretBlink === "false" ||
            frame.dataset.caretSmooth === "true" || frame.dataset.caretBlinkStyle === "fade");
}

function hide() {
    snapNext = true;
    caret.hidden = true;
    caret.removeAttribute("data-caret-moving");
    frame.removeAttribute("data-caret-drawn");
}

function syncMirror() {
    if (layoutDirty) {
        const style = getComputedStyle(editor);
        for (const property of ["font", "letter-spacing", "tab-size", "direction", "text-align",
            "white-space", "word-break", "overflow-wrap", "padding"]) {
            mirror.style.setProperty(property, style.getPropertyValue(property));
        }
        mirror.style.width = editor.clientWidth + "px";
        layoutDirty = false;
        highlightsDirty = true;
    }

    const editorValue = editor.value;
    if (editorValue !== value) {
        value = editorValue;
        // Give empty documents and final blank lines a measurable insertion point.
        text.data = value + "\u200b";
        graphemes = undefined;
        highlightsDirty = true;
    }
}

function drawHighlights(origin) {
    if (highlightsDirty) {
        const marks = document.createDocumentFragment();
        for (const [start, end, color] of highlightRanges) {
            if (end > value.length) continue;
            range.setStart(text, start);
            range.setEnd(text, end);
            for (const rect of range.getClientRects()) {
                if (!rect.width || !rect.height) continue;
                const mark = document.createElement("span");
                mark.className = color;
                mark.style.left = (rect.left - origin.left) + "px";
                mark.style.top = (rect.top - origin.top) + "px";
                mark.style.width = rect.width + "px";
                mark.style.height = rect.height + "px";
                marks.appendChild(mark);
            }
        }
        highlightLayer.replaceChildren(marks);
        highlightsDirty = false;
    }
    highlightLayer.style.transform = `translate(${-editor.scrollLeft}px, ${-editor.scrollTop}px)`;
}

function draw() {
    nextFrame = 0;
    const drawCaret = canDraw();
    if (!drawCaret && !highlightRanges.length) return hide();
    syncMirror();
    const origin = mirror.getBoundingClientRect();
    if (highlightRanges.length) drawHighlights(origin);
    if (!drawCaret) return hide();

    const offset = editor.selectionEnd;
    range.setStart(text, offset);
    range.collapse(true);
    let position = range.getBoundingClientRect();
    if (frame.dataset.caretShape !== "bar" && value[offset] !== "\n") {
        graphemes ??= segmenter.segment(value);
        const glyph = graphemes.containing(offset);
        if (glyph) {
            range.setStart(text, glyph.index);
            range.setEnd(text, glyph.index + glyph.segment.length);
            const bounds = range.getBoundingClientRect();
            if (bounds.width) position = bounds;
        }
    }

    const x = position.left - origin.left - editor.scrollLeft;
    const y = position.top - origin.top - editor.scrollTop;
    if (!position.height || y + position.height <= 0 || y >= editor.clientHeight ||
        x < 0 || x > editor.clientWidth) return hide();

    caret.toggleAttribute("data-caret-moving", !caret.hidden && !snapNext);
    snapNext = false;
    caret.style.transform = `translate(${x}px, ${y}px)`;
    caret.style.setProperty("--note-caret-height", position.height + "px");
    caret.style.setProperty("--note-caret-width", position.width ? position.width + "px" : "1ch");
    caret.hidden = false;
    frame.setAttribute("data-caret-drawn", "");
    if (restartBlink) {
        for (const animation of caret.getAnimations()) {
            if (animation instanceof CSSAnimation) animation.currentTime = 0;
        }
        restartBlink = false;
    }
}

function queue(restart = false, snap = false) {
    if (!canDraw()) {
        hide();
        if (!highlightRanges.length) return;
    }
    restartBlink ||= restart;
    snapNext ||= snap;
    if (!nextFrame) nextFrame = requestAnimationFrame(draw);
}

function resize() {
    layoutDirty = true;
    queue(false, true);
}

const resized = new ResizeObserver(resize);
const changed = new MutationObserver(records => {
    if (records.some(record => record.attributeName === "style" || record.attributeName === "hidden")) {
        layoutDirty = true;
    }
    if (pendingJump && !frame.hidden) {
        const selection = pendingJump;
        pendingJump = undefined;
        jump(...selection);
    }
    readHighlights();
    queue(true, true);
});
const listen = (target, event, handler) => target.addEventListener(event, handler, { signal: controller.signal });

function readHighlights() {
    const encoded = frame.dataset.highlightRanges;
    if (encoded === encodedRanges) return;
    encodedRanges = encoded;
    highlightRanges = JSON.parse(encoded);
    highlightsDirty = true;
    if (!highlightRanges.length) highlightLayer.replaceChildren();
}

function offsetTop(offset) {
    range.setStart(text, offset);
    range.collapse(true);
    return range.getBoundingClientRect().top - mirror.getBoundingClientRect().top;
}

function jump(start, end) {
    if (frame.hidden) {
        // A highlight chosen in Preview can arrive before Edit is visible.
        pendingJump = [start, end];
        return;
    }
    if (editor.readOnly || composing || end > editor.value.length) return;
    syncMirror();
    const y = offsetTop(start);
    editor.focus({ preventScroll: true });
    editor.setSelectionRange(start, end);
    if (y < editor.scrollTop || y + 24 > editor.scrollTop + editor.clientHeight) {
        editor.scrollTop = y - editor.clientHeight / 3;
    }
    editor.dispatchEvent(new Event("selectionchange", { bubbles: true }));
    queue(true, true);
}

// Use the same wrapped layout as the textarea to find the logical line at the drop.
function imageDropTarget(event) {
    syncMirror();
    const bounds = editor.getBoundingClientRect();
    const scale = bounds.height / editor.offsetHeight;
    const y = event.detail.clientY - bounds.top + editor.scrollTop * scale;
    const lineStarts = [0];
    for (const newline of value.matchAll(/\n/g)) lineStarts.push(newline.index + 1);
    function top(offset) {
        range.setStart(text, offset);
        range.collapse(true);
        return range.getBoundingClientRect().top;
    }
    const origin = top(0);
    let first = 0;
    let last = lineStarts.length;
    while (first + 1 < last) {
        const middle = Math.floor((first + last) / 2);
        if (top(lineStarts[middle]) - origin <= y) first = middle;
        else last = middle;
    }
    const newline = value.indexOf("\n", lineStarts[first]);
    event.detail.lineEnd = newline < 0 ? value.length : newline;
}

try {
    resized.observe(editor);
    changed.observe(frame, { attributes: true, attributeFilter: [
        "data-caret-shape", "data-caret-blink", "data-caret-smooth", "data-caret-blink-style", "data-highlight-ranges", "hidden",
    ] });
    changed.observe(editor, { attributes: true, attributeFilter: ["readonly"] });
    changed.observe(editor.closest(".workspace"), { attributes: true, attributeFilter: ["style"] });
    for (const event of ["focusin", "focusout", "input"]) listen(editor, event, () => queue(true));
    listen(editor, "dragover", event => {
        if (event.dataTransfer?.types.includes("Files")) {
            event.preventDefault();
            event.dataTransfer.dropEffect = "copy";
        }
    });
    listen(editor, "xuy-image-drop-target", imageDropTarget);
    listen(editor, "scroll", () => queue(false, true));
    listen(editor, "compositionstart", () => { composing = true; editor.dataset.composing = "true"; hide(); });
    listen(editor, "compositionend", () => { composing = false; delete editor.dataset.composing; queue(true); });
    installCodeBlockKeys(editor, listen, offset => {
        syncMirror();
        return offsetTop(offset);
    }, jump);
    // Older WebKit versions dispatch textarea selection changes on the document.
    listen(document, "selectionchange", () => queue(true));
    listen(document, "visibilitychange", () => queue(true));
    listen(document.fonts, "loadingdone", resize);
    listen(window, "resize", resize);
    listen(window, "blur", hide);
    listen(window, "focus", () => queue(true));
    readHighlights();
    queue(true);
    for (let selection; Array.isArray(selection = await dioxus.recv());) jump(...selection);
} finally {
    controller.abort();
    resized.disconnect();
    changed.disconnect();
    cancelAnimationFrame(nextFrame);
    mirror.remove();
    hide();
}
