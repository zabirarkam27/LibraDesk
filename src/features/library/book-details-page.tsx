import { useCallback, useState } from "react";
import { Link, useParams } from "react-router";
import { BookCover } from "@/features/metadata/book-cover";
import { ExtractionStatus } from "@/features/extraction/extraction-status";
import { RelinkButton } from "@/features/recovery/relink-button";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { desktop } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { getBook } from "./catalog-client";
import { BookEditor } from "./book-editor";
import { getOrganization, readingStatuses } from "./library-client";
import { DesktopNotice, QueryError } from "./query-feedback";
import { ThumbnailEditor } from "./thumbnail-editor";
import { PdfMetadataStatus } from "./pdf-metadata-status";

function Details({ id }: { id: number }) {
  const query = useCallback(async () => {
    const [book, organization] = await Promise.all([getBook(id), getOrganization()]);
    return { book, organization };
  }, [id]);
  const { data, error, retry } = useMetadataQuery(query);
  const [editing, setEditing] = useState(false);
  if (error && !data) return <QueryError retry={retry} />;
  if (!data) return <Skeleton className="h-80" role="status" aria-label="Loading book" />;
  const { book, organization } = data;
  const primary = book.files.find((file) => file.isPrimary) ?? book.files[0];
  const fields = [
    ["Authors", book.authors.map((author) => author.displayName).join(", ") || null],
    ["Subject", book.subject], ["Genre", book.genre], ["Publisher", book.publisher], ["ISBN", book.isbn],
    ["Publication year", book.publicationYear], ["Language", book.language],
    ["Reading status", readingStatuses.find((status) => status.value === book.readingStatus)?.label],
    ["Favorite", book.isFavorite ? "Yes" : "No"],
  ] as const;
  return (
    <>
      {error && <p role="alert" className="mb-3 text-sm text-destructive">The latest book data could not be loaded. Close the editor and reload before saving.</p>}
      <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <Button asChild variant="outline"><Link to="/library">Back to library</Link></Button>
        <div className="flex gap-2">
          {primary && <Button asChild><Link to={`/reader?book=${book.id}&file=${primary.id}`}>Read</Link></Button>}
          <Button variant="outline" onClick={() => setEditing(true)}>Edit book</Button>
        </div>
      </div>
      <section className="grid gap-6 rounded-xl border bg-card p-5 sm:grid-cols-[10rem_minmax(0,1fr)]">
        <div className="space-y-3"><BookCover fileId={primary?.id ?? null} revision={primary?.coverRevision ?? null} className="h-52 w-40" /><ThumbnailEditor book={book} saved={retry} /></div>
        <div className="min-w-0">
          <h1 className="font-display text-3xl wrap-anywhere">{book.title}</h1>
          <dl className="mt-5 grid grid-cols-[8rem_minmax(0,1fr)] gap-3 text-sm">
            {fields.map(([label, value]) => <div key={label} className="contents">
              <dt className="text-muted-foreground">{label}</dt><dd className="wrap-anywhere">{value ?? "Unknown"}</dd>
            </div>)}
          </dl>
        </div>
      </section>
      <section className="mt-5 space-y-3 rounded-xl border bg-card p-5" aria-label="Book organization">
        {([
          ["Categories", organization.categories, book.categoryIds, "categoryId"],
          ["Tags", organization.tags, book.tagIds, "tagId"],
          ["Collections", organization.collections, book.collectionIds, "collectionId"],
        ] as const).map(([label, groups, selected, filter]) => <div key={label}>
          <h2 className="mb-1 text-sm font-semibold">{label}</h2>
          <div className="flex flex-wrap gap-2 text-sm">
            {selected.length ? groups.filter((group) => selected.includes(group.id)).map((group) =>
              <Link key={group.id} className="rounded-md border px-2 py-1 text-primary underline underline-offset-2"
                to={filter === "collectionId" ? `/collections/${group.id}` : `/library?${filter}=${group.id}`}>{group.name}</Link>,
            ) : <span className="text-muted-foreground">None assigned</span>}
          </div>
        </div>)}
      </section>
      <section className="mt-5 rounded-xl border bg-card p-5" aria-labelledby="files-heading">
        <h2 id="files-heading" className="mb-3 text-lg font-semibold">Files</h2>
        {book.files.length ? <ul className="space-y-4">{book.files.map((file) => <li key={file.id} className="text-sm">
          <p className="whitespace-pre-wrap wrap-anywhere">{file.path}</p>
          <p className="mt-1 text-xs text-muted-foreground">
            {file.pages === null ? "Unknown pages" : `${file.pages} pages`} · {formatBytes(file.bytes)} · {file.availability}
          </p>
          <ExtractionStatus file={file.id} revision={file.revision} available={file.availability === "available"} />
          <RelinkButton file={file.id} />
        </li>)}</ul> : <p className="text-sm text-muted-foreground">No file is attached to this book.</p>}
      </section>
      <PdfMetadataStatus book={book.id} files={book.files} />
      {editing && <BookEditor book={book} organization={organization} close={() => { setEditing(false); retry(); }}
        saved={() => { setEditing(false); retry(); }} />}
    </>
  );
}

export function BookDetailsPage() {
  const { id } = useParams();
  const bookId = Number(id);
  return (
    <>
      <PageHeading label="Library" title="Book details" description="Metadata, files, and organization for this book." />
      {!desktop ? <DesktopNotice /> : !Number.isSafeInteger(bookId) || bookId <= 0
        ? <p role="alert">This book address is invalid.</p> : <Details key={id} id={bookId} />}
    </>
  );
}
