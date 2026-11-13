import { useCallback, useState } from "react";
import { Link } from "react-router";
import { invoke } from "@tauri-apps/api/core";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { Button } from "@/shared/ui/button";
import { Skeleton } from "@/shared/ui/skeleton";
import { QueryError } from "./query-feedback";

type Analytics = {
  completed: number; reading: number; pages: number; activeMs: number; sessions: number;
  activity: { day: string; sessions: number; pages: number; activeMs: number }[];
  history: { id: number; bookId: number; title: string; startedAt: number; pages: number; activeMs: number }[];
  page: number; hasMore: boolean;
};
function duration(ms: number) {
  const minutes = Math.floor(ms / 60000);
  return minutes >= 60 ? String(Math.floor(minutes / 60)) + " h " + String(minutes % 60) + " min" : String(minutes) + " min";
}
export function ReadingAnalytics() {
  const [page, setPage] = useState(1);
  const query = useCallback(() => invoke<Analytics>("reading_analytics", { page }), [page]);
  const { data, error, retry } = useMetadataQuery(query);
  if (error) return <section className="mt-5"><QueryError retry={retry} /></section>;
  if (!data) return <Skeleton className="mt-5 h-48" role="status" aria-label="Loading reading analytics" />;
  const maxPages = Math.max(1, ...data.activity.map(day => day.pages));
  return <section className="mt-5 space-y-5 rounded-xl border bg-card p-5" aria-label="Reading analytics">
    <div>
      <h2 className="text-lg font-semibold">Reading analytics</h2>
      <p className="mt-1 text-sm text-muted-foreground">Saved reader sessions and manual reading statuses. Pages count distinct pages viewed within each session; revisits in another session count again. Time counts active reader use.</p>
    </div>
    <dl className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
      {[
        ["Books completed", data.completed.toLocaleString()], ["Currently reading", data.reading.toLocaleString()],
        ["Pages viewed", data.pages.toLocaleString()], ["Active reading", duration(data.activeMs)], ["Sessions", data.sessions.toLocaleString()],
      ].map(([label, value]) => <div key={label}><dt className="text-xs text-muted-foreground">{label}</dt><dd className="mt-1 text-xl font-semibold">{value}</dd></div>)}
    </dl>
    <div>
      <h3 className="font-medium">Activity · last 30 days</h3>
      <p className="mb-3 text-xs text-muted-foreground">Grouped by the session start date in local time. Only days with saved sessions appear.</p>
      {data.activity.length ? <ul className="max-h-64 space-y-2 overflow-y-auto">{data.activity.map(day => <li key={day.day}>
        <div className="flex flex-wrap justify-between gap-2 text-xs"><span>{day.day}</span><span>{day.pages} pages · {duration(day.activeMs)} · {day.sessions} sessions</span></div>
        <progress className="h-2 w-full accent-primary" aria-label={"Pages viewed on " + day.day} max={maxPages} value={day.pages} />
      </li>)}</ul> : <p className="text-sm text-muted-foreground">No reading activity in the last 30 days.</p>}
    </div>
    <div>
      <h3 className="mb-3 font-medium">Reading history</h3>
      {data.history.length ? <ul className="divide-y">{data.history.map(session => <li key={session.id} className="flex flex-wrap items-center justify-between gap-2 py-2 text-sm">
        <div className="min-w-0"><Link className="text-primary underline wrap-anywhere" to={"/library/" + session.bookId}>{session.title}</Link>
          <p className="text-xs text-muted-foreground">{new Date(session.startedAt).toLocaleString()}</p></div>
        <span className="text-xs text-muted-foreground">{session.pages} pages · {duration(session.activeMs)}</span>
      </li>)}</ul> : <p className="text-sm text-muted-foreground">Open a PDF in the reader to start your history.</p>}
      <div className="mt-3 flex items-center gap-3">
        <Button variant="outline" size="sm" disabled={data.page <= 1} onClick={() => setPage(value => value - 1)}>Previous sessions</Button>
        <span className="text-xs" aria-live="polite">History page {data.page}</span>
        <Button variant="outline" size="sm" disabled={!data.hasMore} onClick={() => setPage(value => value + 1)}>Next sessions</Button>
      </div>
    </div>
  </section>;
}
