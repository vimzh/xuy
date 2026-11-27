// Render PTY bytes locally and report input and dimensions back to Rust.
const host = document.getElementById("notebook-terminal");
let terminal;
let observer;
let zoomObserver;
let visibilityObserver;
const scheme = matchMedia("(prefers-color-scheme: dark)");
const style = () => getComputedStyle(host);
const theme = () => ({
    background: style().backgroundColor,
    foreground: style().color,
    cursor: style().color,
    selectionBackground: style().getPropertyValue("--selected").trim(),
});
const applyTheme = () => { terminal.options.theme = theme(); };
try {
    const { Terminal, FitAddon } = await import(host.dataset.module);
    if (!host.isConnected) return;
    terminal = new Terminal({ theme: theme(), fontFamily: "Menlo, monospace", fontSize: 13, scrollback: 1500 });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.attachCustomKeyEventHandler(event => !event.metaKey);
    terminal.open(host);
    fit.fit();
    dioxus.send(["ready", "", terminal.cols, terminal.rows]);
    terminal.onData(data => dioxus.send(["input", data, 0, 0]));
    terminal.onBinary(data => dioxus.send(["binary", data, 0, 0]));
    terminal.onResize(({cols, rows}) => dioxus.send(["resize", "", cols, rows]));
    observer = new ResizeObserver(() => { if (host.clientWidth && host.clientHeight) fit.fit(); });
    observer.observe(host);
    zoomObserver = new MutationObserver(() => { if (host.clientWidth && host.clientHeight) fit.fit(); });
    zoomObserver.observe(host, { attributes: true, attributeFilter: ["data-zoom"] });
    const panel = host.closest("section");
    visibilityObserver = new MutationObserver(() => {
        if (!panel.hidden) {
            fit.fit();
            terminal.focus();
        }
    });
    visibilityObserver.observe(panel, { attributes: true, attributeFilter: ["hidden"] });
    scheme.addEventListener("change", applyTheme);
    terminal.focus();
    for (let bytes; (bytes = await dioxus.recv()) !== null;) {
        await new Promise(resolve => terminal.write(Uint8Array.from(bytes), resolve));
        dioxus.send(["painted", "", 0, 0]);
    }
} catch (error) {
    dioxus.send(["error", String(error), 0, 0]);
    await dioxus.recv();
} finally {
    observer?.disconnect();
    zoomObserver?.disconnect();
    visibilityObserver?.disconnect();
    scheme.removeEventListener("change", applyTheme);
    terminal?.dispose();
}
