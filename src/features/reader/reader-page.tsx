import { useEffect, useRef, useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { BookOpen, ChevronFirst, ChevronLast, ChevronLeft, ChevronRight, Maximize, PanelLeft, PanelRight, Search, ZoomIn, ZoomOut } from "lucide-react";
import "pdfjs-dist/web/pdf_viewer.css";
import "./reader.css";
import { usePreferences } from "@/features/settings/preferences-store";
import { useResolvedTheme } from "@/shared/hooks/use-resolved-theme";
import { desktop, errorMessage } from "@/shared/lib/preferences-client";
import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { fieldClass } from "@/features/library/query-feedback";
import { ReaderSidebar } from "./reader-sidebar";
import { ReaderAnnotations } from "./reader-annotations";
import { useReader } from "./use-reader";
import { flushReaderBeforeClose } from "./reader-client";

function ReaderWorkspace({ book, file, targetPage, revision }: { book: number; file: number | null; targetPage: number | null; revision: number | null }) {
  const navigate = useNavigate();
  const root = useRef<HTMLElement>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const reader = useReader(book, file, targetPage, revision);
  const themePreference = usePreferences((state) => state.preferences.reader);
  const changeTheme = usePreferences((state) => state.changeTheme);
  const theme = useResolvedTheme(themePreference);
  const [left, setLeft] = useState(false);
  const [right, setRight] = useState(false);
  const [settings, setSettings] = useState(false);
  const [find, setFind] = useState("");
  const [fullscreen, setFullscreen] = useState(false);
  const [screenBusy, setScreenBusy] = useState(false);
  const [controlError, setControlError] = useState<string | null>(null);
  const [leaving, setLeaving] = useState(false);
  async function leaveReader() {
    if (leaving) return;
    setLeaving(true);
    try { await flushReaderBeforeClose(); navigate(`/library/${book}`); }
    catch (reason) { setControlError(errorMessage(reason)); setLeaving(false); }
  }

  useEffect(() => {
    const source = reader.source;
    if (!source) return;
    let disposed = false;
    let prior = false;
    const apply = (async () => {
      if (desktop) {
        prior = await getCurrentWindow().isFullscreen();
        if (!disposed && source.preferences.fullscreen) {
          await getCurrentWindow().setFullscreen(true);
          if (!disposed) setFullscreen(true);
        }
      }
    })().catch((reason) => { if (!disposed) setControlError(errorMessage(reason)); });
    return () => {
      disposed = true;
      void apply.then(() => desktop ? getCurrentWindow().setFullscreen(prior) : undefined).catch(() => undefined);
      if (!desktop && document.fullscreenElement === root.current) void document.exitFullscreen().catch(() => undefined);
    };
  }, [reader.source?.sessionId]);

  async function toggleFullscreen() {
    if (screenBusy) return;
    setScreenBusy(true);
    try {
      const next = !fullscreen;
      if (desktop) await getCurrentWindow().setFullscreen(next);
      else if (next) await root.current?.requestFullscreen();
      else if (document.fullscreenElement) await document.exitFullscreen();
      setFullscreen(next);
      reader.changePreferences({ fullscreen: next });
    } catch (reason) { setControlError(errorMessage(reason)); }
    finally { setScreenBusy(false); }
  }
  useEffect(() => {
    if (!reader.engine || reader.error) return;
    function shortcut(event: KeyboardEvent) {
      if (document.querySelector("[role=dialog]")) return;
      if (event.key === "Escape" && fullscreen) { event.preventDefault(); void toggleFullscreen(); return; }
      if (!reader.preferences?.shortcuts) return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
        event.preventDefault(); searchInput.current?.focus(); return;
      }
      if (event.target instanceof HTMLElement && event.target.closest("input,textarea,select,[contenteditable=true]")) return;
      if (event.ctrlKey || event.altKey || event.metaKey || event.shiftKey && event.key !== "+") return;
      switch (event.key) {
        case "ArrowRight": case "PageDown": event.preventDefault(); reader.goTo(reader.page + 1); break;
        case "ArrowLeft": case "PageUp": event.preventDefault(); reader.goTo(reader.page - 1); break;
        case "Home": event.preventDefault(); reader.goTo(1); break;
        case "End": event.preventDefault(); reader.goTo(reader.engine?.pdf.numPages ?? 1); break;
        case "+": case "=": event.preventDefault(); reader.zoom(10); break;
        case "-": event.preventDefault(); reader.zoom(-10); break;
        case "F11": event.preventDefault(); void toggleFullscreen(); break;
      }
    }
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  }, [reader.engine, reader.error, reader.page, reader.preferences?.shortcuts, fullscreen, screenBusy]);
  const total = reader.engine?.pdf.numPages ?? 0;
  const ready = !!reader.engine && !reader.error;
  function directPage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    reader.goTo(Number(new FormData(event.currentTarget).get("page")));
  }
  return (
    <section ref={root} data-theme={theme} aria-label="PDF reader" className={cn(
      "reader-workspace flex h-[calc(100dvh-9rem)] min-h-112 flex-col overflow-hidden rounded-xl border bg-background text-foreground",
      fullscreen && "fixed inset-0 z-50 h-dvh w-screen rounded-none",
    )}>
      <div className="flex flex-wrap items-center justify-between gap-2 border-b bg-card px-3 py-2">
        <h1 className="min-w-0 flex-1 truncate text-sm font-semibold" title={reader.source?.title}>{reader.source?.title ?? "Opening PDF…"}</h1>
        <Button size="sm" variant="outline" disabled={leaving} onClick={() => void leaveReader()}>{leaving ? "Saving…" : "Close reader"}</Button>
      </div>
      <div role="toolbar" aria-label="Reader controls" className="flex flex-wrap items-center gap-1 border-b bg-card p-2">
        <Button size="icon-sm" variant="ghost" aria-label="Toggle left sidebar" aria-expanded={left} disabled={!ready}
          onClick={() => { setLeft((value) => !value); if (!left && (root.current?.clientWidth ?? 0) < 900) setRight(false); }}><PanelLeft /></Button>
        <Button size="icon-sm" variant="ghost" aria-label="First page" disabled={!ready || reader.page <= 1} onClick={() => reader.goTo(1)}><ChevronFirst /></Button>
        <Button size="icon-sm" variant="ghost" aria-label="Previous page" disabled={!ready || reader.page <= 1} onClick={() => reader.goTo(reader.page - 1)}><ChevronLeft /></Button>
        <form key={reader.page} onSubmit={directPage} className="flex items-center gap-1">
          <Input name="page" type="number" aria-label="Page number" className="h-8 w-16 px-1 text-center" min={1} max={total || 1}
            defaultValue={reader.page} disabled={!ready} required />
          <span className="text-xs tabular-nums text-muted-foreground">/ {total || "—"}</span>
          <Button className="sr-only" disabled={!ready}>Go to page</Button>
        </form>
        <Button size="icon-sm" variant="ghost" aria-label="Next page" disabled={!ready || reader.page >= total} onClick={() => reader.goTo(reader.page + 1)}><ChevronRight /></Button>
        <Button size="icon-sm" variant="ghost" aria-label="Last page" disabled={!ready || reader.page >= total} onClick={() => reader.goTo(total)}><ChevronLast /></Button>
        <Button size="icon-sm" variant="ghost" aria-label="Zoom out" disabled={!ready || reader.scale <= 0.25} onClick={() => reader.zoom(-10)}><ZoomOut /></Button>
        <label className="sr-only" htmlFor="reader-zoom">Zoom</label>
        <select id="reader-zoom" className={cn(fieldClass, "w-28 min-h-8")} disabled={!ready}
          value={reader.preferences?.zoom === "custom" ? String(reader.preferences.scale) : reader.preferences?.zoom ?? "fit_width"}
          onChange={(event) => {
            if (event.target.value === "fit_width" || event.target.value === "fit_page") reader.changePreferences({ zoom: event.target.value });
            else reader.changePreferences({ zoom: "custom", scale: Number(event.target.value) });
          }}>
          <option value="fit_width">Fit width</option><option value="fit_page">Fit page</option>
          {[...new Set([25, 50, 75, 100, 125, 150, 200, 300, 400, reader.preferences?.scale ?? 100])].sort((a, b) => a - b)
            .map((scale) => <option key={scale} value={scale}>{scale}%</option>)}
        </select>
        <Button size="icon-sm" variant="ghost" aria-label="Zoom in" disabled={!ready || reader.scale >= 4} onClick={() => reader.zoom(10)}><ZoomIn /></Button>
        <Button size="icon-sm" variant="ghost" aria-label="Toggle fullscreen" aria-pressed={fullscreen} disabled={!ready || screenBusy} onClick={() => void toggleFullscreen()}><Maximize /></Button>
        <Button size="icon-sm" variant="ghost" aria-label="Toggle right sidebar" aria-expanded={right} disabled={!ready}
          onClick={() => { setRight((value) => !value); if (!right && (root.current?.clientWidth ?? 0) < 900) setLeft(false); }}><PanelRight /></Button>
        <span className="ml-auto text-xs tabular-nums text-muted-foreground" role="status">{total ? `${Math.round(reader.page / total * 100)}% · ${Math.round(reader.scale * 100)}%` : "Loading PDF"}</span>
      </div>
      <form role="search" aria-label="Search current PDF" className="flex flex-wrap items-center gap-2 border-b bg-card p-2"
        onSubmit={(event) => { event.preventDefault(); reader.search(find); }}>
        <Input ref={searchInput} className="h-8 min-w-24 flex-1" aria-label="Find in PDF" maxLength={256} placeholder="Find in PDF · Ctrl+F" value={find}
          disabled={!ready} onChange={(event) => setFind(event.target.value)} />
        <Button size="sm" variant="outline" disabled={!ready} aria-label="Search PDF"><Search /></Button>
        <Button type="button" size="icon-sm" variant="ghost" aria-label="Previous match" disabled={!ready || !find} onClick={() => reader.search(find, true, true)}><ChevronLeft /></Button>
        <Button type="button" size="icon-sm" variant="ghost" aria-label="Next match" disabled={!ready || !find} onClick={() => reader.search(find, false, true)}><ChevronRight /></Button>
        <span role="status" className="text-xs text-muted-foreground">{reader.matches.pending ? "Searching…" : `${reader.matches.current} of ${reader.matches.total} matches`}</span>
      </form>
      {reader.persistenceError && <p role="alert" className="border-b px-3 py-1 text-xs text-destructive">{reader.persistenceError}</p>}
      {controlError && <p role="alert" className="border-b px-3 py-1 text-xs text-destructive">{controlError}</p>}
      {reader.source?.staleProgress && <p className="border-b px-3 py-1 text-xs text-muted-foreground">The source file has changed. Check the resumed page and older annotations.</p>}
      <div className="relative flex min-h-0 flex-1">
        {left && reader.engine && <div className="w-36 shrink-0 sm:w-44">
          <ReaderSidebar engine={reader.engine} outline={reader.outline} page={reader.page} goTo={reader.goTo}
            navigate={(destination) => { void reader.navigateOutline(destination).catch(() => setControlError("This outline destination is unavailable.")); }} />
        </div>}
        <div className="relative min-w-0 flex-1 bg-secondary">
          <div ref={reader.container} className="reader-pages absolute inset-0 overflow-auto" tabIndex={0} aria-label="PDF pages">
            <div ref={reader.viewerElement} className="pdfViewer" />
          </div>
          {!ready && <div className="absolute inset-0 z-10 grid place-items-center overflow-auto bg-background/95 p-4">
            {reader.error ? <div role="alert" className="max-w-md space-y-3 text-center text-sm">
              <p>{reader.error}</p><Button variant="outline" onClick={reader.retry}>Reopen PDF</Button>
            </div> : reader.password ? <form onSubmit={(event) => {
              event.preventDefault();
              reader.password?.submit(String(new FormData(event.currentTarget).get("password") ?? ""));
              event.currentTarget.reset();
            }} className="grid w-full max-w-xs gap-3">
              <p className="text-sm">{reader.password.incorrect ? "Incorrect password. Try again." : "This PDF requires a password."}</p>
              <Input type="password" name="password" aria-label="PDF password" autoComplete="off" required />
              <Button>Unlock PDF</Button>
            </form> : <p role="status" className="text-sm text-muted-foreground">Opening PDF…</p>}
          </div>}
        </div>
        {right && reader.source && <aside aria-label="Reader tools" className="w-48 shrink-0 overflow-y-auto border-l bg-card p-3 sm:w-64">
          <div role="group" aria-label="Reader tools tabs" className="mb-3 flex flex-wrap gap-1">
            <Button size="sm" variant={!settings ? "secondary" : "ghost"} aria-pressed={!settings} onClick={() => setSettings(false)}>Saved</Button>
            <Button size="sm" variant={settings ? "secondary" : "ghost"} aria-pressed={settings} onClick={() => setSettings(true)}>Settings</Button>
          </div>
          {!settings ? <ReaderAnnotations source={reader.source} page={reader.page} ready={reader.progressReady} goTo={reader.goTo} /> : <div className="space-y-4">
            <label className="grid gap-1 text-xs font-medium">Reader appearance
              <select className={fieldClass} value={themePreference} onChange={(event) => {
                void changeTheme("reader", event.target.value as "light" | "dark" | "system").catch((reason) => setControlError(errorMessage(reason)));
              }}>
                <option value="light">Light</option><option value="dark">Dark</option><option value="system">System</option>
              </select>
            </label>
            <label className="grid gap-1 text-xs font-medium">Page layout
              <select className={fieldClass} value={reader.preferences?.layout} onChange={(event) =>
                reader.changePreferences({ layout: event.target.value as "single" | "continuous" })}>
                <option value="continuous">Continuous</option><option value="single">Single page</option>
              </select>
            </label>
            <label className="flex items-center gap-2 text-xs"><input type="checkbox" checked={reader.preferences?.shortcuts ?? true}
              onChange={(event) => reader.changePreferences({ shortcuts: event.target.checked })} />Keyboard shortcuts</label>
            <p className="text-xs leading-5 text-muted-foreground">← / → or Page Up / Down change pages. Home / End go to the first / last page. + / − zoom. Ctrl+F finds text. F11 toggles fullscreen. Escape exits fullscreen.</p>
            <p className="text-xs text-muted-foreground">Zoom, layout, fullscreen, and appearance are saved for your next book.</p>
          </div>}
        </aside>}
      </div>
    </section>
  );
}
export function ReaderPage() {
  const [params] = useSearchParams();
  const book = Number(params.get("book"));
  const file = params.has("file") ? Number(params.get("file")) : null;
  const targetPage = params.has("page") ? Number(params.get("page")) : null;
  const revision = params.has("revision") ? Number(params.get("revision")) : null;
  const invalidTarget = targetPage !== null && (!Number.isSafeInteger(targetPage) || targetPage < 1 || targetPage > 1_000_000)
    || revision !== null && (!Number.isSafeInteger(revision) || revision < 1 || file === null);
  if (!desktop || invalidTarget || !Number.isSafeInteger(book) || book <= 0 || file !== null && (!Number.isSafeInteger(file) || file <= 0)) {
    return <section className="grid min-h-80 place-items-center rounded-xl border bg-card p-6 text-center">
      <div><BookOpen className="mx-auto mb-4 size-7 text-muted-foreground" />
        <h1 className="font-display text-3xl">Open a book to read</h1>
        <p className="mt-3 text-sm text-muted-foreground">{desktop ? "Choose a book in your library and select Read." : "Open the Windows app to read your local PDFs."}</p>
        <Button asChild className="mt-4"><Link to="/library">Open library</Link></Button>
      </div>
    </section>;
  }
  return <ReaderWorkspace key={`${book}-${file}-${targetPage}-${revision}`} book={book} file={file} targetPage={targetPage} revision={revision} />;
}
