import { getDocument, GlobalWorkerOptions, PDFDataRangeTransport } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { readRange, type ReaderDocument } from "./reader-client";

GlobalWorkerOptions.workerSrc = workerUrl;
const IPC_LIMIT = 256 * 1024;

class ReaderTransport extends PDFDataRangeTransport {
  private stopped = false;
  constructor(private source: ReaderDocument, private fail: (reason: unknown) => void) {
    super(source.length, new Uint8Array(), true);
  }
  override requestDataRange(begin: number, end: number) {
    void this.read(begin, end).catch((reason) => { if (!this.stopped) this.fail(reason); });
  }
  override abort() { this.stopped = true; }
  private async read(begin: number, end: number) {
    if (!Number.isSafeInteger(begin) || !Number.isSafeInteger(end) || begin < 0 ||
      end <= begin || end > this.source.length) {
      throw new Error("The PDF requested an invalid byte range.");
    }
    // PDF.js expects each requested range in one callback. IPC stays bounded.
    const range = new Uint8Array(end - begin);
    for (let offset = begin; offset < end && !this.stopped; offset += IPC_LIMIT) {
      const length = Math.min(IPC_LIMIT, end - offset);
      const bytes = new Uint8Array(await readRange(this.source.sessionId, offset, length));
      if (bytes.length !== length) throw new Error("The PDF could not be read completely.");
      range.set(bytes, offset - begin);
    }
    if (!this.stopped) this.onDataRange(begin, range);
  }
}

export function loadReaderPdf(source: ReaderDocument, onError: (reason: unknown) => void) {
  const transport = new ReaderTransport(source, onError);
  const base = new URL("pdfjs/", document.baseURI).href;
  const loading = getDocument({
    range: transport, rangeChunkSize: 64 * 1024, disableStream: true, disableAutoFetch: true,
    cMapUrl: base + "cmaps/", cMapPacked: true, standardFontDataUrl: base + "standard_fonts/",
    wasmUrl: base + "wasm/", iccUrl: base + "iccs/", useWasm: true,
    enableXfa: false, useSystemFonts: true, maxImageSize: 32_000_000,
  });
  return { loading, transport };
}
