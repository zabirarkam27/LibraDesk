import { FolderOpen, FolderPlus, History, ScanLine } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { desktop } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/shared/ui/dialog";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { controlScan, pickScanRoots, scanError, startScan, updateScanRoot, type ScanJob, type ScanRoot, type ScanStatus } from "./scan-client";
import { useScanSnapshot } from "./use-scan-snapshot";

const labels: Record<ScanStatus, string> = {
  queued: "Queued", running: "Scanning", paused: "Paused", awaiting_confirmation: "Ready to review",
  importing: "Importing", completed: "Completed", cancelled: "Cancelled", failed: "Failed",
};
const terminal = (job: ScanJob) => job.status === "completed" || job.status === "cancelled" || job.status === "failed";
type Run = (action: () => Promise<unknown>) => Promise<boolean>;
const timestamp = (value: number | null) => value === null ? "—" : new Date(value).toLocaleString();

function ConfirmAction({ trigger, title, description, confirm, disabled, onConfirm }: {
  trigger: string;
  title: string;
  description: string;
  confirm: string;
  disabled: boolean;
  onConfirm: () => Promise<boolean>;
}) {
  const [open, setOpen] = useState(false);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild><Button size="sm" variant="outline" disabled={disabled}>{trigger}</Button></DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose asChild><Button variant="outline">Go back</Button></DialogClose>
          <Button disabled={disabled} onClick={async () => { if (await onConfirm()) setOpen(false); }}>{confirm}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RootRow({ root, active, busy, run }: { root: ScanRoot; active: ScanJob | undefined; busy: boolean; run: Run }) {
  const importing = active?.status === "importing";
  return (
    <li className="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
      <div className="flex min-w-0 items-start gap-3">
        <FolderOpen className="mt-1 size-4 shrink-0 text-muted-foreground" />
        <div className="min-w-0">
          <p className="break-all text-sm font-medium">{root.path}</p>
          <p className="text-xs text-muted-foreground">{root.enabled ? "Enabled" : "Disabled"}</p>
        </div>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button size="sm" disabled={busy || !root.enabled || Boolean(active)} onClick={() => void run(() => startScan(root.id))}><ScanLine /> Scan folder</Button>
        <Button size="sm" variant="ghost" disabled={busy || importing} onClick={() => void run(() => updateScanRoot(root.id, !root.enabled))}>
          {root.enabled ? "Disable" : "Enable"}
        </Button>
        <ConfirmAction
          trigger="Remove"
          title="Remove scan folder?"
          description="The folder will leave the scan list. Imported books, scan history, and the original files stay available."
          confirm="Remove folder"
          disabled={busy || importing}
          onConfirm={() => run(() => updateScanRoot(root.id, null))}
        />
      </div>
    </li>
  );
}

function ScanMetrics({ job }: { job: ScanJob }) {
  const metrics = [
    ["PDFs found", job.found], ["Known size", formatBytes(job.bytes)], ["New files", job.newFiles],
    ["Existing paths", job.existing], ["Possible duplicates", job.duplicates], ["Invalid PDFs", job.invalid],
    ["Unreadable PDFs", job.inaccessible], ["Unreadable folders", job.directoryErrors],
  ];
  return (
    <dl className="grid grid-cols-2 gap-2 md:grid-cols-4">
      {metrics.map(([label, value]) => (
        <div key={label} className="rounded-md border bg-background px-3 py-2">
          <dt className="text-xs text-muted-foreground">{label}</dt>
          <dd className="text-lg font-semibold tabular-nums">{value.toLocaleString()}</dd>
        </div>
      ))}
    </dl>
  );
}

function ActiveScan({ job, busy, run }: { job: ScanJob; busy: boolean; run: Run }) {
  const review = job.status === "awaiting_confirmation";
  const importing = job.status === "importing";
  const eligible = job.newFiles + job.duplicates;
  return (
    <section aria-label={review ? "Import preview" : "Scan progress"} className="space-y-4 rounded-xl border bg-card p-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h2 className="font-display text-2xl">{review ? "Import preview" : labels[job.status]}</h2>
          <p className="mt-1 break-all text-sm text-muted-foreground">{job.path}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          {(job.status === "queued" || job.status === "running") && (
            <Button size="sm" variant="outline" disabled={busy} onClick={() => void run(() => controlScan(job.id, "pause"))}>Pause</Button>
          )}
          {job.status === "paused" && (
            <Button size="sm" disabled={busy} onClick={() => void run(() => controlScan(job.id, "resume"))}>Resume</Button>
          )}
          {!importing && (
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run(() => controlScan(job.id, "cancel"))}>Cancel scan</Button>
          )}
        </div>
      </div>
      {review ? (
        <>
          <ScanMetrics job={job} />
          <p className="text-sm text-muted-foreground">Possible duplicates share a file size. Each eligible path is imported as a separate entry. The known size excludes files that could not be read.</p>
          {eligible > 0 ? (
            <ConfirmAction
              trigger="Review import"
              title="Confirm PDF import"
              description={`Add up to ${eligible.toLocaleString()} PDFs to your local catalog. Their files stay in their original folders.`}
              confirm="Import PDFs"
              disabled={busy}
              onConfirm={() => run(() => controlScan(job.id, "confirm"))}
            />
          ) : (
            <Button disabled={busy} onClick={() => void run(() => controlScan(job.id, "confirm"))}>Finish review</Button>
          )}
        </>
      ) : (
        <p role="status" className="text-sm tabular-nums text-muted-foreground">
          {importing ? `${job.imported} imported · ${job.skipped} already in library · ${job.failed} failed`
            : `${job.found} PDFs found · ${formatBytes(job.bytes)} known size · ${job.directoryErrors} unreadable folders`}
        </p>
      )}
    </section>
  );
}

function ScanHistory({ jobs, before, onPage }: { jobs: ScanJob[]; before: number | null; onPage: (before: number | null) => void }) {
  return (
    <section className="rounded-xl border bg-card" aria-labelledby="scan-history">
      <div className="flex items-center gap-2 border-b px-5 py-4">
        <History className="size-4 text-muted-foreground" />
        <h2 id="scan-history" className="font-display text-2xl">Scan history</h2>
      </div>
      {jobs.length === 0 ? <p className="p-5 text-sm text-muted-foreground">No finished scans on this page.</p> : (
        <ul className="divide-y">
          {jobs.slice(0, 30).map((job) => (
            <li key={job.id} className="space-y-2 px-5 py-4">
              <div className="flex flex-wrap justify-between gap-2">
                <p className="break-all text-sm font-medium">{job.path}</p>
                <span className="rounded-md bg-accent px-2 py-0.5 text-xs text-accent-foreground">{labels[job.status]}</span>
              </div>
              <p className="text-xs text-muted-foreground">Started {timestamp(job.startedAt)} · Finished {timestamp(job.finishedAt)}</p>
              <p className="text-sm tabular-nums text-muted-foreground">
                {job.found} found · {job.imported} imported · {job.failed + job.invalid + job.inaccessible} failed
                {job.directoryErrors ? ` · ${job.directoryErrors} unreadable folders` : ""}
                {job.existing + job.skipped ? ` · ${job.existing + job.skipped} already in library` : ""}
              </p>
              {job.error && <p className="text-sm text-destructive">{job.error === "root_unavailable"
                ? "The scan folder is missing or inaccessible. Reconnect the drive or check folder access."
                : "The scan could not update the database. Start a new scan to retry."}</p>}
              {job.failed > 0 && <p className="text-xs text-muted-foreground">Some PDFs changed or could not be read after preview. Scan the folder again to review them.</p>}
            </li>
          ))}
        </ul>
      )}
      {(before !== null || jobs.length > 30) && (
        <div className="flex gap-2 border-t px-5 py-3">
          {before !== null && <Button size="sm" variant="outline" onClick={() => onPage(null)}>Latest scans</Button>}
          {jobs.length > 30 && <Button size="sm" variant="outline" onClick={() => onPage(jobs[29].id)}>Older scans</Button>}
        </div>
      )}
    </section>
  );
}

function ImportWorkspace() {
  const [before, setBefore] = useState<number | null>(null);
  const { data, error, refresh } = useScanSnapshot(before);
  const [busy, setBusy] = useState(false);

  const run: Run = async (action) => {
    setBusy(true);
    try {
      await action();
      return true;
    } catch (error) {
      toast.error(scanError(error));
      return false;
    } finally {
      setBusy(false);
      refresh();
    }
  };

  const active = data?.jobs.filter((job) => !terminal(job)) ?? [];
  return (
    <div className="space-y-5">
      {error && <p role="alert" className="rounded-lg border bg-card p-4 text-sm text-destructive">{error} <Button size="sm" variant="outline" onClick={refresh}>Retry</Button></p>}
      {!data && !error && <Skeleton className="h-36" role="status" aria-label="Loading scan folders" />}
      {data && (
        <>
          <section className="rounded-xl border bg-card" aria-labelledby="scan-folders">
            <div className="flex flex-wrap items-center justify-between gap-3 border-b px-5 py-4">
              <div>
                <h2 id="scan-folders" className="font-display text-2xl">Scan folders</h2>
                <p className="mt-1 text-sm text-muted-foreground">Choose one or more folders on this computer.</p>
              </div>
              <Button disabled={busy || Boolean(error)} onClick={() => void run(pickScanRoots)}><FolderPlus /> Add folders</Button>
            </div>
            {data.roots.length ? (
              <ul className="divide-y">
                {data.roots.map((root) => <RootRow key={root.id} root={root} active={active.find((job) => job.rootId === root.id)} busy={busy || Boolean(error)} run={run} />)}
              </ul>
            ) : <p className="p-5 text-sm text-muted-foreground">Add a scan folder to get started.</p>}
          </section>
          {active.map((job) => <ActiveScan key={job.id} job={job} busy={busy || Boolean(error)} run={run} />)}
          <ScanHistory jobs={data.jobs.filter(terminal)} before={before} onPage={setBefore} />
        </>
      )}
    </div>
  );
}

export function ImportPage() {
  return (
    <>
      <PageHeading label="Workspace" title="Import PDFs" description="Scan local folders, review the results, and choose when to add PDFs to your library." />
      {desktop ? <ImportWorkspace /> : <p className="rounded-xl border bg-card p-5 text-sm text-muted-foreground">Open LibraDesk for Windows to choose local folders and import PDFs.</p>}
    </>
  );
}
