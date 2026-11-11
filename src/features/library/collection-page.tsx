import { useCallback, useState } from "react";
import { Link, useParams } from "react-router";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { desktop, errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { editCollectionMember, getOrganization, queryCatalog } from "./library-client";
import { DesktopNotice, QueryError } from "./query-feedback";

function Collection({ id }: { id: number }) {
  const [page, setPage] = useState(0);
  const query = useCallback(async () => {
    const [organization, result] = await Promise.all([
      getOrganization(), queryCatalog({ collectionId: id, sort: "collection", direction: "asc", page }),
    ]);
    const collection = organization.collections.find((group) => group.id === id);
    if (!collection) throw new Error("Collection missing");
    return { collection, result };
  }, [id, page]);
  const { data, error, retry } = useMetadataQuery(query);
  const [busy, setBusy] = useState(false);
  const [mutationError, setMutationError] = useState<string | null>(null);
  async function action(book: number, action: "up" | "down" | "remove") {
    if (!data || busy) return;
    setBusy(true);
    setMutationError(null);
    try { await editCollectionMember(id, data.collection.revision, book, action); retry(); }
    catch (reason) { setMutationError(errorMessage(reason)); }
    finally { setBusy(false); }
  }
  if (error) return <QueryError retry={retry} />;
  if (!data) return <Skeleton className="h-60" role="status" aria-label="Loading collection" />;
  const { collection, result } = data;
  return <>
    <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
      <Button asChild variant="outline"><Link to="/organize">Manage collections</Link></Button>
      <Button asChild variant="outline"><Link to={`/library?collectionId=${id}`}>Browse collection</Link></Button>
    </div>
    <h1 className="font-display text-3xl wrap-anywhere">{collection.name}</h1>
    {collection.description && <p className="mt-2 text-sm whitespace-pre-wrap wrap-anywhere">{collection.description}</p>}
    <p className="my-3 text-xs text-muted-foreground">{result.total} books · {collection.sortMode} order. Add books through Edit book.</p>
    {mutationError && <p role="alert" className="mb-3 text-sm text-destructive">{mutationError}</p>}
    {result.books.length ? <ul className="divide-y rounded-xl border bg-card">{result.books.map((book, index) => <li key={book.id} className="flex flex-wrap items-center gap-3 p-3">
      <span className="text-xs text-muted-foreground">{result.page * 50 + index + 1}</span>
      <Link className="min-w-0 flex-1 text-sm font-medium text-primary underline wrap-anywhere" to={`/library/${book.id}`}>{book.title}</Link>
      <div className="flex gap-1">
        {collection.sortMode === "manual" && <>
          <Button variant="outline" size="sm" aria-label={`Move ${book.title} up`} disabled={busy || result.page === 0 && index === 0}
            onClick={() => void action(book.id, "up")}>Up</Button>
          <Button variant="outline" size="sm" aria-label={`Move ${book.title} down`} disabled={busy || result.page * 50 + index === result.total - 1}
            onClick={() => void action(book.id, "down")}>Down</Button>
        </>}
        <Button variant="ghost" size="sm" disabled={busy} aria-label={`Remove ${book.title}`}
          onClick={() => void action(book.id, "remove")}>Remove</Button>
      </div>
    </li>)}</ul> : <DesktopNotice />}
    <nav aria-label="Collection pages" className="mt-4 flex items-center gap-3">
      <Button variant="outline" disabled={busy || result.page === 0} onClick={() => setPage(result.page - 1)}>Previous</Button>
      <span className="text-xs text-muted-foreground">Page {result.page + 1}</span>
      <Button variant="outline" disabled={busy || (result.page + 1) * 50 >= result.total} onClick={() => setPage(result.page + 1)}>Next</Button>
    </nav>
  </>;
}
export function CollectionPage() {
  const { id } = useParams();
  const collectionId = Number(id);
  return <><PageHeading label="Library" title="Collection" description="Your books in a collection or reading list." />
    {!desktop ? <DesktopNotice /> : !Number.isSafeInteger(collectionId) || collectionId <= 0
      ? <p role="alert">This collection address is invalid.</p> : <Collection key={id} id={collectionId} />}</>;
}
