import { LibraryBig } from "lucide-react";
import { useCallback, useState } from "react";
import { Link } from "react-router";
import { desktop } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { BookCover } from "@/features/metadata/book-cover";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { listBooks } from "./catalog-client";

function EmptyCatalog() {
  return (
    <section className="grid min-h-60 place-items-center rounded-xl border bg-card px-6 text-center">
      <div>
        <LibraryBig className="mx-auto mb-4 size-7 text-muted-foreground" />
        <h2 className="font-display text-2xl">{desktop ? "Your library is empty" : "Desktop library"}</h2>
        <p className="mt-2 text-sm text-muted-foreground">{desktop ? "Import PDFs from a local folder to see them here." : "Open the Windows app to see imported books."}</p>
        {desktop && <Button asChild className="mt-4"><Link to="/import">Import PDFs</Link></Button>}
      </div>
    </section>
  );
}

function DesktopCatalog() {
  const [before, setBefore] = useState<number | null>(null);
  const query = useCallback(() => listBooks(before), [before]);
  const { data: entries, error, retry } = useMetadataQuery(query);

  if (error) {
    return (
      <div role="alert" className="rounded-xl border bg-card p-5">
        <p className="mb-3 text-sm text-destructive">LibraDesk could not load the local catalog.</p>
        <Button variant="outline" onClick={retry}>Try again</Button>
      </div>
    );
  }
  if (!entries) return <Skeleton className="h-48" role="status" aria-label="Loading books" />;
  if (!entries.length && before === null) return <EmptyCatalog />;

  return (
    <section aria-label="Catalog books" className="rounded-xl border bg-card">
      <ul className="divide-y">
        {entries.slice(0, 50).map((entry) => (
          <li key={entry.id} className="flex items-start gap-3 px-5 py-4">
            <BookCover fileId={entry.fileId} revision={entry.coverRevision} />
            <div className="min-w-0 flex-1">
              <p className="break-words text-sm font-semibold">{entry.title}</p>
              {(entry.author || entry.pages) && <p className="mt-1 text-xs text-muted-foreground">
                {[entry.author, entry.pages && `${entry.pages} ${entry.pages === 1 ? "page" : "pages"}`].filter(Boolean).join(" · ")}
              </p>}
              {entry.path !== null && <p className="mt-1 break-all text-xs text-muted-foreground">{entry.path}</p>}
              {entry.duplicate && <Link to="/metadata?view=duplicates" className="mt-1 inline-block text-xs text-primary underline">Exact duplicate</Link>}
            </div>
            {entry.bytes !== null && <span className="shrink-0 text-xs text-muted-foreground">{formatBytes(entry.bytes)}</span>}
          </li>
        ))}
      </ul>
      {(before !== null || entries.length > 50) && (
        <div className="flex gap-2 border-t px-5 py-3">
          {before !== null && <Button size="sm" variant="outline" onClick={() => setBefore(null)}>Latest books</Button>}
          {entries.length > 50 && <Button size="sm" variant="outline" onClick={() => setBefore(entries[49].id)}>Older books</Button>}
        </div>
      )}
    </section>
  );
}

export function LibraryPage() {
  return (
    <>
      <PageHeading label="Library" title="All books" description="Browse the PDFs registered in your local library." />
      {desktop ? <DesktopCatalog /> : <EmptyCatalog />}
    </>
  );
}
