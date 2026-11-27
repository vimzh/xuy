// Keep formatted markup around the native text inputs and send only their text.
const content = document.getElementById("markdown-preview-content");
const abort = new AbortController();
const listen = (event, handler) => content.addEventListener(event, handler, { signal: abort.signal });

function send(kind, element, value = "") {
    dioxus.send([kind, Number(content.dataset.previewGeneration), Number(element.dataset.mdText), value]);
}

function update(element) {
    send("input", element, element.innerText);
}

listen("focusin", event => {
    if (event.target.matches("[data-md-text]")) send("focus", event.target);
});

listen("focusout", event => {
    if (event.target.matches("[data-md-text]") && !event.relatedTarget?.matches?.("[data-md-text]")) {
        send("blur", event.target);
    }
});

listen("beforeinput", event => {
    if (!event.target.matches('[data-md-text][aria-multiline="false"]')) return;
    if (event.inputType === "insertParagraph" || event.inputType === "insertLineBreak") {
        event.preventDefault();
    }
});

listen("paste", event => {
    if (event.target.matches('[data-md-text][aria-multiline="false"]') &&
        /[\r\n]/.test(event.clipboardData.getData("text/plain"))) {
        event.preventDefault();
    }
});

listen("input", event => {
    if (event.target.matches("[data-md-text]") && !event.isComposing) update(event.target);
});

listen("compositionend", event => {
    if (event.target.matches("[data-md-text]")) update(event.target);
});

try {
    await dioxus.recv();
} finally {
    abort.abort();
}
