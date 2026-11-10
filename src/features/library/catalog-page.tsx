import { ChevronLeft, ChevronRight, LayoutGrid, LibraryBig, List, Rows3 } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { Link } from "react-router";
import { desktop } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import type { CatalogView } from "./book-card";
import { listBooks } from "./catalog-client";
import { VirtualBooks } from "./virtual-books";

const pageSize = 50;
const views = [
  { id: "grid", label: "Grid", icon: LayoutGrid },
  { id: "list", label: "List", icon: List },
  { id: "compact", label: "Compact", icon: Rows3 },
] as const;

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
  const section = useRef<HTMLElement>(null);
  const [view, setView] = useState<CatalogView>("grid");
  const [cursors, setCursors] = useState<(number | null)[]>([null]);
  const page = cursors.length;
  const before = cursors[page - 1];
  const query = useCallback(async () => ({ before, entries: await listBooks(before) }), [before]);
  const { data, error, retry } = useMetadataQuery(query);
  const entries = data?.before === before ? data.entries : null;
  const books = entries?.slice(0, pageSize);
  const next = entries && entries.length > pageSize ? entries[pageSize - 1].id : null;
  const resetScroll = () => section.current?.closest("main")?.scrollTo({ top: 0 });

  return (
    <section ref={section} aria-label="All books catalog">
      <div className="sticky top-0 z-10 mb-3 flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-card p-2">
        <div role="group" aria-label="Catalog view" className="flex flex-wrap gap-1">
          {views.map(({ id, label, icon: Icon }) => (
            <Button
              key={id}
              size="sm"
              variant={view === id ? "secondary" : "ghost"}
              aria-pressed={view === id}
              onClick={() => {
                if (view === id) return;
                resetScroll();
                setView(id);
              }}
            >
              <Icon aria-hidden="true" />{label}
            </Button>
          ))}
        </div>
        <nav aria-label="Catalog pages" className="flex items-center gap-1">
          <Button
            size="icon-sm"
            variant="outline"
            aria-label="Previous page"
            disabled={page === 1}
            onClick={() => {
              resetScroll();
              setCursors((current) => current.slice(0, -1));
            }}
          >
            <ChevronLeft aria-hidden="true" />
          </Button>
          <span className="min-w-16 text-center text-xs tabular-nums text-muted-foreground" aria-live="polite">Page {page}</span>
          <Button
            size="icon-sm"
            variant="outline"
            aria-label="Next page"
            disabled={error || next === null}
            onClick={() => {
              if (next === null) return;
              resetScroll();
              setCursors((current) => [...current, next]);
            }}
          >
            <ChevronRight aria-hidden="true" />
          </Button>
        </nav>
      </div>
      {error ? (
        <div role="alert" className="rounded-xl border bg-card p-5">
          <p className="mb-3 text-sm text-destructive">LibraDesk could not load the local catalog.</p>
          <Button variant="outline" onClick={retry}>Try again</Button>
        </div>
      ) : !books ? (
        <div role="status">
          <span className="sr-only">Loading books</span>
          <Skeleton className="h-48" aria-hidden="true" />
        </div>
      ) : books.length === 0 ? (
        page === 1 ? <EmptyCatalog /> : (
          <p role="status" className="rounded-xl border bg-card p-5 text-sm text-muted-foreground">
            This page has no books. Use the previous page button to go back.
          </p>
        )
      ) : (
        <>
          <div className="mb-3 flex min-h-6 flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
            <p role="status">{books.length} {books.length === 1 ? "book" : "books"} on this page</p>
            {books.some((book) => book.duplicate) && (
              <Link to="/metadata?view=duplicates" className="text-primary underline underline-offset-2">Review duplicates</Link>
            )}
          </div>
          <VirtualBooks books={books} view={view} />
        </>
      )}
    </section>
  );
}

export function LibraryPage() {
  return (
    <>
      <PageHeading label="Library" title="All books" description="Browse your local collection in grid, list, or compact view." />
      {desktop ? <DesktopCatalog /> : <EmptyCatalog />}
    </>
  );
}
