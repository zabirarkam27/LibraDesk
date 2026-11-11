import { BookCover } from "@/features/metadata/book-cover";
import { formatBytes } from "@/shared/lib/file-size";
import { cn } from "@/shared/lib/utils";
import { Link } from "react-router";
import { Star } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { setBookFlags } from "./library-client";
import type { BookSummary } from "./catalog-client";

export type CatalogView = "grid" | "list" | "compact";

export function BookCard({ book, view }: { book: BookSummary; view: CatalogView }) {
  const grid = view === "grid";
  const compact = view === "compact";
  const [saving, setSaving] = useState(false);
  async function toggleFavorite() {
    if (saving) return;
    setSaving(true);
    try { await setBookFlags(book.id, book.revision, !book.isFavorite); }
    catch (reason) { toast.error(errorMessage(reason)); }
    finally { setSaving(false); }
  }
  const facts = [
    book.pages === null ? null : `${book.pages} ${book.pages === 1 ? "page" : "pages"}`,
    book.bytes === null ? null : formatBytes(book.bytes),
  ].filter(Boolean).join(" · ");

  return (
    <article className={cn(
      "relative flex h-full min-w-0 overflow-hidden rounded-lg border bg-card",
      grid ? "flex-col" : "items-center gap-3 pr-12 pl-3",
    )}>
      <Button size="icon-sm" variant="ghost" className="absolute top-1.5 right-1.5 z-1 bg-card/90"
        aria-label={`${book.isFavorite ? "Remove" : "Add"} ${book.title} ${book.isFavorite ? "from" : "to"} favorites`}
        aria-pressed={book.isFavorite} disabled={saving} onClick={() => void toggleFavorite()}>
        <Star aria-hidden="true" className={book.isFavorite ? "fill-current" : undefined} />
      </Button>
      {grid ? (
        <div className="grid h-40 shrink-0 place-items-center border-b bg-muted/60">
          <BookCover fileId={book.fileId} revision={book.coverRevision} className="h-36 w-28 shadow-sm" />
        </div>
      ) : (
        <BookCover fileId={book.fileId} revision={book.coverRevision} className={compact ? "h-9 w-7" : undefined} />
      )}
      <div className={cn("min-w-0", grid ? "px-3 py-2.5" : "flex-1")}>
        <h2 className={cn("text-sm leading-5 font-semibold", grid ? "line-clamp-2 wrap-anywhere" : "truncate")} title={book.title}>
          <Link to={`/library/${book.id}`} className="rounded-sm hover:underline focus-visible:outline-2 focus-visible:outline-ring">{book.title}</Link>
        </h2>
        {book.author && <p className="truncate text-xs leading-4 text-muted-foreground" title={book.author}>{book.author}</p>}
        {!compact && <p className="flex items-center gap-1 text-xs leading-4 text-muted-foreground">
          {book.readingStatus?.replace("_", " ")}
        </p>}
        {!compact && facts && <p className="mt-1 truncate text-xs leading-4 text-muted-foreground">{facts}</p>}
        {view === "list" && book.path && (
          <p className="truncate text-xs leading-4 text-muted-foreground" title={book.path}>{book.path}</p>
        )}
        {!compact && book.duplicate && <p className="truncate text-xs leading-4 font-medium text-primary">Exact duplicate</p>}
      </div>
      {compact && (
        <div className="hidden shrink-0 text-right text-xs leading-4 text-muted-foreground sm:block">
          {facts && <p>{facts}</p>}
          {book.duplicate && <p className="font-medium text-primary">Exact duplicate</p>}
        </div>
      )}
    </article>
  );
}
