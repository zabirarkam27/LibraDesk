import { AnnotationMode, getDocument, GlobalWorkerOptions, PDFDataRangeTransport, type PDFDocumentProxy } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { coverRange, type CoverResult, type CoverTask, type RawMetadata } from "./client";

const CHUNK_SIZE = 64 * 1024;
const IPC_LIMIT = 256 * 1024;
const REQUEST_LIMIT = 8 * 1024 * 1024;
export const MAX_SOURCE_BYTES = 256 * 1024 * 1024;

GlobalWorkerOptions.workerSrc = workerUrl;

class CoverTransport extends PDFDataRangeTransport {
  private stopped = false;

  constructor(private readonly task: CoverTask, private readonly fail: (error: unknown) => void) {
    super(task.length, new Uint8Array(), true);
  }

  override requestDataRange(begin: number, end: number) {
    void this.read(begin, end).catch((error) => {
      if (!this.stopped) this.fail(error);
    });
  }

  override abort() {
    this.stopped = true;
  }

  private async read(begin: number, end: number) {
    if (end <= begin || end - begin > REQUEST_LIMIT) throw new Error("PDF range exceeds the cover limit");
    const bytes = new Uint8Array(end - begin);
    for (let offset = begin; offset < end && !this.stopped; offset += IPC_LIMIT) {
      const length = Math.min(IPC_LIMIT, end - offset);
      const chunk = new Uint8Array(await coverRange(this.task, offset, length));
      if (chunk.length !== length) throw new Error("Incomplete PDF range");
      bytes.set(chunk, offset - begin);
    }
    if (!this.stopped) this.onDataRange(begin, bytes);
  }
}

async function readMetadata(pdf: PDFDocumentProxy): Promise<RawMetadata> {
  const { info } = await pdf.getMetadata();
  const values = info as Record<string, unknown>;
  const string = (key: string) => typeof values[key] === "string" ? values[key] : null;
  return {
    title: string("Title"), author: string("Author"), subject: string("Subject"),
    creator: string("Creator"), producer: string("Producer"), creationDateRaw: string("CreationDate"),
    pageCount: pdf.numPages || null,
  };
}

export async function renderCover(task: CoverTask): Promise<CoverResult> {
  let rejectRange!: (error: unknown) => void;
  const rangeFailure = new Promise<never>((_, reject) => { rejectRange = reject; });
  const transport = new CoverTransport(task, rejectRange);
  const loading = getDocument({
    range: transport,
    rangeChunkSize: CHUNK_SIZE,
    disableAutoFetch: true,
    disableStream: true,
    useWasm: false,
    useSystemFonts: true,
    maxImageSize: 12_000_000,
  });
  let timer: ReturnType<typeof setTimeout> | undefined;
  const deadline = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error("Cover rendering timed out")), 45_000);
  });
  const guard = <T,>(operation: Promise<T>) => Promise.race([operation, rangeFailure, deadline]);
  let raw: RawMetadata | null = null;
  let canvas: HTMLCanvasElement | undefined;
  try {
    const pdf = await guard(loading.promise);
    if (task.needsMetadata) raw = await guard(readMetadata(pdf)).catch(() => null);
    const page = await guard(pdf.getPage(1));
    const size = page.getViewport({ scale: 1 });
    if (!(size.width > 0 && size.height > 0)) throw new Error("Invalid PDF page dimensions");
    const viewport = page.getViewport({ scale: Math.min(240 / size.width, 320 / size.height) });
    canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.min(240, Math.round(viewport.width)));
    canvas.height = Math.max(1, Math.min(320, Math.round(viewport.height)));
    await guard(page.render({ canvas, viewport, annotationMode: AnnotationMode.DISABLE, background: "#ffffff" }).promise);
    const output = await guard(new Promise<Blob>((resolve, reject) => {
      canvas?.toBlob((blob) => blob ? resolve(blob) : reject(new Error("Unable to encode PNG")), "image/png");
    }));
    return { raw, png: Array.from(new Uint8Array(await output.arrayBuffer())) };
  } catch {
    return { raw, png: null };
  } finally {
    clearTimeout(timer);
    transport.abort();
    await loading.destroy().catch(() => undefined);
    if (canvas) { canvas.width = 0; canvas.height = 0; }
  }
}
