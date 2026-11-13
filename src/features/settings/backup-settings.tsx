import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/shared/ui/button";
import { desktop,errorMessage } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { flushReaderBeforeClose,readerLifecycle } from "@/features/reader/reader-client";
import { RestoreSettings } from "./restore-settings";
type BackupSummary={path:string;createdAt:number;schemaVersion:number;books:number;notes:number;bookmarks:number;readingSessions:number;thumbnails:number;bytes:number};
export function BackupSettings(){
  const [thumbnails,setThumbnails]=useState(false);
  const [choose,setChoose]=useState(false);
  const [busy,setBusy]=useState(false);
  const [restoring,setRestoring]=useState(false);
  const [result,setResult]=useState<BackupSummary|null>(null);
  const [message,setMessage]=useState<string|null>(null);
  const [error,setError]=useState<string|null>(null);
  async function create(){
    setBusy(true);setError(null);setMessage(null);
    try{
      await flushReaderBeforeClose();await readerLifecycle(async()=>undefined);
      const value=await invoke<BackupSummary|null>("create_backup",{includeThumbnails:thumbnails,chooseDestination:choose});
      setResult(value);if(!value)setMessage("Backup creation cancelled.");
    }catch(reason){setError(errorMessage(reason));}
    finally{setBusy(false);}
  }
  return <div className="space-y-5 text-sm">
    <p className="text-muted-foreground">Create a consistent database snapshot with library metadata, settings, notes, bookmarks, extracted text, and reading history. Original PDF files stay in their existing locations.</p>
    <fieldset disabled={busy||restoring||!desktop} className="space-y-3">
      <legend className="mb-3 font-semibold">Backup options</legend>
      <label className="flex items-center gap-2"><input type="checkbox" checked={thumbnails} onChange={event=>setThumbnails(event.target.checked)}/>Include cached thumbnails</label>
      <label className="flex items-center gap-2"><input type="checkbox" checked={choose} onChange={event=>setChoose(event.target.checked)}/>Choose a destination folder</label>
      <p className="text-xs text-muted-foreground">Without a selected destination, backups go to the application's backups folder. Each backup is a separate folder with a versioned manifest and SHA-256 checksums.</p>
      <Button onClick={()=>void create()}>{busy?"Creating and verifying backup…":"Create backup"}</Button>
    </fieldset>
    {!desktop&&<p>Open the Windows app to create local backups.</p>}
    {message&&<p role="status">{message}</p>}
    {error&&<p role="alert" className="text-destructive">{error}</p>}
    {result&&<section role="status" aria-label="Completed backup" className="space-y-2 rounded-lg border p-4">
      <p className="font-semibold">Backup created and verified</p>
      <p className="select-text break-all font-mono text-xs">{result.path}</p>
      <p className="text-xs text-muted-foreground">{new Date(result.createdAt).toLocaleString()} · Schema {result.schemaVersion} · {formatBytes(result.bytes)}</p>
      <p>{result.books} books · {result.notes} notes · {result.bookmarks} bookmarks · {result.readingSessions} reading sessions · {result.thumbnails} thumbnails</p>
      <p className="text-xs text-muted-foreground">Keep the whole backup folder together. Restore checks every listed file before replacing library state.</p>
    </section>}
    <p className="text-xs text-muted-foreground">Backups are manual and are not encrypted.</p>
    <RestoreSettings disabled={busy} onBusy={setRestoring}/>
  </div>;
}
