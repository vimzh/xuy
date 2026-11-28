// Send preview edits to the Markdown draft and apply confirmed checklist changes.
const content = document.getElementById("markdown-preview-content");
const abort = new AbortController();
const listen = (event, handler) => content.addEventListener(event, handler, { signal: abort.signal });

function send(kind, element, value = "") {
    const index = Number(element.dataset.mdText ?? element.dataset.mdTask);
    dioxus.send([kind, Number(content.dataset.previewGeneration), index, value]);
}

function update(element) {
    send("input", element, element.innerText);
}

const editsNote = element => element?.matches?.("[data-md-text], input[data-md-task]");

listen("focusin", event => {
    if (editsNote(event.target)) send("focus", event.target);
});

listen("focusout", event => {
    if (editsNote(event.target) && !editsNote(event.relatedTarget)) {
        send("blur", event.target);
    }
});

listen("click", event => {
    const task = event.target.closest("input[data-md-task]");
    if (!task) return;
    event.preventDefault();
    if (content.closest("[inert]")) return;
    task.focus({ preventScroll: true });
    send("task", task);
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
    for (let reply; Array.isArray(reply = await dioxus.recv());) {
        const [generation, index, checked] = reply;
        if (generation !== Number(content.dataset.previewGeneration)) continue;
        const task = content.querySelector(`input[data-md-task="${index}"]`);
        if (task) task.checked = checked;
    }
} finally {
    abort.abort();
}
