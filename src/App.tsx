import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, type AppInfo, type Capture } from "./lib/api";
import { groupByDay } from "./lib/format";
import { CaptureCard } from "./components/CaptureCard";
import "./App.css";

export default function App() {
  const [captures, setCaptures] = useState<Capture[] | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const notify = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 2000);
  }, []);

  const refresh = useCallback(async () => {
    try {
      setCaptures(await api.list());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    api.appInfo().then(setInfo).catch((e) => setError(String(e)));
    refresh();
    // The Rust side tells us about captures it takes or deletes. Captures made by
    // other writers (the MCP server, later) show up when the window regains focus.
    const unlisten = [
      listen("captures-changed", refresh),
      listen<string>("capture-error", (e) => setError(e.payload)),
      getCurrentWindow().onFocusChanged(({ payload: focused }) => focused && refresh()),
    ];
    return () => unlisten.forEach((p) => p.then((f) => f()));
  }, [refresh]);

  const run = (action: () => Promise<unknown>, done?: string) =>
    action().then(() => done && notify(done)).catch((e) => setError(String(e)));

  const onCopy = (c: Capture) => run(() => api.copy(c.id), "Copied to clipboard");
  const onOpen = (c: Capture) => run(() => api.open(c.id));
  const onReveal = (c: Capture) => run(() => api.reveal(c.id));
  const onDelete = (c: Capture) => {
    // Optimistic: drop it now; the change event re-syncs with the folder.
    setCaptures((prev) => prev?.filter((x) => x.id !== c.id) ?? null);
    run(() => api.remove(c.id), "Moved to Trash").then(refresh);
  };
  const onCapture = () => run(() => api.capture());

  const groups = groupByDay(captures ?? []);

  return (
    <div className="app">
      <header className="toolbar">
        <div className="title">
          <h1>Screenshots</h1>
          {captures && <span className="count">{captures.length}</span>}
        </div>
        <div className="actions">
          <button className="ghost" onClick={() => run(api.openFolder)}>
            Open folder
          </button>
          <button className="primary" onClick={onCapture}>
            Capture region
            {info && <kbd>{info.hotkey}</kbd>}
          </button>
        </div>
      </header>

      {info && !info.hotkeyRegistered && (
        <div className="banner">
          {info.hotkey} is taken by another app, so the hotkey is off. Use the menu bar icon or the button above.
        </div>
      )}
      {error && (
        <div className="banner error" role="alert">
          <span>{error}</span>
          <button className="ghost small" onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <main>
        {captures === null ? null : captures.length === 0 ? (
          <div className="empty">
            <p className="empty-title">No screenshots yet</p>
            <p>
              Press <kbd>{info?.hotkey ?? "⌘⇧2"}</kbd> anywhere, or use the menu bar icon, then drag to select an
              area. Press Space in the overlay to pick a window.
            </p>
          </div>
        ) : (
          groups.map((g) => (
            <section key={g.label}>
              <h2>{g.label}</h2>
              <div className="grid">
                {g.items.map((c) => (
                  <CaptureCard
                    key={c.id}
                    capture={c}
                    onCopy={onCopy}
                    onOpen={onOpen}
                    onReveal={onReveal}
                    onDelete={onDelete}
                  />
                ))}
              </div>
            </section>
          ))
        )}
      </main>

      {info && <footer title={info.dir}>{info.dir}</footer>}
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
