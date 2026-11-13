import { useCallback,useState } from "react";
import { Link } from "react-router";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { desktop,errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { DesktopNotice } from "@/features/library/query-feedback";
import { formatBytes } from "@/shared/lib/file-size";
import { listFiles,snapshot,verify,recoveryEvents } from "./client";
import { RelinkButton } from "./relink-button";
function Recovery() {
  const [after,setAfter]=useState(0);
  const [unavailable,setUnavailable]=useState(true);
  const [busy,setBusy]=useState(false);
  const [actionError,setActionError]=useState<string|null>(null);
  const query=useCallback(async()=>{const [state,files]=await Promise.all([snapshot(),listFiles(after,unavailable)]);return{state,files};},[after,unavailable]);
  const {data,error,retry}=useMetadataQuery(query,recoveryEvents);
  async function check() {
    setBusy(true);setActionError(null);
    try{await verify();retry();}
    catch(reason){setActionError(errorMessage(reason));}
    finally{setBusy(false);}
  }
  return <div className="space-y-5">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={unavailable} onChange={(event)=>{setUnavailable(event.target.checked);setAfter(0);}}/>Show only unavailable files</label>
      <Button variant="outline" disabled={busy||data?.state.status==="running"} onClick={()=>void check()}>Check files now</Button>
    </div>
    <p className="text-sm text-muted-foreground">Checks run in the background at startup and hourly. Unchanged files skip hashing; changed size or timestamp triggers a fresh streamed hash.</p>
    {data&&<p role="status" className="rounded-xl border bg-card p-4 text-sm">{data.state.status==="running"?"Checking":"Last check"}: {data.state.checked} / {data.state.total} files · {data.state.missing} missing · {data.state.inaccessible} inaccessible · {data.state.invalid} invalid · {data.state.changed} changed since tracking began</p>}
    {(error||actionError)&&<div role="alert" className="text-sm text-destructive">{actionError??"Could not load file checks."} <Button variant="outline" size="sm" onClick={retry}>Reload</Button></div>}
    {!data&&!error&&<Skeleton className="h-40" role="status" aria-label="Loading file checks"/>}
    {data&&data.files.length===0&&<p className="rounded-xl border p-5 text-sm text-muted-foreground">No {unavailable?"unavailable ":""}files on this page.</p>}
    <ul className="space-y-3">{data?.files.slice(0,50).map(file=><li key={file.id} className="rounded-xl border bg-card p-4 text-sm">
      <Link className="font-semibold text-primary underline wrap-anywhere" to={`/library/${file.bookId}`}>{file.title}</Link>
      <p className="mt-2 text-xs text-muted-foreground wrap-anywhere">{file.path}</p>
      <p className="mt-2 text-xs">{file.availability} · {formatBytes(file.bytes)} · {file.pages??"Unknown"} pages · {file.result?.replaceAll("_"," ")??"Not checked"}</p>
      <p className="mt-1 text-xs text-muted-foreground">Last checked: {file.checkedAt===null?"Not checked":new Date(file.checkedAt).toLocaleString()}</p>
      <RelinkButton file={file.id}/>
    </li>)}</ul>
    {(after>0||(data?.files.length??0)>50)&&<div className="flex gap-2"><Button variant="outline" disabled={after===0} onClick={()=>setAfter(0)}>First files</Button>{data&&data.files.length>50&&<Button variant="outline" onClick={()=>setAfter(data.files[49].id)}>More files</Button>}</div>}
  </div>;
}
export function RecoveryPage(){return <><PageHeading label="Library" title="File recovery" description="Find unavailable PDFs, verify changes, and reconnect library records to their files."/>{desktop?<Recovery/>:<DesktopNotice/>}</>;}
