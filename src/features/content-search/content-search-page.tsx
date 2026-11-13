import { useCallback, useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { DesktopNotice } from "@/features/library/query-feedback";
import { desktop } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { resultAddress, searchPages } from "./client";

function Results({ text, page, paginate }: { text: string; page: number; paginate: (page: number) => void }) {
  const query = useCallback(() => searchPages(text, page), [text, page]);
  const { data, error, retry } = useMetadataQuery(query);
  return <section aria-label="PDF search results" className="space-y-4">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <p className="text-sm text-muted-foreground">Searches extracted pages. Refresh as background extraction progresses.</p>
      <Button variant="outline" size="sm" onClick={retry}>Refresh results</Button>
    </div>
    {error && <p role="alert" className="rounded-lg border p-4 text-sm text-destructive">PDF search could not be loaded. Try a shorter query or refresh results.</p>}
    {!data && !error && <Skeleton className="h-48" role="status" aria-label="Searching PDF contents" />}
    {data && <>
      <p className="text-xs text-muted-foreground">{data.coverage.extractedPages.toLocaleString()} extracted pages · {data.coverage.completedFiles.toLocaleString()} / {data.coverage.files.toLocaleString()} files complete · {data.coverage.failedFiles.toLocaleString()} failed</p>
      <p role="status" className="text-sm">{text.trim() ? `${data.total.toLocaleString()} matching pages` : "Enter words in বাংলা or English to search PDF contents."}</p>
      {text.trim() && data.total === 0 && <p className="rounded-xl border bg-card p-5 text-sm text-muted-foreground">
        No matching extracted pages. Try different words or check text extraction in Book details. Image-only and password-protected PDFs may have no searchable text.
      </p>}
      <ol className="space-y-3">{data.hits.map((hit) => <li key={hit.id} className="rounded-xl border bg-card p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <Link className="font-semibold text-primary underline underline-offset-2 wrap-anywhere" to={resultAddress(hit)}>{hit.title} · Page {hit.pageNumber}</Link>
            {hit.author && <p className="mt-1 text-xs text-muted-foreground wrap-anywhere">{hit.author}</p>}
          </div>
          <Link className="text-xs underline underline-offset-2" to={`/library/${hit.bookId}`}>Book details</Link>
        </div>
        <p className="mt-3 text-sm leading-6 wrap-anywhere">{hit.snippet}</p>
        <p className="mt-3 text-xs text-muted-foreground wrap-anywhere">{hit.path}</p>
      </li>)}</ol>
      {data.total > 50 && <nav aria-label="PDF search pages" className="flex items-center justify-between gap-3">
        <Button variant="outline" disabled={data.page === 0} onClick={() => paginate(data.page - 1)}>Previous results</Button>
        <span className="text-xs">Page {data.page + 1} / {Math.ceil(data.total / 50)}</span>
        <Button variant="outline" disabled={(data.page + 1) * 50 >= data.total} onClick={() => paginate(data.page + 1)}>Next results</Button>
      </nav>}
    </>}
  </section>;
}

export function ContentSearchPage() {
  const [params, setParams] = useSearchParams();
  const text = params.get("q") ?? "";
  const [draft, setDraft] = useState(text);
  useEffect(() => { setDraft(text); }, [text]);
  const raw = Number(params.get("page"));
  const page = Number.isSafeInteger(raw) && raw >= 0 && raw <= 1_000_000 ? raw : 0;
  return <>
    <PageHeading label="Search" title="PDF contents" description="Find words inside your PDFs and open the matching page." />
    <form className="mb-5 flex gap-2" onSubmit={(event) => {
      event.preventDefault();
      setParams({ q: String(new FormData(event.currentTarget).get("text") ?? "").trim() });
    }}>
      <Input name="text" aria-label="Search PDF contents" value={draft} onChange={(event) => setDraft(event.target.value)} maxLength={256} placeholder="বাংলা or English words" />
      <Button>Search contents</Button>
    </form>
    {desktop ? <Results key={params.toString()} text={text} page={page} paginate={(next) => setParams({ q: text, page: String(next) })} /> : <DesktopNotice />}
  </>;
}
