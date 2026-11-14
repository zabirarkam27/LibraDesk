import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import type { BookFile } from "./catalog-client";

const events = ["pdf-write-updated", "catalog-updated", "metadata-updated"];
type Status = { fileId: number; status: "not_requested" | "queued" | "running" | "completed" | "failed"; error: string | null; updatedAt: number | null };
export function PdfMetadataStatus({ book, files }: { book: number; files: BookFile[] }) {
  const query = useCallback(() => invoke<Status[]>("pdf_metadata_status", { book }), [book]);
  const { data, error, retry } = useMetadataQuery(query, events);
  const [busy, setBusy] = useState<number | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  async function action(file: number, command: "retry_pdf_metadata" | "cancel_pdf_metadata") {
    setBusy(file); setFailure(null);
    try { await invoke(command, { file }); retry(); }
    catch (reason) { setFailure(errorMessage(reason)); }
    finally { setBusy(null); }
  }
  return <section className="mt-5 rounded-xl border bg-card p-5" aria-label="PDF metadata sync">
    <h2 className="text-lg font-semibold">PDF metadata sync</h2>
    <p className="mt-2 text-sm text-muted-foreground">Book edits are saved to this library and queued for each attached PDF. The writer uses an atomic replacement without a safety backup. Title, authors, subject/genre, publishing fields and organization are embedded; source filenames stay the same.</p>
    <p className="mt-2 text-xs text-muted-foreground">Close the reader to allow pending writes. Notes, bookmarks, progress and custom thumbnails stay in the library. Stopping sync prevents pending work; it does not undo metadata already written. Restoring a library backup does not rewrite PDFs automatically.</p>
    {error ? <div className="mt-3"><p role="alert" className="text-sm text-destructive">PDF save status could not be loaded.</p><Button className="mt-2" variant="outline" size="sm" onClick={retry}>Reload sync status</Button></div> :
      !data ? <p role="status" className="mt-3 text-sm">Loading PDF save status…</p> :
      !data.length ? <p className="mt-3 text-sm text-muted-foreground">No PDF is attached.</p> :
      <ul className="mt-4 space-y-3">{data.map(item => <li key={item.fileId} className="rounded-md border p-3 text-sm">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="min-w-0 wrap-anywhere">{files.find(file => file.id === item.fileId)?.path.split(/[/\\]/).pop() ?? "PDF"} · <strong>{({ not_requested: "No sync requested", queued: "Queued · waits for an open reader", running: "Writing PDF metadata…", completed: "Saved to PDF", failed: "PDF save failed" })[item.status]}</strong></p>
          <div className="flex gap-2">
            {item.status !== "running" && item.status !== "queued" && <Button variant="outline" size="sm" disabled={busy !== null} onClick={() => void action(item.fileId, "retry_pdf_metadata")}>{item.status === "failed" ? "Retry PDF save" : "Sync PDF metadata"}</Button>}
            {(item.status === "failed" || item.status === "queued") && <Button variant="outline" size="sm" disabled={busy !== null} onClick={() => void action(item.fileId, "cancel_pdf_metadata")}>Stop sync</Button>}
          </div>
        </div>
        {item.error && <p className="mt-2 text-destructive" role="alert">{item.error}</p>}
        {item.updatedAt && <p className="mt-1 text-xs text-muted-foreground">{new Date(item.updatedAt).toLocaleString()}</p>}
      </li>)}</ul>}
    {failure && <p role="alert" className="mt-3 text-sm text-destructive">{failure}</p>}
  </section>;
}
