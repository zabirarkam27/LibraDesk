import { useCallback,useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { formatBytes } from "@/shared/lib/file-size";
import { desktop,errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { Dialog,DialogContent,DialogHeader,DialogTitle,DialogDescription,DialogFooter } from "@/shared/ui/dialog";
import { useWorkspaceSettings } from "./workspace-store";
import { usePreferences } from "./preferences-store";
type Usage={database:number;thumbnails:number;cache:number;temporary:number;backups:number;logs:number;files:number;skipped:number};
type Kind="cache"|"thumbnails"|"temporary";
const labels:Record<Kind,string>={cache:"Cache files",thumbnails:"Thumbnail cache",temporary:"Old temporary files"};
function NativeStorage(){
  const query=useCallback(()=>invoke<Usage>("storage_usage"),[]);
  const {data,error,retry}=useMetadataQuery(query);
  const [kind,setKind]=useState<Kind|null>(null);
  const [busy,setBusy]=useState(false);
  const [message,setMessage]=useState<string|null>(null);
  const [failure,setFailure]=useState<string|null>(null);
  async function clear(){
    if(!kind||busy)return;setBusy(true);setFailure(null);
    try{
      const result=await invoke<{removed:number;bytes:number;retained:number;usage:Usage}>("clear_cache",{kind});
      setMessage(`Removed ${result.removed} files (${formatBytes(result.bytes)}). Retained ${result.retained} protected, active, or nested entries.`);
      setKind(null);retry();
    }catch(reason){setFailure(errorMessage(reason));}
    finally{setBusy(false);}
  }
  return <div className="space-y-4">
    {error&&<p role="alert">Storage usage could not be loaded. <Button variant="outline" size="sm" onClick={retry}>Reload storage</Button></p>}
    {!data&&!error&&<p role="status" className="text-sm">Measuring local storage…</p>}
    {data&&<dl className="grid gap-3 text-sm sm:grid-cols-2">{([
      ["Database, WAL and shared memory",data.database],["Thumbnail cache",data.thumbnails],["Cache",data.cache],
      ["Temporary storage",data.temporary],["Managed backups",data.backups],["Logs",data.logs],
    ] as const).map(([label,bytes])=><div key={label} className="flex justify-between gap-3 rounded-lg border p-3"><dt>{label}</dt><dd className="shrink-0 font-medium">{formatBytes(bytes)}</dd></div>)}</dl>}
    <p className="text-xs text-muted-foreground">Usage covers application folders. Backups and PDFs stored elsewhere are outside these totals.</p>
    <div className="flex flex-wrap gap-2">{(["cache","thumbnails","temporary"] as const).map(value=><Button key={value} variant="outline" size="sm" disabled={busy||!data} onClick={()=>{setKind(value);setFailure(null);}}>Clear {labels[value].toLowerCase()}</Button>)}</div>
    {message&&<p role="status" className="text-sm">{message}</p>}
    <Dialog open={kind!==null} onOpenChange={open=>{if(!open&&!busy)setKind(null);}}>
      <DialogContent onEscapeKeyDown={event=>{if(busy)event.preventDefault();}} onPointerDownOutside={event=>{if(busy)event.preventDefault();}}>
        <DialogHeader><DialogTitle>Clear {kind?labels[kind].toLowerCase():"cache"}?</DialogTitle>
          <DialogDescription>Only disposable files in the selected application cache folder are eligible. Database records, backups and source PDFs are preserved. Subfolders and linked entries are retained.</DialogDescription></DialogHeader>
        <p className="text-sm text-muted-foreground">{kind==="temporary"?"Only direct temporary files older than 24 hours are removed.":kind==="thumbnails"?"Only generated thumbnail PNGs are removed. Covers regenerate when needed.":"Direct cache files are removed. Active background work may finish before cleanup starts."}</p>
        {failure&&<p role="alert" className="text-sm text-destructive">{failure}</p>}
        <DialogFooter><Button variant="outline" disabled={busy} onClick={()=>setKind(null)}>Cancel</Button><Button disabled={busy} onClick={()=>void clear()}>{busy?"Cleaning…":"Confirm cache cleanup"}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  </div>;
}
export function StorageSettings(){
  const path=useWorkspaceSettings(state=>state.snapshot.dataPath);
  const schema=usePreferences(state=>state.preferences.schemaVersion);
  return <div className="space-y-5">
    {desktop&&<p className="text-sm">SQLite connected · Schema {schema}</p>}
    <div><p className="text-sm font-medium">Application data location</p><p className="mt-2 select-text break-all font-mono text-xs text-muted-foreground">{path}</p></div>
    {desktop?<NativeStorage/>:<p className="text-sm text-muted-foreground">Open the Windows app to inspect disk usage and clean caches.</p>}
  </div>;
}
