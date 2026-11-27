// Stay in the editor for notebook links and scroll only inside the preview.
const content = document.getElementById("markdown-preview-content");
const abort = new AbortController();

function jump(fragment) {
    if (!fragment) return;
    const target = content.querySelector(`#${CSS.escape(fragment)}`);
    if (!target) return;
    target.scrollIntoView({ block: "start" });
    if (target.tabIndex < 0) target.tabIndex = -1;
    target.focus({ preventScroll: true });
}

function decode(fragment) {
    try {
        return decodeURIComponent(fragment);
    } catch {
        return fragment;
    }
}

content.addEventListener("click", event => {
    const link = event.target.closest("a[href]");
    const href = link?.getAttribute("href");
    if (!href?.startsWith("#")) return;
    event.preventDefault();

    const note = "#xuy-note:";
    const wiki = "#xuy-wiki:";
    if (href.startsWith(note)) {
        dioxus.send([decode(href.slice(note.length)), false]);
    } else if (href.startsWith(wiki)) {
        dioxus.send([decode(href.slice(wiki.length)), true]);
    } else {
        jump(decode(href.slice(1)));
    }
}, { signal: abort.signal });

try {
    for (let fragment; typeof (fragment = await dioxus.recv()) === "string";) {
        jump(fragment);
    }
} finally {
    abort.abort();
}
