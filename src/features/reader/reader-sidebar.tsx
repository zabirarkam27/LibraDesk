import { useEffect, useMemo, useRef, useState } from "react";
import type { PDFPageProxy, RenderTask } from "pdfjs-dist";
import { Button } from "@/shared/ui/button";
import type { Outline, ReaderEngine } from "./use-reader";

function OutlineItems({ items, navigate }: { items: Outline; navigate: (destination: string | unknown[]) => void }) {
  const [limit, setLimit] = useState(100);
  return <ul className="space-y-1">
    {items.slice(0, limit).map((item, index) => <li key={index}>
      <OutlineItem item={item} navigate={navigate} />
    </li>)}
    {items.length > limit && <li><Button size="sm" variant="outline" onClick={() => setLimit((value) => value + 100)}>More contents</Button></li>}
  </ul>;
}
function OutlineItem({ item, navigate }: { item: Outline[number]; navigate: (destination: string | unknown[]) => void }) {
  const [expanded, setExpanded] = useState(false);
  return <div>
    <div className="flex items-start gap-1">
      {item.items.length > 0 && <Button variant="ghost" size="icon-xs" aria-label={`${expanded ? "Collapse" : "Expand"} ${item.title}`}
        aria-expanded={expanded} onClick={() => setExpanded((value) => !value)}>{expanded ? "−" : "+"}</Button>}
      <button className="min-w-0 rounded-sm py-1 text-left text-xs hover:underline focus-visible:outline-2 focus-visible:outline-ring disabled:text-muted-foreground"
        disabled={!item.dest} onClick={() => { if (item.dest) navigate(item.dest); }}>{item.title || "Untitled section"}</button>
    </div>
    {expanded && <div className="border-l pl-3"><OutlineItems items={item.items} navigate={navigate} /></div>}
  </div>;
}

function Thumbnail({ engine, page, queue, select, current }: {
  engine: ReaderEngine; page: number; queue: { pending: Promise<unknown> }; select: () => void; current: boolean;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    let disposed = false;
    let render: RenderTask | undefined;
    const element = canvas.current;
    const work = queue.pending.catch(() => undefined).then(async () => {
      if (disposed || !element) return;
      let pdfPage: PDFPageProxy | undefined;
      try {
        pdfPage = await engine.pdf.getPage(page);
        if (disposed) return;
        const size = pdfPage.getViewport({ scale: 1 });
        const viewport = pdfPage.getViewport({ scale: Math.min(104 / size.width, 112 / size.height) });
        element.width = Math.max(1, Math.ceil(viewport.width));
        element.height = Math.max(1, Math.ceil(viewport.height));
        render = pdfPage.render({ canvas: element, viewport, annotationMode: 0 });
        await render.promise;
        if (!disposed) engine.bus.dispatch("thumbnailrendered", { pageNumber: page, pdfPage });
      } catch { /* The main page and page number remain available when a thumbnail fails. */ }
    });
    queue.pending = work;
    return () => { disposed = true; render?.cancel(); if (element) { element.width = 0; element.height = 0; } };
  }, [engine, page, queue]);
  return <button className="grid h-36 w-full content-center justify-items-center gap-1 rounded-md border bg-card text-xs aria-[current=page]:border-ring aria-[current=page]:ring-2 aria-[current=page]:ring-ring focus-visible:outline-2 focus-visible:outline-ring"
    aria-label={`Go to page ${page}`} aria-current={current ? "page" : undefined} onClick={select}>
    <canvas ref={canvas} aria-hidden="true" className="max-h-28 max-w-28 bg-white" /><span>Page {page}</span>
  </button>;
}
function Thumbnails({ engine, page, goTo }: { engine: ReaderEngine; page: number; goTo: (page: number) => void }) {
  const viewport = useRef<HTMLDivElement>(null);
  const [range, setRange] = useState({ start: 0, end: 5 });
  const queue = useMemo(() => ({ pending: Promise.resolve() as Promise<unknown> }), [engine]);
  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const measure = () => setRange({ start: Math.max(0, Math.floor(element.scrollTop / 152) - 1),
      end: Math.min(engine.pdf.numPages, Math.ceil((element.scrollTop + element.clientHeight) / 152) + 1) });
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    element.addEventListener("scroll", measure, { passive: true });
    measure();
    return () => { observer.disconnect(); element.removeEventListener("scroll", measure); };
  }, [engine]);
  return <div ref={viewport} className="min-h-0 flex-1 overflow-y-auto" aria-label="Page thumbnails">
    <div className="relative" style={{ height: engine.pdf.numPages * 152 }}>
      <div className="absolute inset-x-0 space-y-2 px-2" style={{ top: range.start * 152 }}>
        {Array.from({ length: Math.max(0, range.end - range.start) }, (_, index) => range.start + index + 1).map((number) =>
          <Thumbnail key={number} engine={engine} page={number} queue={queue} current={number === page} select={() => goTo(number)} />)}
      </div>
    </div>
  </div>;
}
export function ReaderSidebar({ engine, outline, page, goTo, navigate }: {
  engine: ReaderEngine; outline: Outline; page: number; goTo: (page: number) => void;
  navigate: (destination: string | unknown[]) => void;
}) {
  const [tab, setTab] = useState<"contents" | "pages">("contents");
  return <aside className="flex h-full min-h-0 flex-col border-r bg-card" aria-label="PDF navigation">
    <div role="group" aria-label="Navigation sidebar" className="flex flex-wrap gap-1 border-b p-2">
      <Button size="sm" variant={tab === "contents" ? "secondary" : "ghost"} aria-pressed={tab === "contents"} onClick={() => setTab("contents")}>Contents</Button>
      <Button size="sm" variant={tab === "pages" ? "secondary" : "ghost"} aria-pressed={tab === "pages"} onClick={() => setTab("pages")}>Pages</Button>
    </div>
    {tab === "contents" ? <div className="min-h-0 flex-1 overflow-y-auto p-2">
      {outline.length ? <OutlineItems items={outline} navigate={navigate} /> : <p className="p-2 text-xs text-muted-foreground">This PDF has no table of contents.</p>}
    </div> : <Thumbnails engine={engine} page={page} goTo={goTo} />}
  </aside>;
}
