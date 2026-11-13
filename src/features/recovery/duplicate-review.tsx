import { useCallback,useState } from "react";
import { Link } from "react-router";
import { useMetadataQuery } from "@/features/metadata/use-metadata-query";
import { setDuplicateReview,type DuplicateGroup } from "@/features/metadata/client";
import { Button } from "@/shared/ui/button";
import { Dialog,DialogContent,DialogHeader,DialogTitle,DialogDescription,DialogFooter } from "@/shared/ui/dialog";
import { errorMessage } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { duplicateFiles,duplicateAction,type DuplicateFile } from "./client";
const duplicateEvents=["catalog-updated"];
export function DuplicateReview({group}:{group:DuplicateGroup}) {
  const [after,setAfter]=useState(0);
  const query=useCallback(()=>duplicateFiles(group.hash,after),[group.hash,group.count,group.reviewed,after]);
  const {data,error,retry}=useMetadataQuery(query,duplicateEvents);
  const [selection,setSelection]=useState<DuplicateFile|null>(null);
  const [remove,setRemove]=useState<DuplicateFile|null>(null);
  const [ack,setAck]=useState(false);
  const [busy,setBusy]=useState(false);
  const [message,setMessage]=useState<string|null>(null);
  const keeper=selection??data?.find(file=>file.preferred)??data?.[0];
  async function prefer(){
    if(!keeper)return;setBusy(true);setMessage(null);
    try{await duplicateAction(group.hash,keeper.id,keeper.revision,keeper.id,"prefer");retry();}
    catch(reason){setMessage(errorMessage(reason));}
    finally{setBusy(false);}
  }
  async function ignore(){
    setBusy(true);setMessage(null);
    try{await setDuplicateReview(group.hash,!group.reviewed);}
    catch(reason){setMessage(errorMessage(reason));}
    finally{setBusy(false);}
  }
  async function confirm(){
    if(!remove||!keeper||!ack)return;setBusy(true);setMessage(null);
    try{await duplicateAction(group.hash,remove.id,remove.revision,keeper.id,"remove");setRemove(null);retry();}
    catch(reason){setMessage(errorMessage(reason));}
    finally{setBusy(false);}
  }
  return <li className="space-y-3 px-5 py-4">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <p className="text-sm font-semibold">{group.count} identical files · {group.reviewed?"Ignored / reviewed":"Needs review"}</p>
      <Button size="sm" variant="outline" disabled={busy} onClick={()=>void ignore()}>{group.reviewed?"Reopen review":"Ignore group"}</Button>
    </div>
    <p className="break-all font-mono text-xs text-muted-foreground">{group.hash}</p>
    <p className="text-xs text-muted-foreground">Select a copy to keep as preferred. Other records remain until you remove them individually. PDFs stay in their original locations.</p>
    {(message||error)&&<p role="alert" className="text-sm text-destructive">{message??"Could not load duplicate files. Refresh this group."}</p>}
    <ul className="space-y-3">{data?.slice(0,10).map(file=><li key={file.id} className="rounded-lg border p-3 text-sm">
      <label className="flex items-start gap-2"><input type="radio" className="mt-1" name={group.hash} checked={keeper?.id===file.id} disabled={busy}
        onChange={()=>setSelection(file)} aria-label={`Keep ${file.title} at ${file.path}`}/>
        <span className="min-w-0"><Link className="text-primary underline wrap-anywhere" to={`/library/${file.bookId}`}>{file.title}</Link>{file.preferred&&<span className="ml-2 text-xs font-semibold">Preferred</span>}
          <span className="mt-1 block text-xs text-muted-foreground wrap-anywhere">{file.author??"Unknown author"} · {file.path}</span>
        </span>
      </label>
      <p className="mt-2 text-xs text-muted-foreground">{formatBytes(file.bytes)} · {file.pages??"Unknown"} pages · {file.availability} · {file.notes} notes · {file.bookmarks} bookmarks</p>
      <p className="mt-1 text-xs text-muted-foreground">Last checked: {file.checkedAt===null?"Not checked":new Date(file.checkedAt).toLocaleString()}</p>
      <Button className="mt-2" size="sm" variant="outline" disabled={busy||keeper?.id===file.id||!keeper} onClick={()=>{setRemove(file);setAck(false);setMessage(null);}}>Remove library record</Button>
    </li>)}</ul>
    <div className="flex flex-wrap gap-2">
      <Button size="sm" disabled={busy||!keeper} onClick={()=>void prefer()}>Keep selected as preferred</Button>
      <Button variant="outline" size="sm" onClick={retry} disabled={busy}>Refresh group</Button>
      {after>0&&<Button variant="outline" size="sm" onClick={()=>setAfter(0)}>First copies</Button>}
      {data&&data.length>10&&<Button variant="outline" size="sm" onClick={()=>setAfter(data[9].id)}>More copies</Button>}
    </div>
    <Dialog open={!!remove} onOpenChange={(open)=>{if(!open&&!busy)setRemove(null);}}>
      <DialogContent onEscapeKeyDown={event=>{if(busy)event.preventDefault();}} onPointerDownOutside={event=>{if(busy)event.preventDefault();}}>
        <DialogHeader><DialogTitle>Remove duplicate library record?</DialogTitle><DialogDescription>The source PDF will stay on disk. If this is the book's last file, its book record, notes, bookmarks, progress, and collection memberships are removed from the library.</DialogDescription></DialogHeader>
        <p className="text-sm wrap-anywhere">Remove: {remove?.title} · {remove?.path}</p>
        <p className="text-sm wrap-anywhere">Keep: {keeper?.title} · {keeper?.path}</p>
        <p className="text-xs text-muted-foreground">The kept copy's SHA-256 is rechecked before removal. This can take time for a large PDF.</p>
        <label className="flex items-start gap-2 text-sm"><input type="checkbox" checked={ack} disabled={busy} onChange={event=>setAck(event.target.checked)}/>I understand that this removes the selected record's library data.</label>
        {message&&<p role="alert" className="text-sm text-destructive">{message}</p>}
        <DialogFooter><Button variant="outline" disabled={busy} onClick={()=>setRemove(null)}>Cancel</Button><Button variant="destructive" disabled={busy||!ack} onClick={()=>void confirm()}>{busy?"Verifying kept PDF…":"Confirm record removal"}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  </li>;
}
