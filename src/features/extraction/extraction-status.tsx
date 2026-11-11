import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/shared/ui/button";
import { getStatus, retry, type ExtractionStatus as Status } from "./client";

const errors: Record<string, string> = {
  password_required: "Password-protected PDF. Background extraction requires an unencrypted copy.",
  invalid_pdf: "This PDF could not be parsed.",
  source_unavailable: "The PDF changed or is unavailable. Check its drive and rescan.",
  page_text_limit: "A page exceeds the 256 KiB text limit.",
  extraction_timeout: "Extraction timed out.",
  extraction_failed: "Text extraction failed.",
};

export function ExtractionStatus({ file, revision, available }: { file: number; revision: number; available: boolean }) {
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    let timer: number | undefined;
    let sequence = 0;
    const refresh = async () => {
      const request = ++sequence;
      try {
        const value = await getStatus(file);
        if (active && request === sequence) { setStatus(value); setError(false); }
      } catch { if (active && request === sequence) setError(true); }
    };
    const listening = listen<number>("extraction-updated", ({ payload }) => {
      if (payload !== file || timer !== undefined) return;
      timer = window.setTimeout(() => { timer = undefined; void refresh(); }, 500);
    }).catch(() => { if (active) setError(true); return null; });
    void refresh();
    return () => {
      active = false;
      window.clearTimeout(timer);
      void listening.then((remove) => remove?.());
    };
  }, [file, revision, available]);
  async function retryExtraction() {
    if (!status) return;
    setBusy(true); setError(false);
    try { await retry(file, status.revision); setStatus(await getStatus(file)); }
    catch { setError(true); }
    finally { setBusy(false); }
  }
  const progress = status ? `${status.completedPages}${status.totalPages === null ? "" : ` / ${status.totalPages}`} pages` : "";
  return <div className="mt-2 space-y-1 text-xs text-muted-foreground">
    {status && <p role="status">PDF text: {status.status === "queued" ? "Queued; runs while the reader is closed" :
      status.status === "running" ? "Extracting" : status.status === "completed" ? "Complete" : "Failed"} · {progress}
      {status.status === "completed" && ` · ${status.pagesWithText} with text`}</p>}
    {status?.status === "completed" && status.pagesWithText === 0 && <p>No embedded text found. OCR is not included.</p>}
    {status?.error && <p>{errors[status.error] ?? errors.extraction_failed}</p>}
    {status?.status === "failed" && <Button size="sm" variant="outline" disabled={busy || !available}
      onClick={() => void retryExtraction()}>{busy ? "Retrying…" : "Retry text extraction"}</Button>}
    {error && <p role="alert" className="text-destructive">Could not update extraction status. Reopen book details to refresh.</p>}
  </div>;
}
