import { useEffect, useState, type FormEvent } from "react";
import { errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogFooter } from "@/shared/ui/dialog";
import { fieldClass } from "@/features/library/query-feedback";
import { getAnnotations, saveAnnotation, deleteAnnotation, type AnnotationKind, type ReaderAnnotation, type ReaderDocument, type Annotations } from "./reader-client";

function AnnotationEditor({ kind, annotation, source, page, close, saved }: {
  kind: AnnotationKind; annotation: ReaderAnnotation | null; source: ReaderDocument; page: number;
  close: () => void; saved: () => void;
}) {
  const [text, setText] = useState(annotation?.text ?? "");
  const [key] = useState(() => crypto.randomUUID());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await saveAnnotation(kind, { sessionId: source.sessionId, id: annotation?.id ?? null,
        revision: annotation?.revision ?? null, page: annotation?.page ?? page, text, clientKey: key });
      saved();
    } catch (reason) { setError(errorMessage(reason)); setBusy(false); }
  }
  return <Dialog open onOpenChange={(open) => { if (!open && !busy) close(); }}>
    <DialogContent showCloseButton={!busy}>
      <DialogHeader><DialogTitle>{annotation ? "Edit" : "Add"} {kind}</DialogTitle>
        <DialogDescription>Page {annotation?.page ?? page} · {source.title}</DialogDescription></DialogHeader>
      <form onSubmit={(event) => void submit(event)} className="space-y-4">
        <label className="grid gap-2 text-sm">{kind === "bookmark" ? "Bookmark label" : "Note"}
          <textarea className={fieldClass} rows={kind === "note" ? 5 : 2} disabled={busy} required={kind === "note"}
            maxLength={kind === "note" ? 8000 : 128} value={text} onChange={(event) => setText(event.target.value)} />
        </label>
        {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
        <DialogFooter><Button type="button" variant="outline" disabled={busy} onClick={close}>Cancel</Button>
          <Button disabled={busy || kind === "note" && !text.trim()}>{busy ? "Saving…" : "Save"}</Button></DialogFooter>
      </form>
    </DialogContent>
  </Dialog>;
}
function DeleteAnnotation({ kind, annotation, source, close, saved }: {
  kind: AnnotationKind; annotation: ReaderAnnotation; source: ReaderDocument; close: () => void; saved: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function remove() {
    if (busy) return;
    setBusy(true);
    try { await deleteAnnotation(kind, source.sessionId, annotation.id, annotation.revision); saved(); }
    catch (reason) { setError(errorMessage(reason)); setBusy(false); }
  }
  return <Dialog open onOpenChange={(open) => { if (!open && !busy) close(); }}>
    <DialogContent showCloseButton={!busy}>
      <DialogHeader><DialogTitle>Delete {kind}?</DialogTitle><DialogDescription>Remove the saved {kind} on page {annotation.page}.</DialogDescription></DialogHeader>
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
      <DialogFooter><Button variant="outline" disabled={busy} onClick={close}>Cancel</Button>
        <Button variant="destructive" disabled={busy} onClick={() => void remove()}>{busy ? "Deleting…" : "Delete"}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}
export function ReaderAnnotations({ source, page, ready, goTo }: {
  source: ReaderDocument; page: number; ready: boolean; goTo: (page: number) => void;
}) {
  const [kind, setKind] = useState<AnnotationKind>("bookmark");
  const [data, setData] = useState<Annotations | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [editing, setEditing] = useState<{ kind: AnnotationKind; annotation: ReaderAnnotation | null } | null>(null);
  const [removing, setRemoving] = useState<{ kind: AnnotationKind; annotation: ReaderAnnotation } | null>(null);
  useEffect(() => {
    let disposed = false;
    void getAnnotations(source.bookId).then((result) => { if (!disposed) { setData(result); setError(null); } },
      (reason) => { if (!disposed) setError(errorMessage(reason)); });
    return () => { disposed = true; };
  }, [source.bookId, attempt]);
  const items = kind === "bookmark" ? data?.bookmarks : data?.notes;
  return <div className="space-y-3">
    <div role="group" aria-label="Saved annotations" className="flex flex-wrap gap-1">
      <Button size="sm" variant={kind === "bookmark" ? "secondary" : "ghost"} aria-pressed={kind === "bookmark"} onClick={() => setKind("bookmark")}>Bookmarks</Button>
      <Button size="sm" variant={kind === "note" ? "secondary" : "ghost"} aria-pressed={kind === "note"} onClick={() => setKind("note")}>Notes</Button>
    </div>
    <Button variant="outline" size="sm" disabled={!ready} onClick={() => setEditing({ kind, annotation: null })}>Add {kind}</Button>
    {error && <div role="alert" className="space-y-2 text-xs text-destructive">
      <p>{error}</p><Button size="sm" variant="outline" onClick={() => setAttempt((value) => value + 1)}>Reload annotations</Button>
    </div>}
    {!data && !error && <p role="status" className="text-xs text-muted-foreground">Loading annotations…</p>}
    {items?.length === 0 && <p className="text-xs text-muted-foreground">No saved {kind === "bookmark" ? "bookmarks" : "notes"}.</p>}
    <ul className="space-y-3">{items?.map((annotation) => <li key={annotation.id} className="rounded-md border p-2 text-xs">
      <button className="font-semibold text-primary underline focus-visible:outline-2 focus-visible:outline-ring" onClick={() => goTo(annotation.page)}>Page {annotation.page}</button>
      <p className="mt-1 whitespace-pre-wrap wrap-anywhere">{annotation.text || "Bookmark"}</p>
      {(annotation.fileId !== source.fileId || annotation.fileRevision !== source.revision) &&
        <p className="mt-1 text-muted-foreground">Saved against an earlier file. Check the page after navigating.</p>}
      <div className="mt-2 flex gap-1">
        <Button size="xs" variant="outline" onClick={() => setEditing({ kind, annotation })}>Edit</Button>
        <Button size="xs" variant="ghost" onClick={() => setRemoving({ kind, annotation })}>Delete</Button>
      </div>
    </li>)}</ul>
    {editing && <AnnotationEditor {...editing} source={source} page={page} close={() => setEditing(null)}
      saved={() => { setEditing(null); setAttempt((value) => value + 1); }} />}
    {removing && <DeleteAnnotation {...removing} source={source} close={() => setRemoving(null)}
      saved={() => { setRemoving(null); setAttempt((value) => value + 1); }} />}
  </div>;
}
