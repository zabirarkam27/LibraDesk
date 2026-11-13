import { useEffect,useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/shared/ui/button";
import { Dialog,DialogContent,DialogHeader,DialogTitle,DialogDescription,DialogFooter } from "@/shared/ui/dialog";
import { desktop,errorMessage } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { flushReaderBeforeClose,readerLifecycle } from "@/features/reader/reader-client";
import { pauseBackgroundWork,resumeBackgroundWork } from "@/shared/lib/background-work";
type Preview={token:number;restoredSchema:number;backup:{path:string;createdAt:number;schemaVersion:number;books:number;notes:number;bookmarks:number;readingSessions:number;thumbnails:number;bytes:number}};
type Restored={safetyBackup:string;books:number};
export function RestoreSettings({disabled,onBusy}:{disabled:boolean;onBusy:(busy:boolean)=>void}){
  const [preview,setPreview]=useState<Preview|null>(null);
  const [result,setResult]=useState<Restored|null>(null);
  const [busy,setBusy]=useState(false);
  const [ack,setAck]=useState(false);
  const [error,setError]=useState<string|null>(null);
  const [message,setMessage]=useState<string|null>(null);
  useEffect(()=>()=>resumeBackgroundWork(),[]);
  async function inspect(){
    setBusy(true);onBusy(true);setError(null);setMessage(null);
    try{
      const value=await invoke<Preview|null>("inspect_backup");
      setPreview(value);setAck(false);
      if(!value)setMessage("Backup selection cancelled.");
    }catch(reason){setError(errorMessage(reason));}
    finally{setBusy(false);onBusy(false);}
  }
  async function cancel(){
    if(!preview||busy||result)return;
    setBusy(true);onBusy(true);
    try{await invoke("cancel_restore",{token:preview.token});setPreview(null);setError(null);}
    catch(reason){setError(errorMessage(reason));}
    finally{setBusy(false);onBusy(false);}
  }
  async function restore(){
    if(!preview||!ack||busy)return;
    setBusy(true);onBusy(true);setError(null);
    try{
      await flushReaderBeforeClose();await readerLifecycle(async()=>undefined);
      await pauseBackgroundWork();
      const value=await invoke<Restored>("restore_backup",{token:preview.token});
      setResult(value);
    }catch(reason){resumeBackgroundWork();setPreview(null);setError(errorMessage(reason));}
    finally{setBusy(false);onBusy(false);}
  }
  return <section className="space-y-3 border-t pt-5">
    <h3 className="font-semibold">Restore a backup</h3>
    <p className="text-sm text-muted-foreground">Select a complete LibraDesk backup folder. Inspection verifies files and stages the database before any replacement. A safety backup of the current library is created when you confirm.</p>
    <Button variant="outline" disabled={disabled||busy||!desktop} onClick={()=>void inspect()}>{busy&&!preview?"Inspecting backup…":"Choose backup to inspect"}</Button>
    {message&&<p role="status" className="text-sm">{message}</p>}
    {error&&!preview&&<p role="alert" className="text-sm text-destructive">{error}</p>}
    <Dialog open={preview!==null||result!==null} onOpenChange={open=>{if(!open)void cancel();}}>
      <DialogContent showCloseButton={!busy&&!result} className="max-h-[85dvh] overflow-y-auto sm:max-w-xl" onEscapeKeyDown={event=>{if(busy||result)event.preventDefault();}} onPointerDownOutside={event=>{if(busy||result)event.preventDefault();}}>
        <DialogHeader><DialogTitle>{result?"Library restored":"Confirm library restore"}</DialogTitle>
          <DialogDescription>{result?"Reload the app to use the restored settings and library.":"This replaces the current local library, settings, annotations and reading history. Original PDF files stay where they are."}</DialogDescription></DialogHeader>
        {preview&&!result&&<div className="space-y-3 text-sm">
          <p className="break-all font-mono text-xs">{preview.backup.path}</p>
          <p>{new Date(preview.backup.createdAt).toLocaleString()} · Schema {preview.backup.schemaVersion} → {preview.restoredSchema} · {formatBytes(preview.backup.bytes)}</p>
          <p>{preview.backup.books} books · {preview.backup.notes} notes · {preview.backup.bookmarks} bookmarks · {preview.backup.readingSessions} sessions · {preview.backup.thumbnails} thumbnails</p>
          <p className="text-muted-foreground">Sources may be unavailable on this device; they can be relinked later. A newer or incompatible backup is refused. A failed restore attempts to recover the safety snapshot.</p>
          <label className="flex items-start gap-2"><input type="checkbox" checked={ack} disabled={busy} onChange={event=>setAck(event.target.checked)}/>I understand that this replaces my current library data.</label>
        </div>}
        {busy&&preview&&<p role="status" className="text-sm">Waiting for background work, creating safety backup, and restoring… Keep the app open.</p>}
        {result&&<div className="space-y-3 text-sm"><p role="status">{result.books} books restored. The previous library was saved here:</p><p className="break-all select-text font-mono text-xs">{result.safetyBackup}</p></div>}
        {error&&preview&&<p role="alert" className="text-sm text-destructive">{error}</p>}
        <DialogFooter>{result?<Button onClick={()=>window.location.reload()}>Reload restored library</Button>:<>
          <Button variant="outline" disabled={busy} onClick={()=>void cancel()}>Cancel</Button>
          <Button variant="destructive" disabled={!ack||busy} onClick={()=>void restore()}>Confirm restore</Button>
        </>}</DialogFooter>
      </DialogContent>
    </Dialog>
  </section>;
}
