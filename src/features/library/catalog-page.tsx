import { invoke } from "@tauri-apps/api/core";
import { Book, LibraryBig } from "lucide-react";
import { useEffect, useState } from "react";
import { Link } from "react-router";
import { desktop } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";

type Entry = { id: number; title: string; path: string; bytes: number };

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
  const [attempt, setAttempt] = useState(0);
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let active = true;
    void invoke<Entry[]>("list_imported_books", { before }).then(
      (page) => { if (active) setEntries(page); },
      () => { if (active) setError(true); },
    );
    return () => { active = false; };
  }, [attempt, before]);

  function page(cursor: number | null) {
    setBefore(cursor);
    setEntries(null);
  }

  if (error) {
    return (
      <div role="alert" className="rounded-xl border bg-card p-5">
        <p className="mb-3 text-sm text-destructive">LibraDesk could not load the local catalog.</p>
        <Button variant="outline" onClick={() => { setError(false); setAttempt((value) => value + 1); }}>Try again</Button>
      </div>
    );
  }
  if (!entries) return <Skeleton className="h-48" role="status" aria-label="Loading books" />;
  if (!entries.length && before === null) return <EmptyCatalog />;

  return (
    <section aria-label="Imported books" className="rounded-xl border bg-card">
      <ul className="divide-y">
        {entries.slice(0, 50).map((entry) => (
          <li key={entry.id} className="flex items-start gap-3 px-5 py-4">
            <Book className="mt-1 size-4 shrink-0 text-muted-foreground" />
            <div className="min-w-0 flex-1">
              <p className="break-words text-sm font-semibold">{entry.title}</p>
              <p className="mt-1 break-all text-xs text-muted-foreground">{entry.path}</p>
            </div>
            <span className="shrink-0 text-xs text-muted-foreground">{formatBytes(entry.bytes)}</span>
          </li>
        ))}
      </ul>
      {(before !== null || entries.length > 50) && (
        <div className="flex gap-2 border-t px-5 py-3">
          {before !== null && <Button size="sm" variant="outline" onClick={() => page(null)}>Latest books</Button>}
          {entries.length > 50 && <Button size="sm" variant="outline" onClick={() => page(entries[49].id)}>Older books</Button>}
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
