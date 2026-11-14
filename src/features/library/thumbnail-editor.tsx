import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/shared/ui/dialog";
import { errorMessage } from "@/shared/lib/preferences-client";
import type { BookDetails } from "./catalog-client";

async function normalizeCover(file: File) {
  if (!["image/png", "image/jpeg", "image/webp"].includes(file.type) || file.size > 10 * 1024 * 1024) {
    throw new Error("Choose a PNG, JPEG or WebP image up to 10 MiB.");
  }
  const image = await createImageBitmap(file);
  try {
    if (image.width * image.height > 32_000_000 || !image.width || !image.height) throw new Error("Choose an image up to 32 million pixels.");
    const ratio = Math.min(240 / image.width, 320 / image.height, 1);
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(image.width * ratio));
    canvas.height = Math.max(1, Math.round(image.height * ratio));
    const context = canvas.getContext("2d");
    if (!context) throw new Error("Image conversion is unavailable.");
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    const png = await new Promise<Blob>((resolve, reject) => canvas.toBlob(value => value ? resolve(value) : reject(new Error("Could not convert the thumbnail.")), "image/png"));
    if (png.size > 512 * 1024) throw new Error("The converted thumbnail is too large.");
    return png;
  } finally { image.close(); }
}
export function ThumbnailEditor({ book, saved }: { book: BookDetails; saved: () => void }) {
  const [open, setOpen] = useState(false);
  const [preview, setPreview] = useState<{ png: Blob; url: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(true);
  const selected = useRef(0);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; selected.current++; }; }, []);
  useEffect(() => () => { if (preview) URL.revokeObjectURL(preview.url); }, [preview]);
  function close() {
    if (busy) return;
    selected.current++; setPreview(null); setError(null); setOpen(false);
  }
  async function choose(file: File | undefined) {
    const sequence = ++selected.current;
    setError(null); setPreview(null);
    if (!file) return;
    setBusy(true);
    try {
      const png = await normalizeCover(file);
      if (mounted.current && sequence === selected.current) setPreview({ png, url: URL.createObjectURL(png) });
    } catch (reason) { if (mounted.current && sequence === selected.current) setError(errorMessage(reason)); }
    finally { if (mounted.current && sequence === selected.current) setBusy(false); }
  }
  async function save(remove = false) {
    if (busy || !remove && !preview) return;
    setBusy(true); setError(null);
    try {
      const png = remove ? null : Array.from(new Uint8Array(await preview!.png.arrayBuffer()));
      await invoke("save_custom_cover", { book: book.id, revision: book.revision, png });
      setPreview(null); setOpen(false); saved();
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setBusy(false); }
  }
  return <>
    <Button variant="outline" size="sm" disabled={!book.files.length} onClick={() => { setError(null); setOpen(true); }}>Change thumbnail</Button>
    <Dialog open={open} onOpenChange={value => { if (!value) close(); }}>
      <DialogContent showCloseButton={!busy} onEscapeKeyDown={event => { if (busy) event.preventDefault(); }} onPointerDownOutside={event => { if (busy) event.preventDefault(); }}>
        <DialogHeader><DialogTitle>Book thumbnail</DialogTitle><DialogDescription>Choose an image and preview it before saving. Custom thumbnails are kept with library data, included in database backups, and retained when generated caches are cleared. The PDF's pages are not changed.</DialogDescription></DialogHeader>
        <label className="grid gap-2 text-sm font-medium">Image file
          <input type="file" accept="image/png,image/jpeg,image/webp" disabled={busy} onChange={event => void choose(event.target.files?.[0])} />
        </label>
        <p className="text-xs text-muted-foreground">PNG, JPEG or WebP · up to 10 MiB · converted to a cover up to 240 × 320 pixels.</p>
        {preview && <img src={preview.url} alt="Selected custom thumbnail" className="mx-auto h-64 max-w-full rounded-md border object-contain" />}
        {busy && <p role="status" className="text-sm">Processing thumbnail…</p>}
        {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
        <DialogFooter className="flex-wrap">
          {!!book.customCoverRevision && <Button variant="outline" disabled={busy} onClick={() => void save(true)}>Use automatic thumbnail</Button>}
          <Button variant="outline" disabled={busy} onClick={close}>Cancel</Button>
          <Button disabled={busy || !preview} onClick={() => void save()}>Save thumbnail</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </>;
}
