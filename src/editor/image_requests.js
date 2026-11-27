// Capture the original note and selection before reading the clipboard or opening the picker.
const editor = document.getElementById("note-source");
const controller = new AbortController();
const options = { signal: controller.signal };
let target;

function capture(from, to) {
    target = { source: editor.value, from, to };
}

function encode(file) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve([file.name, reader.result.split(",")[1]]);
        reader.onerror = () => reject(reader.error);
        reader.readAsDataURL(file);
    });
}

editor.addEventListener("xuy-choose-images", event => {
    event.preventDefault();
    if (target || !canInsertImages(editor)) return;
    capture(event.detail.start, event.detail.end);
    dioxus.send({ Ok: null });
}, options);

editor.addEventListener("paste", async event => {
    if (event.defaultPrevented || !canInsertImages(editor)) return;
    const files = Array.from(event.clipboardData?.files ?? []).filter(file => file.type.startsWith("image/"));
    if (!files.length) return;
    event.preventDefault();
    if (target) return;
    capture(editor.selectionStart, editor.selectionEnd);
    if (files.reduce((size, file) => size + file.size, 0) > maxImageBytes) {
        dioxus.send({ Err: "Clipboard images must total 12 MiB or less." });
        return;
    }
    try {
        const encoded = await Promise.all(files.map(encode));
        if (!controller.signal.aborted) dioxus.send({ Ok: encoded });
    } catch (error) {
        if (!controller.signal.aborted) dioxus.send({ Err: "Could not read the clipboard images. " + error.message });
    }
}, options);

try {
    for (;;) {
        const links = await dioxus.recv();
        if (links === false) break;
        if (links && target) {
            const { source, from, to } = target;
            const before = source.slice(0, from);
            const after = source.slice(to);
            const prefix = !before || before.endsWith("\n\n") ? "" : before.endsWith("\n") ? "\n" : "\n\n";
            const suffix = after.startsWith("\n\n") ? "" : !after || after.startsWith("\n") ? "\n" : "\n\n";
            const applied = applyImageEdit(editor, source, from, to, prefix + links + suffix);
            dioxus.send(applied);
            if (applied) {
                if (await dioxus.recv() === false) break;
                editor.dispatchEvent(new Event("input", { bubbles: true }));
            }
        }
        target = undefined;
    }
} finally {
    controller.abort();
}
