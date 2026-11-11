import { closeReader, saveProgress, type ReaderDocument } from "./reader-client";

export function createProgressWriter(source: ReaderDocument, report: (error: unknown) => void) {
  let page = 1;
  let total = 0;
  let sequence = 0;
  let activeMs = 0;
  let lastTick = performance.now();
  let active = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let queue: Promise<void> = Promise.resolve();
  let closed = false;
  let closing: Promise<void> | undefined;

  function tick() {
    const now = performance.now();
    if (active) activeMs += now - lastTick;
    lastTick = now;
    active = total > 0 && document.visibilityState === "visible" && document.hasFocus();
  }
  function checkpoint(final = false) {
    tick();
    if (!total) return Promise.resolve();
    const input = { sessionId: source.sessionId, sequence: ++sequence, page, total, activeMs: Math.floor(activeMs), close: final };
    const next = queue.then(() => saveProgress(input));
    queue = next.then(() => report(null), report);
    return next;
  }
  function activity() { if (!closed) { tick(); if (document.visibilityState !== "visible") void checkpoint().catch(() => undefined); } }
  const interval = window.setInterval(() => { if (!closed) void checkpoint().catch(() => undefined); }, 10_000);
  document.addEventListener("visibilitychange", activity);
  window.addEventListener("blur", activity);
  window.addEventListener("focus", activity);
  function stop() {
    tick();
    active = false;
    clearTimeout(timer);
    clearInterval(interval);
    document.removeEventListener("visibilitychange", activity);
    window.removeEventListener("blur", activity);
    window.removeEventListener("focus", activity);
  }
  return {
    stop,
    async ready(current: number, pages: number) { page = current; total = pages; await checkpoint(); },
    page(current: number) {
      if (closed || !total) return;
      page = current;
      clearTimeout(timer);
      timer = setTimeout(() => { void checkpoint().catch(() => undefined); }, 700);
    },
    close() {
      if (closing) return closing;
      closed = true;
      clearTimeout(timer);
      closing = checkpoint(true).catch(async (error) => {
        if (typeof error === "object" && error !== null && "code" in error && error.code === "reader_source_unavailable") {
          report(error);
          return;
        }
        throw error;
      }).then(() => closeReader(source.sessionId)).then(stop).catch((error) => {
        report(error);
        closed = false;
        closing = undefined;
        throw error;
      });
      return closing;
    },
  };
}
