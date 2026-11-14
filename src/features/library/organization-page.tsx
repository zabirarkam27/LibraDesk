import { useState, type FormEvent } from "react";
import { Link } from "react-router";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { desktop, errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/shared/ui/dialog";
import { categoryPath, deleteGroup, getOrganization, saveGroup, type Group, type GroupInput, type GroupKind } from "./library-client";
import { DesktopNotice, fieldClass, QueryError } from "./query-feedback";

function GroupEditor({ kind, group, categories, close, saved }: {
  kind: GroupKind; group: Group | null; categories: Group[]; close: () => void; saved: () => void;
}) {
  const [draft, setDraft] = useState<GroupInput>({
    id: group?.id, revision: group?.revision, name: group?.name ?? "",
    parentId: group?.parentId ?? null, description: group?.description ?? "", sortMode: group?.sortMode ?? "manual",
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (saving) return;
    setSaving(true);
    setError(null);
    try { await saveGroup(kind, draft); saved(); }
    catch (reason) { setError(errorMessage(reason)); setSaving(false); }
  }
  return (
    <Dialog open onOpenChange={(open) => { if (!open && !saving) close(); }}>
      <DialogContent>
        <DialogHeader><DialogTitle>{group ? "Edit" : "Create"} {kind}</DialogTitle>
          <DialogDescription>{kind === "collection" ? "Build a collection or a reading list and choose its order." : "Give this group a name for organizing your books."}</DialogDescription></DialogHeader>
        <form onSubmit={(event) => void submit(event)} className="space-y-4">
          <fieldset disabled={saving} className="space-y-3">
            <label className="grid gap-1 text-sm font-medium">Name
              <Input required maxLength={256} value={draft.name} onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))} />
            </label>
            {kind === "category" && <label className="grid gap-1 text-sm font-medium">Parent category
              <select className={fieldClass} value={draft.parentId ?? ""} onChange={(event) =>
                setDraft((current) => ({ ...current, parentId: event.target.value ? Number(event.target.value) : null }))}>
                <option value="">Top level</option>
                {categories.filter((category) => category.id !== group?.id).map((category) =>
                  <option key={category.id} value={category.id}>{categoryPath(category, categories)}</option>)}
              </select>
            </label>}
            {kind === "collection" && <>
              <label className="grid gap-1 text-sm font-medium">Description
                <textarea className={fieldClass} maxLength={4096} rows={3} value={draft.description ?? ""}
                  onChange={(event) => setDraft((current) => ({ ...current, description: event.target.value }))} />
              </label>
              <label className="grid gap-1 text-sm font-medium">Collection order
                <select className={fieldClass} value={draft.sortMode} onChange={(event) =>
                  setDraft((current) => ({ ...current, sortMode: event.target.value as Group["sortMode"] }))}>
                  <option value="manual">Manual reading order</option><option value="title">Title</option><option value="added">Date added</option>
                </select>
              </label>
            </>}
          </fieldset>
          {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
          <DialogFooter><Button type="button" variant="outline" disabled={saving} onClick={close}>Cancel</Button>
            <Button disabled={saving || !draft.name.trim()}>{saving ? "Saving…" : "Save"}</Button></DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function DeleteGroup({ kind, group, close, saved }: { kind: GroupKind; group: Group; close: () => void; saved: () => void }) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function remove() {
    if (saving) return;
    setSaving(true);
    try { await deleteGroup(kind, group.id, group.revision); saved(); }
    catch (reason) { setError(errorMessage(reason)); setSaving(false); }
  }
  return <Dialog open onOpenChange={(open) => { if (!open && !saving) close(); }}>
    <DialogContent>
      <DialogHeader><DialogTitle>Delete {group.name}?</DialogTitle>
        <DialogDescription>Remove this {kind} and its book assignments. Its books stay in your library.</DialogDescription></DialogHeader>
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
      <DialogFooter><Button variant="outline" disabled={saving} onClick={close}>Cancel</Button>
        <Button variant="destructive" disabled={saving} onClick={() => void remove()}>{saving ? "Deleting…" : "Delete"}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}

function Groups() {
  const { data, error, retry } = useMetadataQuery(getOrganization);
  const [kind, setKind] = useState<GroupKind>("category");
  const [editing, setEditing] = useState<{ group: Group | null } | null>(null);
  const [removing, setRemoving] = useState<Group | null>(null);
  if (error && !data) return <QueryError retry={retry} />;
  if (!data) return <Skeleton className="h-60" role="status" aria-label="Loading organization" />;
  const groups = kind === "category" ? data.categories : kind === "tag" ? data.tags : data.collections;
  const categoryNames = new Map(data.categories.map((category) => [category.id, category.name]));
  return <>
    {error && <p role="alert" className="mb-3 text-sm text-destructive">The latest organization data could not be loaded. Close any editor and reload before saving.</p>}
    <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
      <div role="group" aria-label="Organization type" className="flex flex-wrap gap-1">
        {(["category", "tag", "collection"] as const).map((value) =>
          <Button key={value} variant={kind === value ? "secondary" : "outline"} aria-pressed={kind === value}
            onClick={() => setKind(value)}>{value === "category" ? "Categories" : value === "tag" ? "Tags" : "Collections"}</Button>)}
      </div>
      <Button onClick={() => setEditing({ group: null })}>Create {kind}</Button>
    </div>
    {groups.length ? <ul className="divide-y rounded-xl border bg-card">{groups.map((group) => <li key={group.id} className="flex flex-wrap items-center justify-between gap-3 p-4">
      <div className="min-w-0 flex-1">
        {group.parentId && <p className="mb-1 text-xs text-muted-foreground">In {categoryNames.get(group.parentId)}</p>}
        <Link className="font-semibold text-primary underline underline-offset-2 wrap-anywhere"
          to={kind === "collection" ? `/collections/${group.id}` : `/library?${kind}Id=${group.id}`}>{group.name}</Link>
        {group.description && <p className="mt-1 text-sm whitespace-pre-wrap wrap-anywhere">{group.description}</p>}
        <p className="mt-1 text-xs text-muted-foreground">{group.count} {group.count === 1 ? "book" : "books"}{kind === "collection" && ` · ${group.sortMode} order`}</p>
      </div>
      <div className="flex gap-2">
        <Button variant="outline" size="sm" aria-label={`Edit ${group.name}`} onClick={() => setEditing({ group })}>Edit</Button>
        <Button variant="ghost" size="sm" aria-label={`Delete ${group.name}`} onClick={() => setRemoving(group)}>Delete</Button>
      </div>
    </li>)}</ul> : <p role="status" className="rounded-xl border bg-card p-5 text-sm text-muted-foreground">Create your first {kind} to organize books.</p>}
    {editing && <GroupEditor key={editing.group?.id ?? "new"} kind={kind} group={editing.group} categories={data.categories}
      close={() => setEditing(null)} saved={() => { setEditing(null); retry(); }} />}
    {removing && <DeleteGroup kind={kind} group={removing} close={() => setRemoving(null)}
      saved={() => { setRemoving(null); retry(); }} />}
  </>;
}
export function OrganizationPage() {
  return <><PageHeading label="Library" title="Organize" description="Manage nested categories, tags, collections, and reading lists. Name and membership changes queue metadata updates for the assigned PDFs." />
    {desktop ? <Groups /> : <DesktopNotice />}</>;
}
