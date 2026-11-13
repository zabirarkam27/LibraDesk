import type { PDFPageProxy } from "pdfjs-dist";
import { desktop } from "@/shared/lib/preferences-client";
import { beginBackgroundWork,backgroundPaused } from "@/shared/lib/background-work";
import { isReaderActive } from "@/features/reader/reader-client";
import { claim, commit, fail, read, release, renew, type Failure, type PageText, type TextTask } from "./client";

const PAGE_BYTES = 256 * 1024;
const encoder = new TextEncoder();
const yieldToUi = () => new Promise<void>((resolve) => window.setTimeout(resolve, 0));
const paused = () => document.hidden || isReaderActive() || backgroundPaused();
class Stop extends Error {}
class Failed extends Error { constructor(public reason: Failure) { super(reason); } }

function classify(error: unknown): Failure {
  if (error instanceof Failed) return error.reason;
  const value = error as { code?: string; name?: string };
  if (value?.code === "reader_source_unavailable") return "source_unavailable";
  if (value?.name === "PasswordException") return "password_required";
  if (value?.name === "InvalidPDFException") return "invalid_pdf";
  return "extraction_failed";
}

// Streaming bounds each page's retained text, including PDFs with very large content streams.
export async function extractPage(page: PDFPageProxy, shouldStop: () => boolean): Promise<string> {
  const stream = page.streamTextContent({ disableNormalization: true }).getReader();
  const parts: string[] = [];
  let bytes = 0;
  try {
    while (true) {
      if (shouldStop()) throw new Stop();
      const chunk = await stream.read();
      if (chunk.done) break;
      for (const item of chunk.value.items) {
        if (!("str" in item)) continue;
        const part = item.str + (item.hasEOL ? "\n" : "");
        bytes += encoder.encode(part).length;
        if (bytes > PAGE_BYTES) throw new Failed("page_text_limit");
        if (part) parts.push(part);
      }
      await yieldToUi();
    }
    return parts.join("").replaceAll("\0", "");
  } finally {
    // The owning task destroys the PDF worker on interruption. Cancelling a stream
    // here races PDF.js's already queued chunks with a closed stream controller.
    stream.releaseLock();
    page.cleanup();
  }
}

export async function processTextTask(task: TextTask): Promise<void> {
  const { loadRangedPdf } = await import("@/features/reader/pdf-source");
  let rejectFailure!: (reason: unknown) => void;
  const failure = new Promise<never>((_, reject) => { rejectFailure = reject; });
  // Each awaited operation races transport, lease loss, and the overall time limit.
  const guard = <T,>(operation: Promise<T>) => Promise.race([operation, failure]);
  const pdf = loadRangedPdf(task.length, (offset, length) => read(task, offset, length), rejectFailure);
  let stopped = false;
  let renewing = false;
  const heartbeat = window.setInterval(() => {
    if (renewing) return;
    renewing = true;
    void renew(task).then((owned) => { if (!owned) rejectFailure(new Stop()); }, rejectFailure)
      .finally(() => { renewing = false; });
  }, 15_000);
  const timeout = window.setTimeout(() => rejectFailure(new Failed("extraction_timeout")), 90_000);
  pdf.loading.onPassword = () => rejectFailure(new Failed("password_required"));
  const startedAt = performance.now();
  const pages: PageText[] = [];
  try {
    const document = await guard(pdf.loading.promise);
    if (task.totalPages !== null && task.totalPages !== document.numPages) throw new Failed("source_unavailable");
    if (document.numPages > 1_000_000) throw new Failed("invalid_pdf");
    let number = task.nextPage;
    for (; number <= document.numPages; number++) {
      if (paused()) break;
      const page = await guard(document.getPage(number));
      const text = await guard(extractPage(page, () => stopped || paused()));
      pages.push({ number, text });
      if (pages.length === 4 || number === document.numPages) {
        if (!await guard(commit(task, document.numPages, pages))) throw new Stop();
        pages.length = 0;
      }
      await yieldToUi();
      if (number - task.nextPage >= 31 || performance.now() - startedAt > 10_000) break;
    }
    if (pages.length && !await guard(commit(task, document.numPages, pages))) throw new Stop();
  } catch (error) {
    if (!(error instanceof Stop) && (error as {code?: string})?.code !== "extraction_stale") {
      await fail(task, classify(error));
    }
  } finally {
    stopped = true;
    window.clearInterval(heartbeat);
    window.clearTimeout(timeout);
    pdf.transport.abort();
    await pdf.loading.destroy().catch(() => undefined);
    await release(task);
  }
}
let started = false;
export function startTextExtractionWorker() {
  if (!desktop || started) return;
  started = true;
  async function run() {
    let delay = 1500;
    const finish=beginBackgroundWork();
    try {
      if(!finish)return;
      if (!paused()) {
        const task = await claim();
        if (task) { await processTextTask(task); delay = 100; }
      }
    } catch { delay = 5000; }
    finally { finish?.();window.setTimeout(() => void run(), delay); }
  }
  void run();
}
