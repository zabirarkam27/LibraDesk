import { FileCheck2, Files } from "lucide-react";
import { useCallback, useState } from "react";
import { useSearchParams } from "react-router";
import { desktop } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { listDuplicates, metadataPage, type MetadataRow } from "./client";
import { DuplicateReview } from "@/features/recovery/duplicate-review";
import { MetadataEditor } from "./metadata-editor";
import { useMetadataQuery } from "./use-metadata-query";

function Queue() {
  const [missingOnly, setMissingOnly] = useState(true);
  const [before, setBefore] = useState<number | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [editing, setEditing] = useState<MetadataRow[] | null>(null);
  const query = useCallback(() => metadataPage(before, missingOnly), [before, missingOnly]);
  const { data, error, retry } = useMetadataQuery(query);
  const rows = data?.slice(0, 50) ?? [];
  const selectedRows = rows.filter(({ id }) => selected.includes(id));

  function changePage(value: number | null) {
    setSelected([]);
    setBefore(value);
  }

  return (
    <section aria-label="Metadata review queue" className="rounded-xl border bg-card">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b px-5 py-4">
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={missingOnly} onChange={(event) => {
            setMissingOnly(event.target.checked);
            changePage(null);
          }} />
          Show only books needing review
        </label>
        <Button size="sm" disabled={!selectedRows.length} onClick={() => setEditing(selectedRows)}>
          Edit selected{selectedRows.length ? ` (${selectedRows.length})` : ""}
        </Button>
      </div>
      {error && <div role="alert" className="p-5 text-sm">Could not load metadata. <Button size="sm" variant="outline" onClick={retry}>Retry</Button></div>}
      {!data && !error && <Skeleton className="m-5 h-24" aria-label="Loading metadata" role="status" />}
      {data && !rows.length && <p className="p-5 text-sm text-muted-foreground">{missingOnly ? "No processed books currently need title or author review." : "No books on this page."}</p>}
      {rows.length > 0 && (
        <ul className="divide-y">
          {rows.map((row) => (
            <li key={row.id} className="flex items-start gap-3 px-5 py-4">
              <input type="checkbox" className="mt-1" aria-label={`Select ${row.title}`} checked={selected.includes(row.id)} onChange={(event) => {
                const checked = event.target.checked;
                setSelected((current) => checked ? [...current, row.id] : current.filter((id) => id !== row.id));
              }} />
              <div className="min-w-0 flex-1">
                <p className="break-words text-sm font-semibold">{row.title}</p>
                <p className="mt-1 break-all text-xs text-muted-foreground">{row.author ? `${row.author} · ` : ""}{row.path}</p>
                <p className="mt-1 text-xs text-muted-foreground">{row.pending ? "Processing PDF…" : row.error ? "Original metadata unavailable" : row.needsReview ? "Title or author needs review" : "Title and author reviewed"}</p>
              </div>
              <Button size="sm" variant="outline" onClick={() => setEditing([row])}>Edit</Button>
            </li>
          ))}
        </ul>
      )}
      {(before !== null || (data?.length ?? 0) > 50) && (
        <div className="flex gap-2 border-t px-5 py-3">
          {before !== null && <Button size="sm" variant="outline" onClick={() => changePage(null)}>Latest books</Button>}
          {data && data.length > 50 && <Button size="sm" variant="outline" onClick={() => changePage(rows[49].id)}>Older books</Button>}
        </div>
      )}
      {editing && <MetadataEditor rows={editing} onClose={() => setEditing(null)} />}
    </section>
  );
}

function Duplicates() {
  const [after, setAfter] = useState<string | null>(null);
  const query = useCallback(() => listDuplicates(after), [after]);
  const { data, error, retry } = useMetadataQuery(query);
  return (
    <section aria-label="Exact duplicates" className="rounded-xl border bg-card">
      <p className="border-b px-5 py-4 text-sm text-muted-foreground">Compare SHA-256 duplicate groups, choose a preferred copy, or remove individual library records after confirmation. Original PDFs are never deleted.</p>
      {error && <div role="alert" className="p-5 text-sm">Could not load duplicate groups. <Button size="sm" variant="outline" onClick={retry}>Retry</Button></div>}
      {!data && !error && <Skeleton className="m-5 h-24" aria-label="Loading duplicate groups" role="status" />}
      {data?.length === 0 && <p className="p-5 text-sm text-muted-foreground">No exact duplicate groups on this page.</p>}
      {data && data.length > 0 && <ul className="divide-y">{data.slice(0, 20).map((group) => <DuplicateReview key={group.hash} group={group} />)}</ul>}
      {(after !== null || (data?.length ?? 0) > 20) && (
        <div className="flex gap-2 border-t px-5 py-3">
          {after !== null && <Button size="sm" variant="outline" onClick={() => setAfter(null)}>First page</Button>}
          {data && data.length > 20 && <Button size="sm" variant="outline" onClick={() => setAfter(data[19].hash)}>More groups</Button>}
        </div>
      )}
    </section>
  );
}

export function MetadataPage() {
  const [params, setParams] = useSearchParams();
  const duplicates = params.get("view") === "duplicates";
  return (
    <>
      <PageHeading label="Workspace" title="Metadata review" description="Check missing book details and review identical PDF files." />
      {desktop ? (
        <>
          <div className="mb-4 flex gap-2" aria-label="Metadata view">
            <Button variant={duplicates ? "outline" : "default"} aria-pressed={!duplicates} onClick={() => setParams({})}><FileCheck2 /> Review queue</Button>
            <Button variant={duplicates ? "default" : "outline"} aria-pressed={duplicates} onClick={() => setParams({ view: "duplicates" })}><Files /> Exact duplicates</Button>
          </div>
          {duplicates ? <Duplicates /> : <Queue />}
        </>
      ) : <p className="rounded-xl border bg-card p-5 text-sm text-muted-foreground">Open LibraDesk for Windows to review the local catalog.</p>}
    </>
  );
}
