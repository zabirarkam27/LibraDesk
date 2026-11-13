import { ChevronLeft, ChevronRight, LayoutGrid, LibraryBig, List, Rows3 } from "lucide-react";
import { useCallback, useMemo, useRef, useState } from "react";
import { Link, useSearchParams } from "react-router";
import { desktop } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import type { CatalogView } from "./book-card";
import { categoryPath, getOrganization, queryCatalog, readingStatuses, type CatalogQuery } from "./library-client";
import { fieldClass } from "./query-feedback";
import { VirtualBooks } from "./virtual-books";
import { useWorkspaceSettings } from "@/features/settings/workspace-store";

const views = [
  { id: "grid", label: "Grid", icon: LayoutGrid },
  { id: "list", label: "List", icon: List },
  { id: "compact", label: "Compact", icon: Rows3 },
] as const;
const sorts = [
  ["added", "Date added"], ["title", "Title"], ["author", "Author"], ["size", "File size"],
  ["year", "Publication year"], ["progress", "Reading progress"], ["relevance", "Search relevance"], ["collection", "Collection order"],
] as const;

function EmptyCatalog({ filtered }: { filtered: boolean }) {
  return (
    <section className="grid min-h-60 place-items-center rounded-xl border bg-card px-6 text-center">
      <div>
        <LibraryBig className="mx-auto mb-4 size-7 text-muted-foreground" aria-hidden="true" />
        <h2 className="font-display text-2xl">{!desktop ? "Desktop library" : filtered ? "No matching books" : "Your library is empty"}</h2>
        <p className="mt-2 text-sm text-muted-foreground">{!desktop ? "Open the Windows app to see your library." : filtered
          ? "Try a different search or clear your filters." : "Import PDFs from a local folder to see them here."}</p>
        {desktop && !filtered && <Button asChild className="mt-4"><Link to="/import">Import PDFs</Link></Button>}
      </div>
    </section>
  );
}
function parseQuery(key: string): CatalogQuery {
  const params = new URLSearchParams(key);
  const id = (name: string) => {
    const value = Number(params.get(name));
    return Number.isSafeInteger(value) && value > 0 ? value : null;
  };
  const sort = sorts.find(([value]) => value === params.get("sort"))?.[0] ?? (params.get("q") ? "relevance" : "added");
  const status = readingStatuses.find(({ value }) => value === params.get("status"))?.value;
  return {
    text: params.get("q") ?? "", page: Math.min(1_000_000, Math.max(0, Math.floor(Number(params.get("page")) || 0))),
    sort, direction: params.get("direction") === "asc" || sort === "relevance" && !params.has("direction") ? "asc" : "desc",
    categoryId: id("categoryId"), tagId: id("tagId"), collectionId: id("collectionId"),
    language: params.get("language") || null, readingStatus: status ?? null, favoritesOnly: params.get("favorites") === "1",
  };
}

function DesktopCatalog() {
  const section = useRef<HTMLElement>(null);
  const defaultView=useWorkspaceSettings(state=>state.snapshot.workspace.libraryView);
  const [selectedView, setView] = useState<CatalogView|null>(null);
  const view=selectedView??defaultView;
  const [params, setParams] = useSearchParams();
  const key = params.toString();
  const input = useMemo(() => parseQuery(key), [key]);
  const query = useCallback(async () => {
    const [result, organization] = await Promise.all([queryCatalog(input), getOrganization()]);
    return { key, result, organization };
  }, [input, key]);
  const { data, error, retry } = useMetadataQuery(query);
  const current = data?.key === key ? data : null;
  const result = current?.result;
  const filtered = Boolean(input.text || input.categoryId || input.tagId || input.collectionId || input.language || input.readingStatus || input.favoritesOnly);
  const resetScroll = () => section.current?.closest("main")?.scrollTo({ top: 0 });

  function filter(name: string, value: string) {
    const next = new URLSearchParams(params);
    if (value) next.set(name, value); else next.delete(name);
    next.delete("page");
    setParams(next);
    resetScroll();
  }
  function goToPage(page: number) {
    const next = new URLSearchParams(params);
    next.set("page", String(page));
    setParams(next);
    resetScroll();
  }
  return (
    <section ref={section} aria-label="All books catalog">
      <form key={input.text} className="mb-4 flex gap-2" onSubmit={(event) => {
        event.preventDefault();
        filter("q", String(new FormData(event.currentTarget).get("search") ?? "").trim());
      }}>
        <Input name="search" aria-label="Search books" placeholder="Title, author, or tag · বাংলা / English" maxLength={256} defaultValue={input.text} />
        <Button>Search</Button>
      </form>
      <div className="mb-4 grid gap-3 rounded-lg border bg-card p-3 sm:grid-cols-2 lg:grid-cols-4">
        {([
          ["Category", "categoryId", current?.organization.categories],
          ["Tag", "tagId", current?.organization.tags],
          ["Collection", "collectionId", current?.organization.collections],
        ] as const).map(([label, name, groups]) => <label key={name} className="grid gap-1 text-xs text-muted-foreground">{label}
          <select className={fieldClass} value={params.get(name) ?? ""} onChange={(event) => filter(name, event.target.value)}>
            <option value="">All {label === "Category" ? "categories" : label.toLowerCase() + "s"}</option>
            {groups?.map((group) => <option key={group.id} value={group.id}>{name === "categoryId" ? categoryPath(group, groups) : group.name}</option>)}
          </select>
        </label>)}
        <label className="grid gap-1 text-xs text-muted-foreground">Language
          <select className={fieldClass} value={input.language ?? ""} onChange={(event) => filter("language", event.target.value)}>
            <option value="">All languages</option>
            {current?.organization.languages.map((language) => <option key={language}>{language}</option>)}
          </select>
        </label>
        <label className="grid gap-1 text-xs text-muted-foreground">Reading status
          <select className={fieldClass} value={input.readingStatus ?? ""} onChange={(event) => filter("status", event.target.value)}>
            <option value="">All statuses</option>
            {readingStatuses.map((status) => <option key={status.value} value={status.value}>{status.label}</option>)}
          </select>
        </label>
        <label className="grid gap-1 text-xs text-muted-foreground">Sort by
          <select className={fieldClass} value={input.sort} onChange={(event) => filter("sort", event.target.value)}>
            {sorts.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
          </select>
        </label>
        <label className="grid gap-1 text-xs text-muted-foreground">Order
          <select className={fieldClass} value={input.direction} onChange={(event) => filter("direction", event.target.value)}>
            <option value="asc">Ascending</option><option value="desc">Descending</option>
          </select>
        </label>
        <div className="flex flex-wrap items-end gap-2">
          <label className="flex min-h-9 items-center gap-2 text-sm">
            <input type="checkbox" checked={input.favoritesOnly} onChange={(event) => filter("favorites", event.target.checked ? "1" : "")} />Favorites
          </label>
          {key && <Button variant="ghost" size="sm" onClick={() => { setParams({}); resetScroll(); }}>Reset</Button>}
        </div>
      </div>
      <div className="sticky top-0 z-10 mb-3 flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-card p-2">
        <div role="group" aria-label="Catalog view" className="flex flex-wrap gap-1">
          {views.map(({ id, label, icon: Icon }) => <Button key={id} size="sm" variant={view === id ? "secondary" : "ghost"}
            aria-pressed={view === id} onClick={() => { if (view !== id) { resetScroll(); setView(id); } }}>
            <Icon aria-hidden="true" />{label}
          </Button>)}
        </div>
        <nav aria-label="Catalog pages" className="flex items-center gap-1">
          <Button size="icon-sm" variant="outline" aria-label="Previous page" disabled={!result || result.page === 0}
            onClick={() => { if (result) goToPage(result.page - 1); }}><ChevronLeft aria-hidden="true" /></Button>
          <span className="min-w-16 text-center text-xs tabular-nums text-muted-foreground" aria-live="polite">Page {(result?.page ?? input.page ?? 0) + 1}</span>
          <Button size="icon-sm" variant="outline" aria-label="Next page" disabled={!result || (result.page + 1) * 50 >= result.total}
            onClick={() => { if (result) goToPage(result.page + 1); }}><ChevronRight aria-hidden="true" /></Button>
        </nav>
      </div>
      {error ? <div role="alert" className="rounded-xl border bg-card p-5">
        <p className="mb-3 text-sm text-destructive">LibraDesk could not load the local catalog.</p>
        <Button variant="outline" onClick={retry}>Try again</Button>
      </div> : !result ? <div role="status">
        <span className="sr-only">Loading books</span><Skeleton className="h-48" aria-hidden="true" />
      </div> : result.books.length === 0 ? <EmptyCatalog filtered={filtered} /> : <>
        <div className="mb-3 flex min-h-6 flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
          <p role="status">{result.total.toLocaleString()} {result.total === 1 ? "book" : "books"} · {result.page * 50 + 1}–{result.page * 50 + result.books.length}</p>
          {result.books.some((book) => book.duplicate) && <Link to="/metadata?view=duplicates" className="text-primary underline underline-offset-2">Review duplicates</Link>}
        </div>
        <VirtualBooks books={result.books} view={view} />
      </>}
    </section>
  );
}
export function LibraryPage() {
  return <>
    <PageHeading label="Library" title="All books" description="Browse your collection, search in বাংলা or English, and combine filters." />
    <Button asChild variant="outline" className="mb-4"><Link to="/organize">Organize library</Link></Button>
    {desktop ? <DesktopCatalog /> : <EmptyCatalog filtered={false} />}
  </>;
}
