import { Link } from "react-router";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { desktop } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import type { BookSummary } from "./catalog-client";
import { getDashboard } from "./library-client";
import { DesktopNotice, QueryError } from "./query-feedback";

function BookList({ title, books }: { title: string; books: BookSummary[] }) {
  return <section className="rounded-xl border bg-card p-5">
    <h2 className="mb-3 text-lg font-semibold">{title}</h2>
    {books.length ? <ul className="divide-y">{books.map((book) => <li key={book.id} className="py-2">
      <Link className="block text-sm font-medium text-primary underline underline-offset-2 wrap-anywhere" to={`/library/${book.id}`}>{book.title}</Link>
      {book.author && <p className="mt-1 truncate text-xs text-muted-foreground">{book.author}</p>}
    </li>)}</ul> : <p className="text-sm text-muted-foreground">No books to show yet.</p>}
  </section>;
}
function Dashboard() {
  const { data, error, retry } = useMetadataQuery(getDashboard);
  if (error) return <QueryError retry={retry} />;
  if (!data) return <Skeleton className="h-64" role="status" aria-label="Loading overview" />;
  return <>
    <dl className="mb-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
      {[
        ["Total books", data.totalBooks.toLocaleString()],
        ["Storage", formatBytes(data.totalStorage)],
        ["Known pages", data.totalPages.toLocaleString()],
        ["Favorites", data.favorites.toLocaleString()],
        ["Currently reading", data.reading.toLocaleString()],
      ].map(([label, value]) => <div key={label} className="rounded-lg border bg-card p-4">
        <dt className="text-xs text-muted-foreground">{label}</dt><dd className="mt-2 font-display text-2xl">{value}</dd>
      </div>)}
    </dl>
    <div className="grid gap-4 lg:grid-cols-2">
      <BookList title="Recently added" books={data.recentlyAdded} />
      <BookList title="Recently read" books={data.recentlyRead} />
      <BookList title="Favorites" books={data.favoriteBooks} />
      <BookList title="Currently reading" books={data.currentlyReading} />
    </div>
    <section className="mt-5 rounded-xl border bg-card p-5">
      <h2 className="mb-3 text-lg font-semibold">Category distribution</h2>
      {data.categories.length ? <ul className="space-y-3">{data.categories.map((category) => <li key={category.id}>
        <div className="mb-1 flex justify-between gap-3 text-sm">
          <Link className="text-primary underline wrap-anywhere" to={`/library?categoryId=${category.id}`}>{category.name}</Link>
          <span className="tabular-nums">{category.count}</span>
        </div>
        <progress className="h-2 w-full accent-primary" max={Math.max(1, data.totalBooks)} value={category.count} aria-label={category.name} />
      </li>)}</ul> : <p className="text-sm text-muted-foreground">Assign categories to see their distribution.</p>}
    </section>
  </>;
}
export function OverviewPage() {
  return <>
    <PageHeading label="Overview" title="Your reading room" description="Your books, reading activity, and library totals." />
    <div className="mb-5 flex flex-wrap gap-2">
      <Button asChild><Link to="/import">Import PDFs</Link></Button>
      <Button asChild variant="outline"><Link to="/library">Open library</Link></Button>
      <Button asChild variant="outline"><Link to="/organize">Organize</Link></Button>
    </div>
    {desktop ? <Dashboard /> : <DesktopNotice />}
  </>;
}
