import { useEffect, useRef, useState } from "react";
import type { PDFDocumentProxy, PDFDocumentLoadingTask } from "pdfjs-dist";
import type { EventBus, PDFViewer, PDFLinkService, GenericL10n } from "pdfjs-dist/web/pdf_viewer.mjs";
import { errorMessage } from "@/shared/lib/preferences-client";
import { loadReaderPdf } from "./pdf-source";
import { createProgressWriter } from "./progress-writer";
import {
  closeReader, openReader, readerLifecycle, registerReaderShutdown, saveReaderPreferences,
  type ReaderDocument, type ReaderPreferences,
} from "./reader-client";

export type Outline = NonNullable<Awaited<ReturnType<PDFDocumentProxy["getOutline"]>>>;
export type ReaderEngine = { pdf: PDFDocumentProxy; viewer: PDFViewer; bus: EventBus; link: PDFLinkService };
export function useReader(book: number, file: number | null, targetPage: number | null = null, expectedRevision: number | null = null) {
  const container = useRef<HTMLDivElement>(null);
  const viewerElement = useRef<HTMLDivElement>(null);
  const engineRef = useRef<ReaderEngine | null>(null);
  const preferenceRef = useRef<ReaderPreferences | null>(null);
  const navigationEpoch = useRef(0);
  const [engine, setEngine] = useState<ReaderEngine | null>(null);
  const [source, setSource] = useState<ReaderDocument | null>(null);
  const [page, setPage] = useState(1);
  const [scale, setScale] = useState(1);
  const [preferences, setPreferences] = useState<ReaderPreferences | null>(null);
  const [outline, setOutline] = useState<Outline>([]);
  const [error, setError] = useState<string | null>(null);
  const [persistenceError, setPersistenceError] = useState<string | null>(null);
  const [progressReady, setProgressReady] = useState(false);
  const [password, setPassword] = useState<{ incorrect: boolean; submit: (value: string) => void } | null>(null);
  const [matches, setMatches] = useState({ current: 0, total: 0, pending: false });
  const [attempt, setAttempt] = useState(0);
  const preferencesQueue = useRef<Promise<unknown>>(Promise.resolve());

  useEffect(() => {
    let disposed = false;
    let descriptor: ReaderDocument | null = null;
    let writer: ReturnType<typeof createProgressWriter> | undefined;
    let loading: PDFDocumentLoadingTask | undefined;
    let viewer: PDFViewer | undefined;
    let l10n: GenericL10n | undefined;
    let transport: { abort: () => void } | undefined;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const abort = new AbortController();
    setError(null);
    setPersistenceError(null);
    setSource(null);
    setEngine(null);
    setPassword(null);
    setProgressReady(false);
    setOutline([]);
    const report = (reason: unknown) => { if (!disposed) setPersistenceError(reason === null ? null : errorMessage(reason)); };
    async function finish() {
      if (writer) await writer.close();
      else if (descriptor) await closeReader(descriptor.sessionId);
    }
    const unregister = registerReaderShutdown(() => readerLifecycle(finish));

    async function start() {
      try {
        descriptor = await readerLifecycle(async () => disposed ? null : openReader(book, file));
        if (disposed || !descriptor || !container.current || !viewerElement.current) return;
        setSource(descriptor);
        if (expectedRevision !== null && descriptor.revision !== expectedRevision) {
          throw new Error("This search result refers to an older PDF revision. Refresh PDF contents search before opening it.");
        }
        preferenceRef.current = descriptor.preferences;
        setPreferences(descriptor.preferences);
        writer = createProgressWriter(descriptor, report);
        const loaded = loadReaderPdf(descriptor, (reason) => {
          if (!disposed) {
            setError(errorMessage(reason));
            writer?.stop();
            void writer?.close().catch(() => descriptor && closeReader(descriptor.sessionId)).catch(() => undefined);
            transport?.abort();
            void loading?.destroy();
          }
        });
        loading = loaded.loading;
        transport = loaded.transport;
        timeout = setTimeout(() => {
          if (!disposed) { setError("The PDF is taking too long to open. Retry or check its drive."); void loading?.destroy(); }
        }, 60_000);
        loading.onPassword = (update: (password: string) => void, reason: number) => {
          clearTimeout(timeout);
          if (!disposed) setPassword({ incorrect: reason === 2, submit: (value) => { setPassword(null); update(value); } });
        };
        const pdf = await loading.promise;
        clearTimeout(timeout);
        if (disposed) return;
        if (pdf.numPages > 1_000_000) throw new Error("This PDF has too many pages to open.");
        if (targetPage !== null && targetPage > pdf.numPages) throw new Error("This search result's page is unavailable. Refresh PDF contents search.");
        const component = await import("pdfjs-dist/web/pdf_viewer.mjs");
        if (disposed || !container.current || !viewerElement.current) return;
        const bus = new component.EventBus();
        const link = new component.PDFLinkService({ eventBus: bus, ignoreDestinationZoom: true });
        link.externalLinkEnabled = false;
        const find = new component.PDFFindController({ eventBus: bus, linkService: link });
        l10n = new component.GenericL10n("en-us");
        const options = {
          container: container.current, viewer: viewerElement.current, eventBus: bus,
          linkService: link, findController: find, l10n, annotationMode: 1,
          maxCanvasPixels: 12_000_000, maxCanvasDim: 8192, enableAutoLinking: false,
          enableDetailCanvas: false, abortSignal: abort.signal,
        };
        viewer = new component.PDFViewer(options);
        link.setViewer(viewer);
        link.setDocument(pdf);
        const initial = targetPage ?? Math.min(pdf.numPages, Math.max(1, descriptor.resumePage));
        let initialized = false;
        bus.on("pagesinit", () => {
          if (disposed || !viewer || !descriptor) return;
          viewer.scrollMode = descriptor.preferences.layout === "single" ? component.ScrollMode.PAGE : component.ScrollMode.VERTICAL;
          viewer.currentScaleValue = descriptor.preferences.zoom === "custom" ? String(descriptor.preferences.scale / 100)
            : descriptor.preferences.zoom === "fit_page" ? "page-fit" : "page-width";
          viewer.currentPageNumber = initial;
          initialized = true;
          setPage(initial);
          setScale(viewer.currentScale);
          const ready = { pdf, viewer, bus, link };
          engineRef.current = ready;
          setEngine(ready);
          void writer?.ready(initial, pdf.numPages).then(() => { if (!disposed) setProgressReady(true); }).catch(report);
        }, { signal: abort.signal });
        bus.on("pagechanging", ({ pageNumber }: { pageNumber: number }) => {
          if (!disposed && initialized) { setPage(pageNumber); writer?.page(pageNumber); }
        }, { signal: abort.signal });
        bus.on("scalechanging", ({ scale }: { scale: number }) => { if (!disposed) setScale(scale); }, { signal: abort.signal });
        bus.on("updatefindmatchescount", ({ matchesCount }: { matchesCount: { current: number; total: number } }) => {
          if (!disposed) setMatches({ ...matchesCount, pending: false });
        }, { signal: abort.signal });
        bus.on("updatefindcontrolstate", ({ state, matchesCount }: { state: number; matchesCount: { current: number; total: number } }) => {
          if (!disposed) setMatches({ ...matchesCount, pending: state === component.FindState.PENDING });
        }, { signal: abort.signal });
        bus.on("pagerendered", ({ error }: { error?: unknown }) => {
          if (!disposed && error) setError("This PDF page could not be rendered. Try reopening the book.");
        }, { signal: abort.signal });
        viewer.setDocument(pdf);
        void pdf.getOutline().then((items) => { if (!disposed) setOutline(items ?? []); }).catch(() => undefined);
      } catch (reason) {
        if (!disposed) {
          setError((current) => current ?? (reason instanceof Error ? reason.message : errorMessage(reason)));
          await finish().catch(report);
          transport?.abort();
          await loading?.destroy().catch(() => undefined);
        }
      }
    }
    void start();
    return () => {
      disposed = true;
      unregister();
      writer?.stop();
      clearTimeout(timeout);
      abort.abort();
      engineRef.current = null;
      viewer?.setDocument(null);
      transport?.abort();
      void loading?.destroy().catch(() => undefined);
      void l10n?.destroy().catch(() => undefined);
      void readerLifecycle(finish).catch(() => { if (descriptor) void closeReader(descriptor.sessionId).catch(() => undefined); });
    };
  }, [book, file, targetPage, expectedRevision, attempt]);

  function goTo(value: number) {
    navigationEpoch.current++;
    const current = engineRef.current;
    if (current) current.viewer.currentPageNumber = Math.min(current.pdf.numPages, Math.max(1, Math.round(value) || 1));
  }
  async function navigateOutline(destination: string | unknown[]) {
    const current = engineRef.current;
    if (!current) return;
    const epoch = ++navigationEpoch.current;
    const explicit = typeof destination === "string" ? await current.pdf.getDestination(destination) : destination;
    if (!Array.isArray(explicit)) throw new Error("This outline destination is unavailable.");
    const reference = explicit[0];
    let pageNumber: number;
    if (typeof reference === "number" && Number.isInteger(reference)) pageNumber = reference + 1;
    else if (reference && typeof reference === "object" && "num" in reference && "gen" in reference) {
      pageNumber = await current.pdf.getPageIndex({ num: Number(reference.num), gen: Number(reference.gen) }) + 1;
    } else throw new Error("This outline page is unavailable.");
    if (engineRef.current !== current || navigationEpoch.current !== epoch) return;
    if (pageNumber < 1 || pageNumber > current.pdf.numPages) throw new Error("This outline page is unavailable.");
    current.viewer.scrollPageIntoView({ pageNumber, destArray: explicit, ignoreDestinationZoom: true });
  }
  function changePreferences(update: Partial<ReaderPreferences>) {
    const previous = preferenceRef.current;
    if (!previous) return;
    const next = { ...previous, ...update };
    preferenceRef.current = next;
    setPreferences(next);
    const viewer = engineRef.current?.viewer;
    if (viewer) {
      viewer.scrollMode = next.layout === "single" ? 3 : 0;
      viewer.currentScaleValue = next.zoom === "custom" ? String(next.scale / 100) : next.zoom === "fit_page" ? "page-fit" : "page-width";
    }
    const saved = preferencesQueue.current.catch(() => undefined).then(() => saveReaderPreferences(next));
    preferencesQueue.current = saved;
    void saved.catch((reason) => setPersistenceError(errorMessage(reason)));
  }
  function zoom(delta: number) {
    const current = engineRef.current;
    if (current) changePreferences({ zoom: "custom", scale: Math.min(400, Math.max(25, Math.round(current.viewer.currentScale * 100) + delta)) });
  }
  function search(query: string, previous = false, again = false) {
    engineRef.current?.bus.dispatch("find", { source: null, type: again ? "again" : "", query,
      phraseSearch: true, caseSensitive: false, entireWord: false, highlightAll: true, findPrevious: previous,
      matchDiacritics: false });
  }
  return { container, viewerElement, source, engine, page, scale, preferences, outline, error, persistenceError, progressReady, password, matches,
    goTo, navigateOutline, zoom, search, changePreferences, retry: () => setAttempt((value) => value + 1) };
}
