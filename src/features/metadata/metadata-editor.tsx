import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { saveMetadata, type Edit, type MetadataRow } from "./client";

const fields = ["title", "author", "subject", "genre"] as const;

export function MetadataEditor({ rows, onClose }: { rows: MetadataRow[]; onClose: () => void }) {
  const single = rows.length === 1 ? rows[0] : null;
  const [draft, setDraft] = useState<Record<Edit["field"], string>>({
    title: single?.title ?? "", author: single?.author ?? "", subject: single?.subject ?? "", genre: single?.genre ?? "",
  });
  const [fieldsToSave, setFieldsToSave] = useState<Edit["field"][]>([]);
  const [saving, setSaving] = useState(false);
  const edits = fieldsToSave.map((field) => ({ field, value: draft[field] }));
  const invalidTitle = fieldsToSave.includes("title") && !draft.title.trim();
  const raw = single?.raw;

  function toggle(field: Edit["field"], checked: boolean) {
    setFieldsToSave((current) => checked
      ? (current.includes(field) ? current : [...current, field])
      : current.filter((value) => value !== field));
  }

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!edits.length || invalidTitle || saving) return;
    setSaving(true);
    try {
      await saveMetadata(rows.map(({ id }) => id), edits);
      toast.success("Library metadata saved; PDF updates queued");
      onClose();
    } catch {
      toast.error("LibraDesk could not save the selected metadata.");
      setSaving(false);
    }
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open && !saving) onClose(); }}>
      <DialogContent className="max-h-[85dvh] overflow-y-auto" showCloseButton={!saving}>
        <DialogHeader>
          <DialogTitle>{single ? "Edit book metadata" : `Edit ${rows.length} books`}</DialogTitle>
          <DialogDescription>Selected fields are saved to the library and queued for atomic PDF metadata updates without a safety backup. A checked, empty author or subject clears it. Check each book's PDF metadata sync status for failures.</DialogDescription>
        </DialogHeader>
        {raw && (
          <details className="rounded-md border px-3 py-2 text-sm">
            <summary className="cursor-pointer font-medium">Original PDF metadata</summary>
            <dl className="mt-3 grid grid-cols-[6rem_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-xs">
              {([
                ["Title", raw.title], ["Author", raw.author], ["Subject", raw.subject],
                ["Creator", raw.creator], ["Producer", raw.producer], ["Created", raw.creationDateRaw], ["Pages", raw.pageCount],
              ] as const).map(([label, value]) => (
                <div key={label} className="contents">
                  <dt className="text-muted-foreground">{label}</dt>
                  <dd className="whitespace-pre-wrap wrap-anywhere">{value ?? "—"}</dd>
                </div>
              ))}
            </dl>
          </details>
        )}
        <form className="space-y-4" onSubmit={(event) => void submit(event)}>
          <fieldset disabled={saving} className="space-y-3">
            {fields.map((field) => (
              <div key={field} className="space-y-1.5">
                <label className="flex items-center gap-2 text-sm font-medium">
                  <input type="checkbox" checked={fieldsToSave.includes(field)} onChange={(event) => toggle(field, event.target.checked)} />
                  <span>Apply <span className="capitalize">{field}</span></span>
                </label>
                <Input aria-label={field} value={draft[field]} maxLength={4096} onChange={(event) => {
                  setDraft((current) => ({ ...current, [field]: event.target.value }));
                  toggle(field, true);
                }} />
              </div>
            ))}
          </fieldset>
          {invalidTitle && <p className="text-sm text-destructive">Enter a title or uncheck Apply Title.</p>}
          <DialogFooter>
            <Button type="button" variant="outline" disabled={saving} onClick={onClose}>Cancel</Button>
            <Button type="submit" disabled={saving || !edits.length || invalidTitle}>{saving ? "Saving…" : "Save metadata"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
