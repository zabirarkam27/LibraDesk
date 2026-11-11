import { useState, type FormEvent } from "react";
import { errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogFooter } from "@/shared/ui/dialog";
import { updateBook, type BookDetails, type BookInput } from "./catalog-client";
import { categoryPath, readingStatuses, type Group, type Organization } from "./library-client";
import { fieldClass } from "./query-feedback";

function Assignments({ label, groups, selected, onChange }: {
  label: string; groups: Group[]; selected: number[]; onChange: (ids: number[]) => void;
}) {
  return (
    <fieldset className="rounded-md border p-3">
      <legend className="px-1 text-sm font-medium">{label}</legend>
      {groups.length ? <div className="grid max-h-40 gap-2 overflow-y-auto">
        {groups.map((group) => <label key={group.id} className="flex items-start gap-2 text-sm">
          <input type="checkbox" className="mt-1" checked={selected.includes(group.id)} onChange={(event) =>
            onChange(event.target.checked ? [...selected, group.id] : selected.filter((id) => id !== group.id))} />
          <span className="wrap-anywhere">{label === "Categories" ? categoryPath(group, groups) : group.name}</span>
        </label>)}
      </div> : <p className="text-xs text-muted-foreground">Create {label.toLowerCase()} in Organize.</p>}
    </fieldset>
  );
}

export function BookEditor({ book, organization, close, saved }: {
  book: BookDetails; organization: Organization; close: () => void; saved: () => void;
}) {
  const [original] = useState(book);
  const [draft, setDraft] = useState<BookInput>({
    title: book.title, subject: book.subject, publisher: book.publisher, isbn: book.isbn,
    publicationYear: book.publicationYear, language: book.language,
    readingStatus: book.readingStatus, isFavorite: book.isFavorite,
    categoryIds: book.categoryIds, tagIds: book.tagIds, collectionIds: book.collectionIds,
  });
  const [authors, setAuthors] = useState(book.authors.map((author) => author.displayName).join("\n"));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (saving) return;
    setSaving(true);
    setError(null);
    try {
      await updateBook(original.id, original.revision, {
        ...draft, authorNames: authors.split("\n").map((name) => name.trim()).filter(Boolean),
      });
      saved();
    } catch (reason) { setError(errorMessage(reason)); setSaving(false); }
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open && !saving) close(); }}>
      <DialogContent className="max-h-[90dvh] overflow-y-auto sm:max-w-2xl" showCloseButton={!saving}>
        <DialogHeader><DialogTitle>Edit book</DialogTitle>
          <DialogDescription>Save metadata and organize this book in your library.</DialogDescription></DialogHeader>
        <form onSubmit={(event) => void submit(event)} className="space-y-4">
          <fieldset disabled={saving} className="grid gap-3 sm:grid-cols-2">
            {(["title", "subject", "publisher", "isbn", "language"] as const).map((field) => (
              <label key={field} className="grid gap-1 text-sm font-medium">
                <span>{field === "isbn" ? "ISBN" : field[0].toUpperCase() + field.slice(1)}</span>
                <Input required={field === "title"} maxLength={4096} value={draft[field] ?? ""}
                  onChange={(event) => setDraft((current) => ({ ...current, [field]: event.target.value }))} />
              </label>
            ))}
            <label className="grid gap-1 text-sm font-medium">Publication year
              <Input type="number" min={1} max={9999} step={1} value={draft.publicationYear ?? ""}
                onChange={(event) => setDraft((current) => ({ ...current, publicationYear: event.target.value ? Number(event.target.value) : null }))} />
            </label>
            <label className="grid gap-1 text-sm font-medium sm:col-span-2">Authors, one per line
              <textarea className={fieldClass} rows={3} maxLength={100000} value={authors} onChange={(event) => setAuthors(event.target.value)} />
            </label>
            <label className="grid gap-1 text-sm font-medium">Reading status
              <select className={fieldClass} value={draft.readingStatus} onChange={(event) =>
                setDraft((current) => ({ ...current, readingStatus: event.target.value as BookDetails["readingStatus"] }))}>
                {readingStatuses.map((status) => <option key={status.value} value={status.value}>{status.label}</option>)}
              </select>
            </label>
            <label className="flex items-center gap-2 text-sm font-medium">
              <input type="checkbox" checked={draft.isFavorite} onChange={(event) =>
                setDraft((current) => ({ ...current, isFavorite: event.target.checked }))} />Favorite
            </label>
            <Assignments label="Categories" groups={organization.categories} selected={draft.categoryIds ?? []}
              onChange={(categoryIds) => setDraft((current) => ({ ...current, categoryIds }))} />
            <Assignments label="Tags" groups={organization.tags} selected={draft.tagIds ?? []}
              onChange={(tagIds) => setDraft((current) => ({ ...current, tagIds }))} />
            <div className="sm:col-span-2"><Assignments label="Collections" groups={organization.collections} selected={draft.collectionIds ?? []}
              onChange={(collectionIds) => setDraft((current) => ({ ...current, collectionIds }))} /></div>
          </fieldset>
          {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
          <DialogFooter>
            <Button type="button" variant="outline" disabled={saving} onClick={close}>Cancel</Button>
            <Button disabled={saving || !draft.title.trim()}>{saving ? "Saving…" : "Save book"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
