import { BookCover } from "@/features/metadata/book-cover";
import { formatBytes } from "@/shared/lib/file-size";
import { cn } from "@/shared/lib/utils";
import type { BookSummary } from "./catalog-client";

export type CatalogView = "grid" | "list" | "compact";

export function BookCard({ book, view }: { book: BookSummary; view: CatalogView }) {
  const grid = view === "grid";
  const compact = view === "compact";
  const facts = [
    book.pages === null ? null : `${book.pages} ${book.pages === 1 ? "page" : "pages"}`,
    book.bytes === null ? null : formatBytes(book.bytes),
  ].filter(Boolean).join(" · ");

  return (
    <article className={cn(
      "flex h-full min-w-0 overflow-hidden rounded-lg border bg-card",
      grid ? "flex-col" : "items-center gap-3 px-3",
    )}>
      {grid ? (
        <div className="grid h-40 shrink-0 place-items-center border-b bg-muted/60">
          <BookCover fileId={book.fileId} revision={book.coverRevision} className="h-36 w-28 shadow-sm" />
        </div>
      ) : (
        <BookCover fileId={book.fileId} revision={book.coverRevision} className={compact ? "h-9 w-7" : undefined} />
      )}
      <div className={cn("min-w-0", grid ? "px-3 py-2.5" : "flex-1")}>
        <h2 className={cn("text-sm leading-5 font-semibold", grid ? "line-clamp-2 wrap-anywhere" : "truncate")} title={book.title}>
          {book.title}
        </h2>
        {book.author && <p className="truncate text-xs leading-4 text-muted-foreground" title={book.author}>{book.author}</p>}
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
